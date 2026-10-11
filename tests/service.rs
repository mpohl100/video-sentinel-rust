#![warn(clippy::pedantic)]
#![allow(
    clippy::missing_errors_doc,
    clippy::missing_panics_doc,
    clippy::too_many_lines,
    clippy::unused_async_trait_impl
)]

use std::path::PathBuf;
use std::sync::{Once, OnceLock};

use async_graphql::EmptySubscription;
use async_graphql::Schema;
use async_graphql_axum::{GraphQLRequest, GraphQLResponse};
use axum::Router;
use axum::extract::State;
use axum::routing::post;
use graphql_client::QueryBody;
use num_traits::ToPrimitive;
use opencv::core::Vector as CvVector;
use opencv::imgcodecs;
use opencv::imgproc;
use opencv::prelude::*;
use reqwest::multipart::{Form, Part};
use serde_json::{Value, json};
use tokio::net::TcpListener;
use tokio::sync::oneshot;
use tokio::task::JoinHandle;
use video_rs::{Decoder, Frame};
use video_sentinel::graphql::{MutationRoot, QueryRoot};
use video_sentinel::slices::WrappedRgbImage;

type TestSchema = Schema<QueryRoot, MutationRoot, EmptySubscription>;

fn u32_from_usize(value: usize) -> u32 {
    value
        .to_u32()
        .expect("frame dimension should fit within u32")
}

const FETCH_SESSION_STATE: &str = r"
query FetchSessionState($sessionId: String!) {
  getOrdinarySession(sessionId: $sessionId) {
    basicParams {
      doGrayscale
      gradientThreshold
    }
    results
  }
  getEyeSession(sessionId: $sessionId) {
    basicParams {
      doGrayscale
      gradientThreshold
    }
    eyeParams {
      tileParams {
        tileX
        tileY
      }
      bucketDelta
      traceParams {
        numSkeleton
        closeSliceThreshold
      }
      targetSimilarity
    }
    results
  }
  getObjectSession(sessionId: $sessionId) {
    basicParams {
      doGrayscale
      gradientThreshold
    }
    objectDetectionParams {
      tileParams {
        tileX
        tileY
      }
      bucketDelta
      traceParams {
        numSkeleton
        closeSliceThreshold
      }
      targetSimilarity
    }
    objectsToBeDetected {
      objectId
      surroundingBoundingBox {
        topLeft {
          x
          y
          z
        }
        bottomRight {
          x
          y
          z
        }
      }
    }
    results
  }
  getBasicParams(sessionId: $sessionId) {
    doGrayscale
    gradientThreshold
  }
  getTileParams(sessionId: $sessionId) {
    tileX
    tileY
  }
  getTraceParams(sessionId: $sessionId) {
    numSkeleton
    closeSliceThreshold
  }
  getBucketDelta(sessionId: $sessionId)
  getTargetSimilarity(sessionId: $sessionId)
}
";

const GET_RECTANGLES: &str = r"
mutation GetRectangles($sessionId: String!, $image: Upload!) {
  getRectangles(sessionId: $sessionId, image: $image) {
    boundingBox {
      topLeft {
        x
        y
        z
      }
      bottomRight {
        x
        y
        z
      }
    }
    boundingCircle {
      center {
        x
        y
        z
      }
      radius
    }
    color
    area
    centerOfMass {
      x
      y
      z
    }
    sliceMatrix {
      lineNumber
      slices {
        start {
          x
          y
        }
        end {
          x
          y
        }
      }
    }
    averageColor {
      red
      green
      blue
    }
    results
  }
}
";

const CREATE_ORDINARY_SESSION: &str = r"
mutation CreateOrdinarySession(
  $sessionId: String!
  $basicParams: BasicParamsInputGql!
  $results: ResultsGql!
) {
  createOrdinarySession(sessionId: $sessionId, basicParams: $basicParams, results: $results)
}
";

const CREATE_EYE_SESSION: &str = r"
mutation CreateEyeSession(
  $sessionId: String!
  $basicParams: BasicParamsInputGql!
  $eyeParams: EyeParamsInputGql!
  $results: ResultsGql!
) {
  createEyeSession(
    sessionId: $sessionId
    basicParams: $basicParams
    eyeParams: $eyeParams
    results: $results
  )
}
";

const CREATE_OBJECT_SESSION: &str = r"
mutation CreateObjectSession(
  $sessionId: String!
  $basicParams: BasicParamsInputGql!
  $objectDetectionParams: ObjectDetectionParamsInputGql!
  $results: ResultsGql!
) {
  createObjectSession(
    sessionId: $sessionId
    basicParams: $basicParams
    objectDetectionParams: $objectDetectionParams
    results: $results
  )
}
";

const UPDATE_BASIC_PARAMS: &str = r"
mutation UpdateBasicParams($sessionId: String!, $basicParams: BasicParamsInputGql!) {
  updateBasicParams(sessionId: $sessionId, basicParams: $basicParams)
}
";

const UPDATE_TILE_PARAMS: &str = r"
mutation UpdateTileParams($sessionId: String!, $tileParams: TileParamsInputGql!) {
  updateTileParams(sessionId: $sessionId, tileParams: $tileParams)
}
";

const UPDATE_TRACE_PARAMS: &str = r"
mutation UpdateTraceParams($sessionId: String!, $traceParams: TraceParamsInputGql!) {
  updateTraceParams(sessionId: $sessionId, traceParams: $traceParams)
}
";

const UPDATE_BUCKET_DELTA: &str = r"
mutation UpdateBucketDelta($sessionId: String!, $bucketDelta: Float!) {
  updateBucketDelta(sessionId: $sessionId, bucketDelta: $bucketDelta)
}
";

const UPDATE_TARGET_SIMILARITY: &str = r"
mutation UpdateTargetSimilarity($sessionId: String!, $targetSimilarity: Float!) {
  updateTargetSimilarity(sessionId: $sessionId, targetSimilarity: $targetSimilarity)
}
";

const UPDATE_EYE_PARAMS: &str = r"
mutation UpdateEyeParams($sessionId: String!, $eyeParams: EyeParamsInputGql!) {
  updateEyeParams(sessionId: $sessionId, eyeParams: $eyeParams)
}
";

const UPDATE_OBJECT_DETECTION_PARAMS: &str = r"
mutation UpdateObjectDetectionParams(
  $sessionId: String!
  $objectDetectionParams: ObjectDetectionParamsInputGql!
) {
  updateObjectDetectionParams(
    sessionId: $sessionId
    objectDetectionParams: $objectDetectionParams
  )
}
";

const ADD_OBJECT_AS_IMAGE: &str = r"
mutation AddObjectToBeDetectedAsImage(
  $sessionId: String!
  $objectId: String!
  $image: Upload!
  $surroundingRectangle: RectangleInputGql!
) {
  addObjectToBeDetectedAsImage(
    sessionId: $sessionId
    objectId: $objectId
    image: $image
    surroundingRectangle: $surroundingRectangle
  )
}
";

const ADD_OBJECT_AS_ASCII_ART: &str = r"
mutation AddObjectToBeDetectedAsAsciiArt(
  $sessionId: String!
  $objectId: String!
  $asciiArt: String!
) {
  addObjectToBeDetectedAsAsciiArt(
    sessionId: $sessionId
    objectId: $objectId
    asciiArt: $asciiArt
  )
}
";

const DELETE_SESSION: &str = r"
mutation DeleteSession($sessionId: String!) {
  deleteSession(sessionId: $sessionId)
}
";

const DELETE_REFERENCE_OBJECT: &str = r"
mutation DeleteReferenceObject($sessionId: String!, $objectId: String!) {
  deleteReferenceObject(sessionId: $sessionId, objectId: $objectId)
}
";

static VIDEO_RS_INIT: Once = Once::new();
static EXAMPLE_FRAMES: OnceLock<Vec<ExampleFrame>> = OnceLock::new();

#[derive(Clone)]
struct ExampleFrame {
    jpeg: Vec<u8>,
    width: u32,
    height: u32,
}

struct UploadPart<'a> {
    variable_path: &'a str,
    file_name: &'a str,
    bytes: &'a [u8],
    mime: &'a str,
}

struct TestServer {
    base_url: String,
    client: reqwest::Client,
    shutdown_tx: Option<oneshot::Sender<()>>,
    server_task: JoinHandle<()>,
}

impl TestServer {
    async fn spawn() -> Self {
        let listener = TcpListener::bind("127.0.0.1:0")
            .await
            .expect("test listener should bind");
        let addr = listener
            .local_addr()
            .expect("test listener should expose local address");
        let schema = video_sentinel::graphql::create_schema();
        let app = Router::new()
            .route("/graphql", post(graphql_handler))
            .with_state(schema);
        let (shutdown_tx, shutdown_rx) = oneshot::channel();

        let server_task = tokio::spawn(async move {
            axum::serve(listener, app)
                .with_graceful_shutdown(async move {
                    let _ = shutdown_rx.await;
                })
                .await
                .expect("test GraphQL server should run");
        });

        Self {
            base_url: format!("http://{addr}"),
            client: reqwest::Client::builder()
                .build()
                .expect("HTTP client should build"),
            shutdown_tx: Some(shutdown_tx),
            server_task,
        }
    }

    async fn graphql_json(
        &self,
        query: &'static str,
        operation_name: &'static str,
        variables: Value,
    ) -> Value {
        let body = QueryBody {
            variables,
            query,
            operation_name,
        };
        let response = self
            .client
            .post(format!("{}/graphql", self.base_url))
            .json(&body)
            .send()
            .await
            .expect("GraphQL JSON request should complete");
        parse_json_response(response).await
    }

    async fn graphql_upload(
        &self,
        query: &'static str,
        operation_name: &'static str,
        variables: Value,
        uploads: &[UploadPart<'_>],
    ) -> Value {
        let body = QueryBody {
            variables,
            query,
            operation_name,
        };

        let mut map = serde_json::Map::new();
        let mut form = Form::new().text(
            "operations",
            serde_json::to_string(&body).expect("operations should serialize"),
        );

        for (index, upload) in uploads.iter().enumerate() {
            map.insert(
                index.to_string(),
                Value::Array(vec![Value::String(upload.variable_path.to_string())]),
            );
            form = form.part(
                index.to_string(),
                Part::bytes(upload.bytes.to_vec())
                    .file_name(upload.file_name.to_string())
                    .mime_str(upload.mime)
                    .expect("upload MIME should be valid"),
            );
        }

        let response = self
            .client
            .post(format!("{}/graphql", self.base_url))
            .multipart(form.text(
                "map",
                serde_json::to_string(&Value::Object(map)).expect("upload map should serialize"),
            ))
            .send()
            .await
            .expect("GraphQL upload request should complete");
        parse_json_response(response).await
    }

    async fn shutdown(mut self) {
        if let Some(shutdown_tx) = self.shutdown_tx.take() {
            let _ = shutdown_tx.send(());
        }
        self.server_task
            .await
            .expect("test GraphQL server should stop cleanly");
    }
}

async fn graphql_handler(
    State(schema): State<TestSchema>,
    request: GraphQLRequest,
) -> GraphQLResponse {
    schema.execute(request.into_inner()).await.into()
}

fn example_frames() -> &'static [ExampleFrame] {
    EXAMPLE_FRAMES.get_or_init(load_example_frames).as_slice()
}

fn load_example_frames() -> Vec<ExampleFrame> {
    VIDEO_RS_INIT.call_once(|| {
        video_rs::init().expect("video-rs should initialize exactly once");
    });

    let video_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("video")
        .join("BillardTakeoff.mp4");

    let mut decoder = Decoder::new(video_path.as_path()).expect("example video should open");
    let frames: Vec<_> = decoder
        .decode_iter()
        .take(3)
        .map(|frame_result| {
            let (_timestamp, frame) = frame_result.expect("example video frame should decode");
            let image = wrapped_rgb_image_from_frame(&frame);
            ExampleFrame {
                width: image.width(),
                height: image.height(),
                jpeg: jpeg_bytes_from_image(&image),
            }
        })
        .collect();

    assert_eq!(frames.len(), 3, "expected three example frames");
    frames
}

fn wrapped_rgb_image_from_frame(frame: &Frame) -> WrappedRgbImage {
    let (height, width, channels) = frame.dim();
    assert_eq!(channels, 3, "example video should decode as RGB");

    let data = if let Some(slice) = frame.as_slice_memory_order() {
        slice.to_vec()
    } else {
        frame.iter().copied().collect()
    };

    WrappedRgbImage::from_rgb_data(u32_from_usize(width), u32_from_usize(height), &data)
        .expect("decoded frame should convert into WrappedRgbImage")
}

fn jpeg_bytes_from_image(image: &WrappedRgbImage) -> Vec<u8> {
    let rgb = image
        .get_mat()
        .try_clone()
        .expect("wrapped RGB image should clone");
    let mut bgr = Mat::default();
    imgproc::cvt_color(&rgb, &mut bgr, imgproc::COLOR_RGB2BGR, 0)
        .expect("RGB frame should convert to BGR for JPEG encoding");

    let mut encoded = CvVector::<u8>::new();
    imgcodecs::imencode(".jpg", &bgr, &mut encoded, &CvVector::<i32>::new())
        .expect("example frame should encode as JPEG");
    encoded.to_vec()
}

async fn parse_json_response(response: reqwest::Response) -> Value {
    let status = response.status();
    let body = response
        .text()
        .await
        .expect("response body should be readable");
    assert!(
        status.is_success(),
        "expected HTTP success, got {status}: {body}"
    );
    serde_json::from_str(&body).expect("GraphQL response should be valid JSON")
}

fn basic_params_json(do_grayscale: bool, gradient_threshold: u8) -> Value {
    json!({
        "doGrayscale": do_grayscale,
        "gradientThreshold": gradient_threshold,
    })
}

fn tile_params_json(tile_x: f64, tile_y: f64) -> Value {
    json!({
        "tileX": tile_x,
        "tileY": tile_y,
    })
}

fn trace_params_json(num_skeleton: usize, close_slice_threshold: f64) -> Value {
    json!({
        "numSkeleton": num_skeleton,
        "closeSliceThreshold": close_slice_threshold,
    })
}

fn eye_params_json(
    tile_x: f64,
    tile_y: f64,
    bucket_delta: f64,
    num_skeleton: usize,
    close_slice_threshold: f64,
    target_similarity: f64,
) -> Value {
    json!({
        "tileParams": tile_params_json(tile_x, tile_y),
        "bucketDelta": bucket_delta,
        "traceParams": trace_params_json(num_skeleton, close_slice_threshold),
        "targetSimilarity": target_similarity,
    })
}

fn object_detection_params_json(
    tile_x: f64,
    tile_y: f64,
    bucket_delta: f64,
    num_skeleton: usize,
    close_slice_threshold: f64,
    target_similarity: f64,
) -> Value {
    json!({
        "tileParams": tile_params_json(tile_x, tile_y),
        "bucketDelta": bucket_delta,
        "traceParams": trace_params_json(num_skeleton, close_slice_threshold),
        "targetSimilarity": target_similarity,
    })
}

fn full_frame_rectangle_json(frame: &ExampleFrame) -> Value {
    json!({
        "topLeft": { "x": 0.0, "y": 0.0, "z": 0.0 },
        "bottomRight": {
            "x": f64::from(frame.width),
            "y": f64::from(frame.height),
            "z": 0.0,
        },
    })
}

fn data<'a>(response: &'a Value, field: &str) -> &'a Value {
    &response["data"][field]
}

fn error_messages(response: &Value) -> Vec<String> {
    response["errors"]
        .as_array()
        .map(|errors| {
            errors
                .iter()
                .filter_map(|error| error["message"].as_str().map(ToOwned::to_owned))
                .collect()
        })
        .unwrap_or_default()
}

fn assert_no_errors(response: &Value) {
    let errors = error_messages(response);
    assert!(
        errors.is_empty(),
        "expected GraphQL success, got errors {errors:?} in response {response}"
    );
}

fn assert_bool_success(response: &Value, field: &str) {
    assert_no_errors(response);
    assert_eq!(
        data(response, field),
        &Value::Bool(true),
        "expected `{field}` to return true in response {response}"
    );
}

fn assert_error_message(response: &Value, expected: &str) {
    let errors = error_messages(response);
    assert_eq!(
        errors,
        vec![expected.to_string()],
        "unexpected GraphQL response {response}"
    );
}

fn assert_error_contains(response: &Value, expected_substring: &str) {
    let errors = error_messages(response);
    assert_eq!(errors.len(), 1, "unexpected GraphQL response {response}");
    assert!(
        errors[0].contains(expected_substring),
        "expected `{}` to contain `{expected_substring}`",
        errors[0]
    );
}

fn assert_rectangles_payload(response: &Value) {
    assert_no_errors(response);
    assert!(
        data(response, "getRectangles").as_array().is_some(),
        "expected getRectangles payload in response {response}"
    );
}

fn first_bounding_box(response: &Value) -> Value {
    assert_no_errors(response);
    let rectangles = data(response, "getRectangles")
        .as_array()
        .expect("getRectangles should return an array");
    assert!(
        !rectangles.is_empty(),
        "expected at least one rectangle in response {response}"
    );
    rectangles[0]["boundingBox"].clone()
}

async fn get_rectangles(server: &TestServer, session_id: &str, image: &ExampleFrame) -> Value {
    let uploads = vec![UploadPart {
        variable_path: "variables.image",
        file_name: "frame.jpg",
        bytes: &image.jpeg,
        mime: "image/jpeg",
    }];

    server
        .graphql_upload(
            GET_RECTANGLES,
            "GetRectangles",
            json!({
                "sessionId": session_id,
                "image": Value::Null,
            }),
            &uploads,
        )
        .await
}

async fn add_object_as_image(
    server: &TestServer,
    session_id: &str,
    object_id: &str,
    image: &[u8],
    surrounding_rectangle: Value,
) -> Value {
    server
        .graphql_upload(
            ADD_OBJECT_AS_IMAGE,
            "AddObjectToBeDetectedAsImage",
            json!({
                "sessionId": session_id,
                "objectId": object_id,
                "image": Value::Null,
                "surroundingRectangle": surrounding_rectangle,
            }),
            &[UploadPart {
                variable_path: "variables.image",
                file_name: "reference.jpg",
                bytes: image,
                mime: "image/jpeg",
            }],
        )
        .await
}

#[tokio::test(flavor = "multi_thread")]
async fn ordinary_session_graphql_roundtrip_covers_success_and_unsupported_operations() {
    let frames = example_frames();
    let server = TestServer::spawn().await;
    let session_id = "ordinary-session";

    assert_bool_success(
        &server
            .graphql_json(
                CREATE_ORDINARY_SESSION,
                "CreateOrdinarySession",
                json!({
                    "sessionId": session_id,
                    "basicParams": basic_params_json(false, 15),
                    "results": "ABSOLUTE",
                }),
            )
            .await,
        "createOrdinarySession",
    );
    assert_error_message(
        &server
            .graphql_json(
                CREATE_ORDINARY_SESSION,
                "CreateOrdinarySession",
                json!({
                    "sessionId": session_id,
                    "basicParams": basic_params_json(false, 15),
                    "results": "ABSOLUTE",
                }),
            )
            .await,
        "Session already exists",
    );

    assert_bool_success(
        &server
            .graphql_json(
                UPDATE_BASIC_PARAMS,
                "UpdateBasicParams",
                json!({
                    "sessionId": session_id,
                    "basicParams": basic_params_json(true, 12),
                }),
            )
            .await,
        "updateBasicParams",
    );

    for frame in frames {
        assert_rectangles_payload(&get_rectangles(&server, session_id, frame).await);
    }

    let session_state = server
        .graphql_json(
            FETCH_SESSION_STATE,
            "FetchSessionState",
            json!({ "sessionId": session_id }),
        )
        .await;
    assert_no_errors(&session_state);
    assert_eq!(
        data(&session_state, "getOrdinarySession")["basicParams"]["doGrayscale"],
        Value::Bool(true)
    );
    assert_eq!(
        data(&session_state, "getOrdinarySession")["basicParams"]["gradientThreshold"],
        Value::from(12)
    );
    assert!(data(&session_state, "getOrdinarySession")["results"].is_string());
    assert!(data(&session_state, "getEyeSession").is_null());
    assert!(data(&session_state, "getObjectSession").is_null());
    assert_eq!(
        data(&session_state, "getBasicParams")["doGrayscale"],
        Value::Bool(true)
    );
    assert_eq!(
        data(&session_state, "getBasicParams")["gradientThreshold"],
        Value::from(12)
    );
    assert!(data(&session_state, "getTileParams").is_null());
    assert!(data(&session_state, "getTraceParams").is_null());
    assert!(data(&session_state, "getBucketDelta").is_null());
    assert!(data(&session_state, "getTargetSimilarity").is_null());

    assert_error_message(
        &server
            .graphql_json(
                UPDATE_TILE_PARAMS,
                "UpdateTileParams",
                json!({
                    "sessionId": session_id,
                    "tileParams": tile_params_json(0.2, 0.8),
                }),
            )
            .await,
        "Session type does not support tile params",
    );
    assert_error_message(
        &server
            .graphql_json(
                UPDATE_TRACE_PARAMS,
                "UpdateTraceParams",
                json!({
                    "sessionId": session_id,
                    "traceParams": trace_params_json(18, 0.25),
                }),
            )
            .await,
        "Session type does not support trace params",
    );
    assert_error_message(
        &server
            .graphql_json(
                UPDATE_BUCKET_DELTA,
                "UpdateBucketDelta",
                json!({
                    "sessionId": session_id,
                    "bucketDelta": 0.4,
                }),
            )
            .await,
        "Session type does not support bucket delta",
    );
    assert_error_message(
        &server
            .graphql_json(
                UPDATE_TARGET_SIMILARITY,
                "UpdateTargetSimilarity",
                json!({
                    "sessionId": session_id,
                    "targetSimilarity": 0.85,
                }),
            )
            .await,
        "Session type does not support target similarity",
    );
    assert_error_message(
        &server
            .graphql_json(
                UPDATE_EYE_PARAMS,
                "UpdateEyeParams",
                json!({
                    "sessionId": session_id,
                    "eyeParams": eye_params_json(0.2, 0.3, 0.4, 18, 0.15, 0.8),
                }),
            )
            .await,
        "Session type does not support eye params",
    );
    assert_error_message(
        &server
            .graphql_json(
                UPDATE_OBJECT_DETECTION_PARAMS,
                "UpdateObjectDetectionParams",
                json!({
                    "sessionId": session_id,
                    "objectDetectionParams": object_detection_params_json(0.2, 0.3, 0.4, 18, 0.15, 0.8),
                }),
            )
            .await,
        "Session type does not support object detection params",
    );
    assert_error_message(
        &add_object_as_image(
            &server,
            session_id,
            "ordinary-ref",
            &frames[0].jpeg,
            full_frame_rectangle_json(&frames[0]),
        )
        .await,
        "Session type does not support adding object to be detected",
    );
    assert_error_message(
        &server
            .graphql_json(
                ADD_OBJECT_AS_ASCII_ART,
                "AddObjectToBeDetectedAsAsciiArt",
                json!({
                    "sessionId": session_id,
                    "objectId": "ordinary-ref",
                    "asciiArt": "##\n##",
                }),
            )
            .await,
        "Session type does not support adding object to be detected",
    );
    assert_error_message(
        &server
            .graphql_json(
                DELETE_REFERENCE_OBJECT,
                "DeleteReferenceObject",
                json!({
                    "sessionId": session_id,
                    "objectId": "ordinary-ref",
                }),
            )
            .await,
        "Session type does not support deleting reference object",
    );
    assert_error_contains(
        &add_object_as_image(
            &server,
            session_id,
            "bad-jpeg",
            b"not-a-jpeg",
            full_frame_rectangle_json(&frames[0]),
        )
        .await,
        "Failed to decode JPEG upload",
    );
    assert_error_contains(
        &server
            .graphql_upload(
                GET_RECTANGLES,
                "GetRectangles",
                json!({
                    "sessionId": session_id,
                    "image": Value::Null,
                }),
                &[UploadPart {
                    variable_path: "variables.image",
                    file_name: "bad.jpg",
                    bytes: b"not-a-jpeg",
                    mime: "image/jpeg",
                }],
            )
            .await,
        "Failed to decode JPEG upload",
    );

    assert_bool_success(
        &server
            .graphql_json(
                DELETE_SESSION,
                "DeleteSession",
                json!({ "sessionId": session_id }),
            )
            .await,
        "deleteSession",
    );
    assert_error_message(
        &server
            .graphql_json(
                DELETE_SESSION,
                "DeleteSession",
                json!({ "sessionId": session_id }),
            )
            .await,
        "Session not found",
    );

    server.shutdown().await;
}

#[tokio::test(flavor = "multi_thread")]
async fn eye_session_graphql_roundtrip_requires_previous_frame_and_exposes_eye_queries() {
    let frames = example_frames();
    let server = TestServer::spawn().await;
    let session_id = "eye-session";

    assert_bool_success(
        &server
            .graphql_json(
                CREATE_EYE_SESSION,
                "CreateEyeSession",
                json!({
                    "sessionId": session_id,
                    "basicParams": basic_params_json(false, 15),
                    "eyeParams": eye_params_json(0.5, 0.5, 0.25, 12, 0.2, 0.9),
                    "results": "RELATIVE",
                }),
            )
            .await,
        "createEyeSession",
    );
    assert_error_message(
        &server
            .graphql_json(
                CREATE_EYE_SESSION,
                "CreateEyeSession",
                json!({
                    "sessionId": session_id,
                    "basicParams": basic_params_json(false, 15),
                    "eyeParams": eye_params_json(0.5, 0.5, 0.25, 12, 0.2, 0.9),
                    "results": "RELATIVE",
                }),
            )
            .await,
        "Session already exists",
    );

    assert_bool_success(
        &server
            .graphql_json(
                UPDATE_BASIC_PARAMS,
                "UpdateBasicParams",
                json!({
                    "sessionId": session_id,
                    "basicParams": basic_params_json(true, 9),
                }),
            )
            .await,
        "updateBasicParams",
    );
    assert_bool_success(
        &server
            .graphql_json(
                UPDATE_TILE_PARAMS,
                "UpdateTileParams",
                json!({
                    "sessionId": session_id,
                    "tileParams": tile_params_json(0.2, 0.4),
                }),
            )
            .await,
        "updateTileParams",
    );
    assert_bool_success(
        &server
            .graphql_json(
                UPDATE_TRACE_PARAMS,
                "UpdateTraceParams",
                json!({
                    "sessionId": session_id,
                    "traceParams": trace_params_json(24, 0.3),
                }),
            )
            .await,
        "updateTraceParams",
    );
    assert_bool_success(
        &server
            .graphql_json(
                UPDATE_BUCKET_DELTA,
                "UpdateBucketDelta",
                json!({
                    "sessionId": session_id,
                    "bucketDelta": 0.55,
                }),
            )
            .await,
        "updateBucketDelta",
    );
    assert_bool_success(
        &server
            .graphql_json(
                UPDATE_TARGET_SIMILARITY,
                "UpdateTargetSimilarity",
                json!({
                    "sessionId": session_id,
                    "targetSimilarity": 0.87,
                }),
            )
            .await,
        "updateTargetSimilarity",
    );
    assert_bool_success(
        &server
            .graphql_json(
                UPDATE_EYE_PARAMS,
                "UpdateEyeParams",
                json!({
                    "sessionId": session_id,
                    "eyeParams": eye_params_json(0.15, 0.35, 0.45, 18, 0.25, 0.92),
                }),
            )
            .await,
        "updateEyeParams",
    );

    let first_eye_response = get_rectangles(&server, session_id, &frames[0]).await;
    assert_rectangles_payload(&first_eye_response);
    let first_eye_colors = data(&first_eye_response, "getRectangles")
        .as_array()
        .expect("expected eye rectangles array");
    assert!(
        first_eye_colors
            .iter()
            .all(|rectangle| rectangle["color"] == Value::String("BLUE".to_string()))
    );
    let second_eye_response = get_rectangles(&server, session_id, &frames[1]).await;
    assert_rectangles_payload(&second_eye_response);
    let third_eye_response = get_rectangles(&server, session_id, &frames[2]).await;
    assert_rectangles_payload(&third_eye_response);

    let session_state = server
        .graphql_json(
            FETCH_SESSION_STATE,
            "FetchSessionState",
            json!({ "sessionId": session_id }),
        )
        .await;
    assert_no_errors(&session_state);
    assert!(data(&session_state, "getOrdinarySession").is_null());
    assert!(data(&session_state, "getObjectSession").is_null());
    assert_eq!(
        data(&session_state, "getEyeSession")["basicParams"]["doGrayscale"],
        Value::Bool(true)
    );
    assert_eq!(
        data(&session_state, "getEyeSession")["basicParams"]["gradientThreshold"],
        Value::from(9)
    );
    assert_eq!(
        data(&session_state, "getEyeSession")["eyeParams"]["tileParams"]["tileX"],
        Value::from(0.15)
    );
    assert_eq!(
        data(&session_state, "getEyeSession")["eyeParams"]["tileParams"]["tileY"],
        Value::from(0.35)
    );
    assert_eq!(
        data(&session_state, "getEyeSession")["eyeParams"]["bucketDelta"],
        Value::from(0.45)
    );
    assert_eq!(
        data(&session_state, "getEyeSession")["eyeParams"]["traceParams"]["numSkeleton"],
        Value::from(18)
    );
    assert_eq!(
        data(&session_state, "getEyeSession")["eyeParams"]["traceParams"]["closeSliceThreshold"],
        Value::from(0.25)
    );
    assert_eq!(
        data(&session_state, "getEyeSession")["eyeParams"]["targetSimilarity"],
        Value::from(0.92)
    );
    assert!(data(&session_state, "getEyeSession")["results"].is_string());
    assert_eq!(
        data(&session_state, "getBasicParams")["doGrayscale"],
        Value::Bool(true)
    );
    assert_eq!(
        data(&session_state, "getTileParams")["tileX"],
        Value::from(0.15)
    );
    assert_eq!(
        data(&session_state, "getTileParams")["tileY"],
        Value::from(0.35)
    );
    assert_eq!(
        data(&session_state, "getTraceParams")["numSkeleton"],
        Value::from(18)
    );
    assert_eq!(
        data(&session_state, "getTraceParams")["closeSliceThreshold"],
        Value::from(0.25)
    );
    assert_eq!(data(&session_state, "getBucketDelta"), &Value::from(0.45));
    assert_eq!(
        data(&session_state, "getTargetSimilarity"),
        &Value::from(0.92)
    );

    server.shutdown().await;
}

#[tokio::test(flavor = "multi_thread")]
async fn object_session_graphql_roundtrip_covers_reference_objects_and_detection_queries() {
    let frames = example_frames();
    let server = TestServer::spawn().await;
    let session_id = "object-session";
    let reference_source_session_id = "object-reference-source";

    assert_bool_success(
        &server
            .graphql_json(
                CREATE_OBJECT_SESSION,
                "CreateObjectSession",
                json!({
                    "sessionId": session_id,
                    "basicParams": basic_params_json(false, 15),
                    "objectDetectionParams": object_detection_params_json(0.5, 0.5, 0.25, 12, 0.2, 0.9),
                    "results": "ABSOLUTE",
                }),
            )
            .await,
        "createObjectSession",
    );
    assert_error_message(
        &server
            .graphql_json(
                CREATE_OBJECT_SESSION,
                "CreateObjectSession",
                json!({
                    "sessionId": session_id,
                    "basicParams": basic_params_json(false, 15),
                    "objectDetectionParams": object_detection_params_json(0.5, 0.5, 0.25, 12, 0.2, 0.9),
                    "results": "ABSOLUTE",
                }),
            )
            .await,
        "Session already exists",
    );

    assert_bool_success(
        &server
            .graphql_json(
                UPDATE_BASIC_PARAMS,
                "UpdateBasicParams",
                json!({
                    "sessionId": session_id,
                    "basicParams": basic_params_json(true, 11),
                }),
            )
            .await,
        "updateBasicParams",
    );
    assert_bool_success(
        &server
            .graphql_json(
                UPDATE_TILE_PARAMS,
                "UpdateTileParams",
                json!({
                    "sessionId": session_id,
                    "tileParams": tile_params_json(0.3, 0.25),
                }),
            )
            .await,
        "updateTileParams",
    );
    assert_bool_success(
        &server
            .graphql_json(
                UPDATE_TRACE_PARAMS,
                "UpdateTraceParams",
                json!({
                    "sessionId": session_id,
                    "traceParams": trace_params_json(28, 0.35),
                }),
            )
            .await,
        "updateTraceParams",
    );
    assert_bool_success(
        &server
            .graphql_json(
                UPDATE_BUCKET_DELTA,
                "UpdateBucketDelta",
                json!({
                    "sessionId": session_id,
                    "bucketDelta": 0.55,
                }),
            )
            .await,
        "updateBucketDelta",
    );
    assert_bool_success(
        &server
            .graphql_json(
                UPDATE_TARGET_SIMILARITY,
                "UpdateTargetSimilarity",
                json!({
                    "sessionId": session_id,
                    "targetSimilarity": 0.93,
                }),
            )
            .await,
        "updateTargetSimilarity",
    );
    assert_bool_success(
        &server
            .graphql_json(
                UPDATE_OBJECT_DETECTION_PARAMS,
                "UpdateObjectDetectionParams",
                json!({
                    "sessionId": session_id,
                    "objectDetectionParams": object_detection_params_json(0.18, 0.28, 0.42, 16, 0.22, 0.88),
                }),
            )
            .await,
        "updateObjectDetectionParams",
    );
    assert_bool_success(
        &server
            .graphql_json(
                ADD_OBJECT_AS_ASCII_ART,
                "AddObjectToBeDetectedAsAsciiArt",
                json!({
                    "sessionId": session_id,
                    "objectId": "ref-ascii",
                    "asciiArt": "#######\n#######\n#######\n#######\n#######\n#######\n#######",
                }),
            )
            .await,
        "addObjectToBeDetectedAsAsciiArt",
    );

    for frame in frames {
        assert_rectangles_payload(&get_rectangles(&server, session_id, frame).await);
    }

    assert_bool_success(
        &server
            .graphql_json(
                CREATE_ORDINARY_SESSION,
                "CreateOrdinarySession",
                json!({
                    "sessionId": reference_source_session_id,
                    "basicParams": basic_params_json(false, 15),
                    "results": "ABSOLUTE",
                }),
            )
            .await,
        "createOrdinarySession",
    );
    let reference_source_rectangles =
        get_rectangles(&server, reference_source_session_id, &frames[0]).await;
    let reference_bounding_box = first_bounding_box(&reference_source_rectangles);
    assert_bool_success(
        &add_object_as_image(
            &server,
            session_id,
            "ref-image",
            &frames[0].jpeg,
            reference_bounding_box,
        )
        .await,
        "addObjectToBeDetectedAsImage",
    );

    let session_state = server
        .graphql_json(
            FETCH_SESSION_STATE,
            "FetchSessionState",
            json!({ "sessionId": session_id }),
        )
        .await;
    assert_no_errors(&session_state);
    assert!(data(&session_state, "getOrdinarySession").is_null());
    assert!(data(&session_state, "getEyeSession").is_null());
    assert_eq!(
        data(&session_state, "getObjectSession")["basicParams"]["doGrayscale"],
        Value::Bool(true)
    );
    assert_eq!(
        data(&session_state, "getObjectSession")["basicParams"]["gradientThreshold"],
        Value::from(11)
    );
    assert_eq!(
        data(&session_state, "getObjectSession")["objectDetectionParams"]["tileParams"]["tileX"],
        Value::from(0.18)
    );
    assert_eq!(
        data(&session_state, "getObjectSession")["objectDetectionParams"]["tileParams"]["tileY"],
        Value::from(0.28)
    );
    assert_eq!(
        data(&session_state, "getObjectSession")["objectDetectionParams"]["bucketDelta"],
        Value::from(0.42)
    );
    assert_eq!(
        data(&session_state, "getObjectSession")["objectDetectionParams"]["traceParams"]["numSkeleton"],
        Value::from(16)
    );
    assert_eq!(
        data(&session_state, "getObjectSession")["objectDetectionParams"]["traceParams"]["closeSliceThreshold"],
        Value::from(0.22)
    );
    assert_eq!(
        data(&session_state, "getObjectSession")["objectDetectionParams"]["targetSimilarity"],
        Value::from(0.88)
    );
    assert_eq!(
        data(&session_state, "getObjectSession")["objectsToBeDetected"]
            .as_array()
            .map(Vec::len),
        Some(2)
    );
    assert!(data(&session_state, "getObjectSession")["results"].is_string());
    assert_eq!(
        data(&session_state, "getBasicParams")["gradientThreshold"],
        Value::from(11)
    );
    assert_eq!(
        data(&session_state, "getTileParams")["tileX"],
        Value::from(0.18)
    );
    assert_eq!(
        data(&session_state, "getTileParams")["tileY"],
        Value::from(0.28)
    );
    assert_eq!(
        data(&session_state, "getTraceParams")["numSkeleton"],
        Value::from(16)
    );
    assert_eq!(
        data(&session_state, "getTraceParams")["closeSliceThreshold"],
        Value::from(0.22)
    );
    assert_eq!(data(&session_state, "getBucketDelta"), &Value::from(0.42));
    assert_eq!(
        data(&session_state, "getTargetSimilarity"),
        &Value::from(0.88)
    );

    assert_error_message(
        &server
            .graphql_json(
                DELETE_REFERENCE_OBJECT,
                "DeleteReferenceObject",
                json!({
                    "sessionId": session_id,
                    "objectId": "missing-object",
                }),
            )
            .await,
        "Reference object not found",
    );
    assert_bool_success(
        &server
            .graphql_json(
                DELETE_REFERENCE_OBJECT,
                "DeleteReferenceObject",
                json!({
                    "sessionId": session_id,
                    "objectId": "ref-image",
                }),
            )
            .await,
        "deleteReferenceObject",
    );
    assert_bool_success(
        &server
            .graphql_json(
                DELETE_REFERENCE_OBJECT,
                "DeleteReferenceObject",
                json!({
                    "sessionId": session_id,
                    "objectId": "ref-ascii",
                }),
            )
            .await,
        "deleteReferenceObject",
    );

    let session_state_after_delete = server
        .graphql_json(
            FETCH_SESSION_STATE,
            "FetchSessionState",
            json!({ "sessionId": session_id }),
        )
        .await;
    assert_no_errors(&session_state_after_delete);
    assert_eq!(
        data(&session_state_after_delete, "getObjectSession")["objectsToBeDetected"]
            .as_array()
            .map(Vec::len),
        Some(0)
    );

    assert_error_contains(
        &add_object_as_image(
            &server,
            session_id,
            "bad-jpeg",
            b"still-not-a-jpeg",
            full_frame_rectangle_json(&frames[0]),
        )
        .await,
        "Failed to decode JPEG upload",
    );
    assert_error_message(
        &server
            .graphql_json(
                ADD_OBJECT_AS_ASCII_ART,
                "AddObjectToBeDetectedAsAsciiArt",
                json!({
                    "sessionId": session_id,
                    "objectId": "empty-ref",
                    "asciiArt": "##\n##",
                }),
            )
            .await,
        "No reference mosaics detected",
    );

    server.shutdown().await;
}

#[tokio::test(flavor = "multi_thread")]
async fn missing_session_graphql_operations_return_not_found_after_real_frame_processing() {
    let frames = example_frames();
    let server = TestServer::spawn().await;
    let existing_session_id = "existing-session";
    let missing_session_id = "missing-session";

    assert_bool_success(
        &server
            .graphql_json(
                CREATE_ORDINARY_SESSION,
                "CreateOrdinarySession",
                json!({
                    "sessionId": existing_session_id,
                    "basicParams": basic_params_json(false, 15),
                    "results": "ABSOLUTE",
                }),
            )
            .await,
        "createOrdinarySession",
    );

    for frame in frames {
        assert_rectangles_payload(&get_rectangles(&server, existing_session_id, frame).await);
    }

    let missing_state = server
        .graphql_json(
            FETCH_SESSION_STATE,
            "FetchSessionState",
            json!({ "sessionId": missing_session_id }),
        )
        .await;
    assert_no_errors(&missing_state);
    assert!(data(&missing_state, "getOrdinarySession").is_null());
    assert!(data(&missing_state, "getEyeSession").is_null());
    assert!(data(&missing_state, "getObjectSession").is_null());
    assert!(data(&missing_state, "getBasicParams").is_null());
    assert!(data(&missing_state, "getTileParams").is_null());
    assert!(data(&missing_state, "getTraceParams").is_null());
    assert!(data(&missing_state, "getBucketDelta").is_null());
    assert!(data(&missing_state, "getTargetSimilarity").is_null());

    assert_error_message(
        &server
            .graphql_json(
                UPDATE_BASIC_PARAMS,
                "UpdateBasicParams",
                json!({
                    "sessionId": missing_session_id,
                    "basicParams": basic_params_json(true, 7),
                }),
            )
            .await,
        "Session not found",
    );
    assert_error_message(
        &server
            .graphql_json(
                UPDATE_TILE_PARAMS,
                "UpdateTileParams",
                json!({
                    "sessionId": missing_session_id,
                    "tileParams": tile_params_json(0.2, 0.3),
                }),
            )
            .await,
        "Session not found",
    );
    assert_error_message(
        &server
            .graphql_json(
                UPDATE_TRACE_PARAMS,
                "UpdateTraceParams",
                json!({
                    "sessionId": missing_session_id,
                    "traceParams": trace_params_json(21, 0.18),
                }),
            )
            .await,
        "Session not found",
    );
    assert_error_message(
        &server
            .graphql_json(
                UPDATE_BUCKET_DELTA,
                "UpdateBucketDelta",
                json!({
                    "sessionId": missing_session_id,
                    "bucketDelta": 0.31,
                }),
            )
            .await,
        "Session not found",
    );
    assert_error_message(
        &server
            .graphql_json(
                UPDATE_TARGET_SIMILARITY,
                "UpdateTargetSimilarity",
                json!({
                    "sessionId": missing_session_id,
                    "targetSimilarity": 0.75,
                }),
            )
            .await,
        "Session not found",
    );
    assert_error_message(
        &server
            .graphql_json(
                UPDATE_EYE_PARAMS,
                "UpdateEyeParams",
                json!({
                    "sessionId": missing_session_id,
                    "eyeParams": eye_params_json(0.3, 0.4, 0.5, 8, 0.1, 0.8),
                }),
            )
            .await,
        "Session not found",
    );
    assert_error_message(
        &server
            .graphql_json(
                UPDATE_OBJECT_DETECTION_PARAMS,
                "UpdateObjectDetectionParams",
                json!({
                    "sessionId": missing_session_id,
                    "objectDetectionParams": object_detection_params_json(0.3, 0.4, 0.5, 8, 0.1, 0.8),
                }),
            )
            .await,
        "Session not found",
    );
    assert_error_message(
        &add_object_as_image(
            &server,
            missing_session_id,
            "missing-ref",
            &frames[0].jpeg,
            full_frame_rectangle_json(&frames[0]),
        )
        .await,
        "Session not found",
    );
    assert_error_message(
        &server
            .graphql_json(
                ADD_OBJECT_AS_ASCII_ART,
                "AddObjectToBeDetectedAsAsciiArt",
                json!({
                    "sessionId": missing_session_id,
                    "objectId": "missing-ref",
                    "asciiArt": "##\n##",
                }),
            )
            .await,
        "Session not found",
    );
    assert_error_message(
        &server
            .graphql_json(
                DELETE_REFERENCE_OBJECT,
                "DeleteReferenceObject",
                json!({
                    "sessionId": missing_session_id,
                    "objectId": "missing-ref",
                }),
            )
            .await,
        "Session not found",
    );
    assert_error_message(
        &server
            .graphql_json(
                DELETE_SESSION,
                "DeleteSession",
                json!({ "sessionId": missing_session_id }),
            )
            .await,
        "Session not found",
    );
    assert_error_message(
        &get_rectangles(&server, missing_session_id, &frames[0]).await,
        "Session not found",
    );

    server.shutdown().await;
}

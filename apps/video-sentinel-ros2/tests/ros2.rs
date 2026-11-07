#![warn(clippy::pedantic)]
#![allow(
    clippy::missing_errors_doc,
    clippy::missing_panics_doc,
    clippy::too_many_lines,
    clippy::unused_async_trait_impl
)]

use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::{Once, OnceLock};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use futures::{StreamExt, pin_mut};
use opencv::core::Vector as CvVector;
use opencv::imgcodecs;
use opencv::imgproc;
use opencv::prelude::*;
use ros2_client::{
    Context, DEFAULT_SUBSCRIPTION_QOS, MessageTypeName, Name, NodeName, NodeOptions, Subscription,
    builtin_interfaces::Time,
};
use tempfile::TempDir;
use tokio::sync::mpsc;
use tokio::time::timeout;
use video_rs::{Decoder, Frame};
use vs_core::mosaics::Results;
use vs_core::slices::{Rectangle, WrappedRgbImage};
use vs_ros2::ros2_node::{Detection2D, Detection2DArray, RosSessionType, default_output_topic};
use vs_service::service::{
    AddObjectToBeDetectedResult, BasicParamsInput, CreateEyeSessionResult,
    CreateObjectSessionResult, CreateOrdinarySessionResult, EyeParamsInput, GetRectanglesResult,
    ObjectDetectionParamsInput, Service, TileParamsInput, TraceParamsInput,
};

const SESSION_ID_ORDINARY: &str = "ordinary-session";
const SESSION_ID_EYE: &str = "eye-session";
const SESSION_ID_OBJECT: &str = "object-session";
const DETECTION_TYPE_PACKAGE: &str = "vision_msgs";
const DETECTION_TYPE_NAME: &str = "Detection2DArray";
const PUBLISHER_LIFETIME: Duration = Duration::from_secs(6);
const DETECTION_GRACE_PERIOD: Duration = Duration::from_secs(20);
const IMAGE_PUBLISH_COUNT: usize = 6;

static VIDEO_RS_INIT: Once = Once::new();
static EXAMPLE_FRAME: OnceLock<ExampleFrame> = OnceLock::new();
#[derive(Clone)]
struct ExampleFrame {
    jpeg: Vec<u8>,
}

struct ChildGuard {
    child: Child,
}

impl ChildGuard {
    fn spawn(binary_path: &Path, args: &[String]) -> Self {
        let child = Command::new("bash")
            .arg("-lc")
            .arg(ros_shell_command(binary_path, args))
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("ROS2 child process should start");
        Self { child }
    }

    fn terminate(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }

    fn try_wait(&mut self) -> Option<std::process::ExitStatus> {
        self.child
            .try_wait()
            .expect("child process status should be readable")
    }

    fn debug_output(&mut self) -> String {
        let mut output = String::new();
        if let Some(stdout) = self.child.stdout.as_mut() {
            let _ = stdout.read_to_string(&mut output);
        }
        if let Some(stderr) = self.child.stderr.as_mut() {
            let mut stderr_text = String::new();
            let _ = stderr.read_to_string(&mut stderr_text);
            if !stderr_text.is_empty() {
                if !output.is_empty() {
                    output.push_str("\n--- stderr ---\n");
                }
                output.push_str(&stderr_text);
            }
        }
        output
    }
}

impl Drop for ChildGuard {
    fn drop(&mut self) {
        self.terminate();
    }
}

struct TempDirImage {
    _dir: TempDir,
    path: PathBuf,
}

struct SessionTestCase {
    output_topic: String,
    config_path: PathBuf,
    _object_reference: Option<(TempDirImage, Rectangle)>,
    expected_signatures: Vec<String>,
}

#[tokio::test(flavor = "multi_thread")]
async fn ros2_nodes_publish_expected_rectangles_for_all_session_types() {
    let frame = example_frame();
    let unique = unique_suffix();
    let temp_dir = tempfile::tempdir().expect("tempdir should be created");
    let input_image_path = temp_dir.path().join("input.jpg");
    fs::write(&input_image_path, &frame.jpeg).expect("input image should be written");

    let image_topic = format!("/video_sentinel/test/{unique}/image");
    let mut cases = Vec::new();
    for (session_type, session_id_prefix) in [
        (RosSessionType::Ordinary, SESSION_ID_ORDINARY),
        (RosSessionType::Eye, SESSION_ID_EYE),
        (RosSessionType::Object, SESSION_ID_OBJECT),
    ] {
        let session_topic_segment = match session_type {
            RosSessionType::Ordinary => "ordinary",
            RosSessionType::Eye => "eye",
            RosSessionType::Object => "object",
        };
        let session_name = format!("{session_id_prefix}-{unique}");
        let output_topic = default_output_topic(&session_name, session_type);
        let node_name = format!("ros2_test_node_{session_topic_segment}_{unique}");
        let config_path = temp_dir
            .path()
            .join(format!("{session_topic_segment}.yaml"));
        let object_reference = if session_type == RosSessionType::Object {
            Some(prepare_object_reference(frame).await)
        } else {
            None
        };
        let expected_signatures = expected_detection_signatures(
            session_type,
            &session_name,
            frame,
            object_reference.as_ref(),
        )
        .await;
        write_config(
            &config_path,
            &node_name,
            session_type,
            &session_name,
            &image_topic,
            object_reference.as_ref(),
        );
        cases.push(SessionTestCase {
            output_topic,
            config_path,
            _object_reference: object_reference,
            expected_signatures,
        });
    }

    let context = Context::new().expect("test ROS2 context should be created");
    let mut subscriber_node = context
        .new_node(
            NodeName::new("/", &format!("subscriber_{unique}")).expect("subscriber node name"),
            NodeOptions::new(),
        )
        .expect("subscriber node should be created");
    let subscriptions: Vec<Subscription<Detection2DArray>> = cases
        .iter()
        .map(|case| {
            let topic = subscriber_node
                .create_topic(
                    &Name::parse(&case.output_topic).expect("output topic should parse"),
                    MessageTypeName::new(DETECTION_TYPE_PACKAGE, DETECTION_TYPE_NAME),
                    &DEFAULT_SUBSCRIPTION_QOS,
                )
                .expect("output topic should be created");
            subscriber_node
                .create_subscription(&topic, None)
                .expect("output subscriber should be created")
        })
        .collect();

    let subscriber_spinner = tokio::spawn(
        subscriber_node
            .spinner()
            .expect("subscriber spinner should be created")
            .spin(),
    );
    let mut ros2_nodes: Vec<_> = cases
        .iter()
        .map(|case| {
            ChildGuard::spawn(
                Path::new(env!("CARGO_BIN_EXE_video_sentinel_ros2_node")),
                &[
                    "--config".to_string(),
                    case.config_path.display().to_string(),
                ],
            )
        })
        .collect();

    timeout(Duration::from_secs(10), async {
        loop {
            for node in &mut ros2_nodes {
                ensure_background_process_running(node, "ROS2 node");
            }
            if subscriber_node.subscription_count(&image_topic) >= cases.len()
                && cases
                    .iter()
                    .all(|case| subscriber_node.publisher_count(&case.output_topic) > 0)
            {
                break;
            }
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
    })
    .await
    .expect("all ROS2 nodes should create their input subscriptions and output publishers");

    let (message_sender, mut message_receiver) = mpsc::unbounded_channel();
    let receive_tasks: Vec<_> = subscriptions
        .into_iter()
        .map(|subscription| {
            tokio::spawn(collect_detection_messages(
                subscription,
                message_sender.clone(),
            ))
        })
        .collect();
    drop(message_sender);
    let mut expected_signatures: Vec<_> = cases
        .iter()
        .flat_map(|case| case.expected_signatures.iter().cloned())
        .collect();
    expected_signatures.sort_unstable();
    let mut actual_signatures = Vec::new();

    let input_stamp_base = Time::now().to_nanos();
    let mut image_publisher = spawn_image_publisher(
        &image_topic,
        &input_image_path,
        cases.len(),
        input_stamp_base,
    );

    // The publisher lives for at most 6 seconds; detections may then arrive for 20 more seconds.
    let publishers_deadline = tokio::time::Instant::now() + PUBLISHER_LIFETIME;
    let detections_deadline = publishers_deadline + DETECTION_GRACE_PERIOD;
    let mut publishers_killed = false;
    let mut all_detections_received = false;
    let expected_frame_ids: std::collections::HashSet<_> = (0..IMAGE_PUBLISH_COUNT)
        .map(|index| format!("camera-{index}"))
        .collect();
    let mut actual_frame_ids = std::collections::HashSet::new();
    let mut detection_message_count = 0;
    while !all_detections_received {
        let now = tokio::time::Instant::now();
        if !publishers_killed && now >= publishers_deadline {
            image_publisher.terminate();
            publishers_killed = true;
        }
        if now >= detections_deadline {
            break;
        }
        let next_wakeup = if publishers_killed {
            detections_deadline
        } else {
            publishers_deadline
        };
        match tokio::time::timeout_at(next_wakeup, message_receiver.recv()).await {
            Ok(Some(Ok(message))) => {
                assert!(
                    expected_frame_ids.contains(&message.header.frame_id),
                    "unexpected output frame ID: {}",
                    message.header.frame_id
                );
                let expected_frame_index = message
                    .header
                    .frame_id
                    .strip_prefix("camera-")
                    .expect("output should retain its input frame ID")
                    .parse::<i64>()
                    .expect("frame ID should contain its input sequence number");
                assert_eq!(
                    message.header.stamp.to_nanos(),
                    input_stamp_base + expected_frame_index
                );
                actual_frame_ids.insert(message.header.frame_id.clone());
                detection_message_count += 1;
                actual_signatures.extend(detection_signatures(&message.detections));
                actual_signatures.sort_unstable();
                all_detections_received =
                    signatures_contain(&expected_signatures, &actual_signatures)
                        && actual_frame_ids.len() >= 2
                        && detection_message_count >= cases.len();
            }
            Ok(Some(Err(error))) => panic!("failed to receive detection message: {error}"),
            Ok(None) => panic!("all detection subscriptions ended unexpectedly"),
            Err(_) => {}
        }
    }

    image_publisher.terminate();
    ros2_nodes.iter_mut().for_each(ChildGuard::terminate);
    receive_tasks
        .iter()
        .for_each(tokio::task::JoinHandle::abort);
    subscriber_spinner.abort();

    if !all_detections_received {
        let node_output = ros2_nodes
            .iter_mut()
            .map(ChildGuard::debug_output)
            .collect::<Vec<_>>()
            .join("\n--- ROS2 node ---\n");
        panic!(
            "not all expected detections arrived within {:?} after publishing ({} of {} expected signatures collected)\n--- image publisher ---\n{}\n--- ROS2 nodes ---\n{node_output}",
            PUBLISHER_LIFETIME + DETECTION_GRACE_PERIOD,
            actual_signatures.len(),
            expected_signatures.len(),
            image_publisher.debug_output(),
        );
    }
}

fn ensure_background_process_running(child: &mut ChildGuard, process_name: &str) {
    if let Some(status) = child.try_wait() {
        let output = child.debug_output();
        panic!(
            "{process_name} exited early with status {status}{}\n",
            if output.is_empty() {
                String::new()
            } else {
                format!("\n\nchild output:\n{output}")
            }
        );
    }
}

async fn expected_detection_signatures(
    session_type: RosSessionType,
    session_id: &str,
    frame: &ExampleFrame,
    object_reference: Option<&(TempDirImage, Rectangle)>,
) -> Vec<String> {
    let mut service = Service::new();
    let results = Results::Absolute;
    let image = WrappedRgbImage::from_jpeg_bytes(&frame.jpeg)
        .expect("JPEG frame should convert to WrappedRgbImage");

    let rectangles = match session_type {
        RosSessionType::Ordinary => {
            assert!(matches!(
                service.create_ordinary_session(session_id, basic_params(), results),
                CreateOrdinarySessionResult::Success
            ));
            service.get_rectangles(session_id, image).await
        }
        RosSessionType::Eye => {
            assert!(matches!(
                service.create_eye_session(session_id, basic_params(), eye_params(), results),
                CreateEyeSessionResult::Success
            ));
            service.get_rectangles(session_id, image).await
        }
        RosSessionType::Object => {
            let (reference_image, reference_rectangle) =
                object_reference.expect("object session test case requires a reference image");
            assert!(matches!(
                service.create_object_session(
                    session_id,
                    basic_params(),
                    object_detection_params(),
                    results,
                ),
                CreateObjectSessionResult::Success
            ));
            let reference = WrappedRgbImage::from_file(
                reference_image
                    .path
                    .to_str()
                    .expect("reference image path should be valid UTF-8"),
            )
            .expect("reference image should load");
            assert!(matches!(
                service.add_object_to_be_detected_as_image(
                    session_id,
                    "reference-object",
                    &reference,
                    *reference_rectangle,
                ),
                AddObjectToBeDetectedResult::Success
            ));
            service.get_rectangles(session_id, image).await
        }
    };

    match rectangles {
        GetRectanglesResult::Success(rectangles) => {
            let mut signatures: Vec<_> = rectangles
                .into_iter()
                .map(|rectangle| {
                    signature_from_rectangle(&rectangle.bounding_box, rectangle.label.as_deref())
                })
                .collect();
            signatures.sort_unstable();
            signatures
        }
        GetRectanglesResult::SessionNotFound => panic!("reference session should exist"),
        GetRectanglesResult::ComputationFailed(error) => {
            panic!("failed to compute expected detections: {error}")
        }
    }
}

async fn prepare_object_reference(frame: &ExampleFrame) -> (TempDirImage, Rectangle) {
    let reference_rectangle = reference_bounding_box(frame).await;
    let reference_image = write_reference_image(frame);
    (reference_image, reference_rectangle)
}

async fn collect_detection_messages(
    subscriber: Subscription<Detection2DArray>,
    sender: mpsc::UnboundedSender<Result<Detection2DArray, String>>,
) {
    let stream = subscriber.async_stream();
    pin_mut!(stream);
    while let Some(message_result) = stream.next().await {
        match message_result {
            Ok((message, _message_info)) => {
                if sender.send(Ok(message)).is_err() {
                    return;
                }
            }
            Err(error) => {
                let _ = sender.send(Err(format!(
                    "failed to receive a detection message: {error}"
                )));
                return;
            }
        }
    }
    let _ = sender.send(Err("detection stream ended unexpectedly".to_string()));
}

fn spawn_image_publisher(
    image_topic: &str,
    image_path: &Path,
    subscriber_count: usize,
    base_stamp_nanos: i64,
) -> ChildGuard {
    ChildGuard::spawn(
        Path::new(env!("CARGO_BIN_EXE_image_publisher_app")),
        &[
            "--topic".to_string(),
            image_topic.to_string(),
            "--image".to_string(),
            image_path.to_string_lossy().into_owned(),
            "--wait-for-subscribers".to_string(),
            subscriber_count.to_string(),
            "--startup-delay-ms".to_string(),
            "0".to_string(),
            "--post-publish-delay-ms".to_string(),
            "100".to_string(),
            "--publish-count".to_string(),
            IMAGE_PUBLISH_COUNT.to_string(),
            "--publish-interval-ms".to_string(),
            "250".to_string(),
            "--vary-frame-id".to_string(),
            "--base-stamp-nanos".to_string(),
            base_stamp_nanos.to_string(),
        ],
    )
}

async fn reference_bounding_box(frame: &ExampleFrame) -> Rectangle {
    let mut service = Service::new();
    assert!(matches!(
        service.create_ordinary_session("reference-session", basic_params(), Results::Absolute),
        CreateOrdinarySessionResult::Success
    ));
    let image = WrappedRgbImage::from_jpeg_bytes(&frame.jpeg)
        .expect("reference JPEG frame should convert to image");
    match service.get_rectangles("reference-session", image).await {
        GetRectanglesResult::Success(rectangles) => {
            rectangles
                .first()
                .expect("reference frame should produce at least one rectangle")
                .bounding_box
        }
        GetRectanglesResult::SessionNotFound => panic!("reference session should exist"),
        GetRectanglesResult::ComputationFailed(error) => {
            panic!("reference bounding box computation failed: {error}")
        }
    }
}

fn write_reference_image(frame: &ExampleFrame) -> TempDirImage {
    let dir = tempfile::tempdir().expect("reference tempdir should be created");
    let path = dir.path().join("reference.jpg");
    fs::write(&path, &frame.jpeg).expect("reference image should be written");
    TempDirImage { _dir: dir, path }
}

fn write_config(
    config_path: &Path,
    node_name: &str,
    session_type: RosSessionType,
    session_id: &str,
    image_topic: &str,
    object_reference: Option<&(TempDirImage, Rectangle)>,
) {
    let session_yaml = match (session_type, object_reference) {
        (RosSessionType::Ordinary, _) => format!(
            "session:\n  type: ordinary\n  id: {session_id}\n  basic_params:\n    do_grayscale: false\n    gradient_threshold: 15\n"
        ),
        (RosSessionType::Eye, _) => format!(
            "session:\n  type: eye\n  id: {session_id}\n  basic_params:\n    do_grayscale: false\n    gradient_threshold: 15\n  eye_params:\n    tile_params:\n      tile_x: 0.5\n      tile_y: 0.5\n    bucket_delta: 0.25\n    trace_params:\n      num_skeleton: 12\n    target_similarity: 0.9\n"
        ),
        (RosSessionType::Object, Some((reference_image, rectangle))) => format!(
            "session:\n  type: object\n  id: {session_id}\n  basic_params:\n    do_grayscale: false\n    gradient_threshold: 15\n  object_detection_params:\n    tile_params:\n      tile_x: 0.5\n      tile_y: 0.5\n    bucket_delta: 0.25\n    trace_params:\n      num_skeleton: 12\n    target_similarity: 0.9\n  reference_objects:\n    - source: image_path\n      object_id: reference-object\n      image_path: {}\n      surrounding_rectangle:\n        top_left:\n          x: {}\n          y: {}\n        bottom_right:\n          x: {}\n          y: {}\n",
            reference_image.path.display(),
            rectangle.get_top_left().x,
            rectangle.get_top_left().y,
            rectangle.get_bottom_right().x,
            rectangle.get_bottom_right().y
        ),
        (RosSessionType::Object, None) => {
            panic!("object session test case requires a reference image")
        }
    };

    let yaml = format!(
        "node:\n  name: {node_name}\n  namespace: /\n  max_message_age_ms: 120000\ntopics:\n  image: {image_topic}\n{session_yaml}"
    );
    fs::write(config_path, yaml).expect("config should be written");
}

fn detection_signatures(detections: &[Detection2D]) -> Vec<String> {
    let mut signatures: Vec<_> = detections
        .iter()
        .map(|detection| {
            let mut labels: Vec<_> = detection
                .results
                .iter()
                .map(|result| result.hypothesis.class_id.as_str())
                .collect();
            labels.sort_unstable();
            let label = if labels.is_empty() {
                None
            } else {
                Some(labels.join(","))
            };
            format!(
                "{}|{:.6}|{:.6}|{:.6}|{:.6}",
                label.unwrap_or_default(),
                detection.bbox.center.position.x,
                detection.bbox.center.position.y,
                detection.bbox.size_x,
                detection.bbox.size_y,
            )
        })
        .collect();
    signatures.sort_unstable();
    signatures
}

fn signature_from_rectangle(rectangle: &Rectangle, label: Option<&str>) -> String {
    let top_left = rectangle.get_top_left();
    let bottom_right = rectangle.get_bottom_right();
    let center_x = f64::midpoint(top_left.x, bottom_right.x);
    let center_y = f64::midpoint(top_left.y, bottom_right.y);
    format!(
        "{}|{center_x:.6}|{center_y:.6}|{:.6}|{:.6}",
        label.unwrap_or_default(),
        rectangle.get_width(),
        rectangle.get_height(),
    )
}

fn signatures_contain(expected: &[String], actual: &[String]) -> bool {
    let mut actual_counts = std::collections::HashMap::new();
    for signature in actual {
        *actual_counts.entry(signature).or_insert(0) += 1;
    }
    let mut expected_counts = std::collections::HashMap::new();
    for signature in expected {
        *expected_counts.entry(signature).or_insert(0) += 1;
    }
    expected_counts
        .iter()
        .all(|(signature, count)| actual_counts.get(signature).copied().unwrap_or(0) >= *count)
}

fn basic_params() -> BasicParamsInput {
    BasicParamsInput {
        do_grayscale: false,
        gradient_threshold: 15,
    }
}

fn eye_params() -> EyeParamsInput {
    EyeParamsInput {
        tile_params: TileParamsInput {
            tile_x: 0.5,
            tile_y: 0.5,
        },
        bucket_delta: 0.25,
        trace_params: TraceParamsInput { num_skeleton: 12 },
        target_similarity: 0.9,
    }
}

fn object_detection_params() -> ObjectDetectionParamsInput {
    ObjectDetectionParamsInput {
        tile_params: TileParamsInput {
            tile_x: 0.5,
            tile_y: 0.5,
        },
        bucket_delta: 0.25,
        trace_params: TraceParamsInput { num_skeleton: 12 },
        target_similarity: 0.9,
    }
}

fn example_frame() -> &'static ExampleFrame {
    EXAMPLE_FRAME.get_or_init(load_example_frame)
}

fn load_example_frame() -> ExampleFrame {
    VIDEO_RS_INIT.call_once(|| {
        video_rs::init().expect("video-rs should initialize exactly once");
    });

    let video_path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../video")
        .join("BillardTakeoff.mp4");

    let mut decoder = Decoder::new(video_path.as_path()).expect("example video should open");
    let (_timestamp, frame) = decoder
        .decode_iter()
        .next()
        .expect("example video should have at least one frame")
        .expect("example frame should decode");
    frame_to_example_frame(&frame)
}

fn frame_to_example_frame(frame: &Frame) -> ExampleFrame {
    let (height, width, channels) = frame.dim();
    assert_eq!(channels, 3, "example video should decode as RGB");
    let rgb = if let Some(slice) = frame.as_slice_memory_order() {
        slice.to_vec()
    } else {
        frame.iter().copied().collect()
    };
    ExampleFrame {
        jpeg: jpeg_bytes_from_rgb(
            u32::try_from(width).expect("frame width should fit into u32"),
            u32::try_from(height).expect("frame height should fit into u32"),
            &rgb,
        ),
    }
}

fn jpeg_bytes_from_rgb(width: u32, height: u32, rgb_bytes: &[u8]) -> Vec<u8> {
    let image = WrappedRgbImage::from_rgb_data(width, height, rgb_bytes)
        .expect("frame should convert to WrappedRgbImage");
    let rgb = image
        .get_mat()
        .try_clone()
        .expect("wrapped image should clone");
    let mut bgr = Mat::default();
    imgproc::cvt_color(&rgb, &mut bgr, imgproc::COLOR_RGB2BGR, 0)
        .expect("RGB image should convert to BGR");
    let mut encoded = CvVector::<u8>::new();
    imgcodecs::imencode(".jpg", &bgr, &mut encoded, &CvVector::<i32>::new())
        .expect("reference frame should encode as JPEG");
    encoded.to_vec()
}

fn ros_shell_command(binary_path: &Path, args: &[String]) -> String {
    let setup_script = ros2_setup_script();
    let binary = shell_escape(&binary_path.to_string_lossy());
    let args = args
        .iter()
        .map(|arg| shell_escape(arg))
        .collect::<Vec<_>>()
        .join(" ");
    format!(
        "source {} && exec {} {}",
        shell_escape(&setup_script),
        binary,
        args
    )
}

fn ros2_setup_script() -> String {
    if let Ok(distro) = std::env::var("ROS_DISTRO") {
        let candidate = Path::new("/opt/ros").join(&distro).join("setup.bash");
        if candidate.exists() {
            return candidate.to_string_lossy().into_owned();
        }
    }

    let mut candidates: Vec<_> = fs::read_dir("/opt/ros")
        .expect("/opt/ros should exist for ROS2 tests")
        .filter_map(Result::ok)
        .map(|entry| entry.path().join("setup.bash"))
        .filter(|path| path.exists())
        .collect();
    candidates.sort();
    candidates
        .last()
        .expect("at least one ROS2 setup.bash should exist")
        .to_string_lossy()
        .into_owned()
}

fn shell_escape(value: &str) -> String {
    format!("'{}'", value.replace('\'', "'\\''"))
}

fn unique_suffix() -> String {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system time should be after epoch")
        .as_nanos();
    format!("u{}_{}", std::process::id(), nanos)
}

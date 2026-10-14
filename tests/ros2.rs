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
use std::process::{Child, Command, Output, Stdio};
use std::sync::{Once, OnceLock};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use futures::{StreamExt, pin_mut};
use opencv::core::Vector as CvVector;
use opencv::imgcodecs;
use opencv::imgproc;
use opencv::prelude::*;
use ros2_client::{
    Context, DEFAULT_SUBSCRIPTION_QOS, MessageTypeName, Name, NodeName, NodeOptions, Subscription,
};
use tempfile::TempDir;
use tokio::sync::Mutex;
use tokio::time::timeout;
use video_rs::{Decoder, Frame};
use video_sentinel::mosaics::Results;
use video_sentinel::ros2_node::{
    Detection2D, Detection2DArray, RosSessionType, default_output_topic,
};
use video_sentinel::service::{
    AddObjectToBeDetectedResult, BasicParamsInput, CreateEyeSessionResult,
    CreateObjectSessionResult, CreateOrdinarySessionResult, EyeParamsInput, GetRectanglesResult,
    ObjectDetectionParamsInput, Service, TileParamsInput, TraceParamsInput,
};
use video_sentinel::slices::{Rectangle, WrappedRgbImage};

const SESSION_ID_ORDINARY: &str = "ordinary-session";
const SESSION_ID_EYE: &str = "eye-session";
const SESSION_ID_OBJECT: &str = "object-session";
const DETECTION_TYPE_PACKAGE: &str = "vision_msgs";
const DETECTION_TYPE_NAME: &str = "Detection2DArray";

static VIDEO_RS_INIT: Once = Once::new();
static EXAMPLE_FRAME: OnceLock<ExampleFrame> = OnceLock::new();
static ROS2_TEST_MUTEX: OnceLock<Mutex<()>> = OnceLock::new();

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

#[tokio::test(flavor = "multi_thread")]
async fn ros2_node_publishes_rectangles_for_ordinary_session() {
    let _guard = ros2_test_lock().lock().await;
    run_session_case(RosSessionType::Ordinary, SESSION_ID_ORDINARY).await;
}

#[tokio::test(flavor = "multi_thread")]
async fn ros2_node_publishes_rectangles_for_eye_session() {
    let _guard = ros2_test_lock().lock().await;
    run_session_case(RosSessionType::Eye, SESSION_ID_EYE).await;
}

#[tokio::test(flavor = "multi_thread")]
async fn ros2_node_publishes_rectangles_for_object_session() {
    let _guard = ros2_test_lock().lock().await;
    run_session_case(RosSessionType::Object, SESSION_ID_OBJECT).await;
}

async fn run_session_case(session_type: RosSessionType, session_id_prefix: &str) {
    let frame = example_frame();
    let unique = unique_suffix();
    let session_topic_segment = match session_type {
        RosSessionType::Ordinary => "ordinary",
        RosSessionType::Eye => "eye",
        RosSessionType::Object => "object",
    };
    let session_name = format!("{session_id_prefix}-{unique}");
    let image_topic = format!("/video_sentinel/test/{unique}/{session_topic_segment}/image");
    let output_topic = default_output_topic(&session_name, session_type);
    let node_name = format!("ros2_test_node_{unique}");

    let temp_dir = tempfile::tempdir().expect("tempdir should be created");
    let input_image_path = temp_dir.path().join("input.jpg");
    fs::write(&input_image_path, &frame.jpeg).expect("input image should be written");

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

    let config_path = temp_dir.path().join("session.yaml");
    write_config(
        &config_path,
        &node_name,
        session_type,
        &session_name,
        &image_topic,
        object_reference.as_ref(),
    );

    let mut ros2_node = ChildGuard::spawn(
        Path::new(env!("CARGO_BIN_EXE_video_sentinel_ros2_node")),
        &[
            "--config".to_string(),
            config_path.to_string_lossy().into_owned(),
        ],
    );
    wait_for_background_process(&mut ros2_node, "ROS2 node").await;

    let context = Context::new().expect("test ROS2 context should be created");
    let mut subscriber_node = context
        .new_node(
            NodeName::new("/", &format!("subscriber_{unique}")).expect("subscriber node name"),
            NodeOptions::new(),
        )
        .expect("subscriber node should be created");

    let rectangles_topic = subscriber_node
        .create_topic(
            &Name::parse(&output_topic).expect("output topic should parse"),
            MessageTypeName::new(DETECTION_TYPE_PACKAGE, DETECTION_TYPE_NAME),
            &DEFAULT_SUBSCRIPTION_QOS,
        )
        .expect("output topic should be created");
    let rectangles_subscriber: Subscription<Detection2DArray> = subscriber_node
        .create_subscription(&rectangles_topic, None)
        .expect("output subscriber should be created");

    let subscriber_spinner = tokio::spawn(
        subscriber_node
            .spinner()
            .expect("subscriber spinner should be created")
            .spin(),
    );
    tokio::time::sleep(Duration::from_secs(2)).await;
    wait_for_background_process(&mut ros2_node, "ROS2 node").await;

    let receive_task = tokio::spawn(collect_detection_message(rectangles_subscriber));
    tokio::time::sleep(Duration::from_secs(3)).await;
    run_image_publisher(&image_topic, &input_image_path)
        .unwrap_or_else(|error| panic_with_child_output(&error, &mut ros2_node));

    let message = timeout(Duration::from_secs(120), receive_task)
        .await
        .unwrap_or_else(|_| {
            panic_with_child_output("timed out waiting for detections", &mut ros2_node)
        })
        .expect("receive task should join")
        .unwrap_or_else(|error| panic_with_child_output(&error, &mut ros2_node));
    subscriber_spinner.abort();

    assert_eq!(message.header.frame_id, "camera");

    let actual_signatures = detection_signatures(&message.detections);
    assert_detection_signatures_eq(&expected_signatures, &actual_signatures);
}

async fn wait_for_background_process(child: &mut ChildGuard, process_name: &str) {
    for _attempt in 0..50 {
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
        tokio::time::sleep(Duration::from_millis(100)).await;
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

async fn collect_detection_message(
    subscriber: Subscription<Detection2DArray>,
) -> Result<Detection2DArray, String> {
    let stream = subscriber.async_stream();
    pin_mut!(stream);
    let next = timeout(Duration::from_secs(120), stream.next())
        .await
        .map_err(|_| "timed out waiting for a detection message".to_string())?;
    let Some(message_result) = next else {
        return Err("detection stream ended unexpectedly".to_string());
    };
    let (message, _message_info) = message_result
        .map_err(|error| format!("failed to receive a detection message: {error}"))?;
    Ok(message)
}

fn run_image_publisher(image_topic: &str, image_path: &Path) -> Result<(), String> {
    let output = run_ros_command(
        Path::new(env!("CARGO_BIN_EXE_image_publisher_app")),
        &[
            "--topic".to_string(),
            image_topic.to_string(),
            "--image".to_string(),
            image_path.to_string_lossy().into_owned(),
            "--startup-delay-ms".to_string(),
            "1000".to_string(),
            "--post-publish-delay-ms".to_string(),
            "2500".to_string(),
            "--publish-count".to_string(),
            "8".to_string(),
            "--publish-interval-ms".to_string(),
            "750".to_string(),
        ],
    )?;

    if output.status.success() {
        return Ok(());
    }

    Err(format!(
        "image publisher command failed with status {}:\n{}",
        output
            .status
            .code()
            .map_or_else(|| "signal".to_string(), |code| code.to_string()),
        command_output_text(&output),
    ))
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
            "session:\n  type: eye\n  id: {session_id}\n  basic_params:\n    do_grayscale: false\n    gradient_threshold: 15\n  eye_params:\n    tile_params:\n      tile_x: 0.5\n      tile_y: 0.5\n    bucket_delta: 0.25\n    trace_params:\n      num_skeleton: 12\n      close_slice_threshold: 0.2\n    target_similarity: 0.9\n"
        ),
        (RosSessionType::Object, Some((reference_image, rectangle))) => format!(
            "session:\n  type: object\n  id: {session_id}\n  basic_params:\n    do_grayscale: false\n    gradient_threshold: 15\n  object_detection_params:\n    tile_params:\n      tile_x: 0.5\n      tile_y: 0.5\n    bucket_delta: 0.25\n    trace_params:\n      num_skeleton: 12\n      close_slice_threshold: 0.2\n    target_similarity: 0.9\n  reference_objects:\n    - source: image_path\n      object_id: reference-object\n      image_path: {}\n      surrounding_rectangle:\n        top_left:\n          x: {}\n          y: {}\n        bottom_right:\n          x: {}\n          y: {}\n",
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

fn panic_with_child_output(message: &str, child: &mut ChildGuard) -> ! {
    child.terminate();
    let output = child.debug_output();
    assert!(!output.is_empty(), "{message}");
    panic!("{message}\n\nchild output:\n{output}");
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

fn assert_detection_signatures_eq(expected: &[String], actual: &[String]) {
    assert_eq!(
        expected.len(),
        actual.len(),
        "expected {} detections, got {}",
        expected.len(),
        actual.len()
    );

    for (index, (expected_signature, actual_signature)) in expected.iter().zip(actual).enumerate() {
        assert_eq!(
            expected_signature, actual_signature,
            "detection signature mismatch at index {index}"
        );
    }
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
        trace_params: TraceParamsInput {
            num_skeleton: 12,
            close_slice_threshold: 0.2,
        },
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
        trace_params: TraceParamsInput {
            num_skeleton: 12,
            close_slice_threshold: 0.2,
        },
        target_similarity: 0.9,
    }
}

fn ros2_test_lock() -> &'static Mutex<()> {
    ROS2_TEST_MUTEX.get_or_init(|| Mutex::new(()))
}

fn example_frame() -> &'static ExampleFrame {
    EXAMPLE_FRAME.get_or_init(load_example_frame)
}

fn load_example_frame() -> ExampleFrame {
    VIDEO_RS_INIT.call_once(|| {
        video_rs::init().expect("video-rs should initialize exactly once");
    });

    let video_path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("video")
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

fn run_ros_command(binary_path: &Path, args: &[String]) -> Result<Output, String> {
    Command::new("bash")
        .arg("-lc")
        .arg(ros_shell_command(binary_path, args))
        .output()
        .map_err(|error| format!("failed to start `{}`: {error}", binary_path.display()))
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

fn command_output_text(output: &Output) -> String {
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    match (stdout.trim().is_empty(), stderr.trim().is_empty()) {
        (true, true) => String::new(),
        (false, true) => stdout.into_owned(),
        (true, false) => stderr.into_owned(),
        (false, false) => format!("{stdout}\n--- stderr ---\n{stderr}"),
    }
}

fn unique_suffix() -> String {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system time should be after epoch")
        .as_nanos();
    format!("u{}_{}", std::process::id(), nanos)
}

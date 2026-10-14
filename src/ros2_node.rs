use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};

use clap::Parser;
use futures::{StreamExt, pin_mut};
use ros2_client::{
    Context, DEFAULT_PUBLISHER_QOS, DEFAULT_SUBSCRIPTION_QOS, MessageTypeName, Name, NodeName,
    NodeOptions, builtin_interfaces::Time,
};
use rs_math3d::Vec3d;
use serde::{Deserialize, Serialize};

use crate::mosaics::Results;
use crate::service::{
    BasicParamsInput, CreateEyeSessionResult, CreateObjectSessionResult,
    CreateOrdinarySessionResult, EyeParamsInput, GetRectanglesResult, ObjectDetectionParamsInput,
    Service, TileParamsInput, TraceParamsInput,
};
use crate::slices::{Rectangle, WrappedRgbImage};

const IMAGE_TYPE_PACKAGE: &str = "sensor_msgs";
const IMAGE_TYPE_NAME: &str = "CompressedImage";
const DETECTION_TYPE_PACKAGE: &str = "vision_msgs";
const DETECTION_TYPE_NAME: &str = "Detection2DArray";

#[derive(Debug, Parser)]
#[command(name = "video-sentinel-ros2-node")]
pub struct CliArgs {
    #[arg(long)]
    pub config: PathBuf,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RosSessionType {
    Ordinary,
    Eye,
    Object,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Ros2NodeConfig {
    #[serde(default)]
    node: NodeConfig,
    topics: TopicsConfig,
    session: SessionConfig,
}

#[derive(Debug, Clone, Deserialize)]
struct NodeConfig {
    #[serde(default = "default_node_name")]
    name: String,
    #[serde(default = "default_node_namespace")]
    namespace: String,
    #[serde(default)]
    output_frame_id: Option<String>,
    #[serde(default = "default_max_message_age_ms")]
    max_message_age_ms: u64,
}

#[derive(Debug, Clone, Deserialize)]
struct TopicsConfig {
    image: String,
    #[serde(default)]
    output: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum SessionConfig {
    Ordinary {
        id: String,
        #[serde(default)]
        basic_params: BasicParamsYaml,
        #[serde(default)]
        results: ResultsModeYaml,
    },
    Eye {
        id: String,
        #[serde(default)]
        basic_params: BasicParamsYaml,
        #[serde(default)]
        results: ResultsModeYaml,
        #[serde(default)]
        eye_params: EyeParamsYaml,
    },
    Object {
        id: String,
        #[serde(default)]
        basic_params: BasicParamsYaml,
        #[serde(default)]
        results: ResultsModeYaml,
        #[serde(default)]
        object_detection_params: ObjectDetectionParamsYaml,
        #[serde(default)]
        reference_objects: Vec<ReferenceObjectYaml>,
    },
}

#[derive(Debug, Clone, Copy, Deserialize)]
struct BasicParamsYaml {
    #[serde(default)]
    do_grayscale: bool,
    #[serde(default = "default_gradient_threshold")]
    gradient_threshold: u8,
}

#[derive(Debug, Clone, Copy, Deserialize)]
struct TileParamsYaml {
    #[serde(default = "default_tile_fraction")]
    tile_x: f64,
    #[serde(default = "default_tile_fraction")]
    tile_y: f64,
}

#[derive(Debug, Clone, Copy, Deserialize)]
struct TraceParamsYaml {
    #[serde(default = "default_num_skeleton")]
    num_skeleton: usize,
    #[serde(default = "default_close_slice_threshold")]
    close_slice_threshold: f64,
}

#[derive(Debug, Clone, Copy, Deserialize)]
struct EyeParamsYaml {
    #[serde(default)]
    tile_params: TileParamsYaml,
    #[serde(default = "default_bucket_delta")]
    bucket_delta: f64,
    #[serde(default)]
    trace_params: TraceParamsYaml,
    #[serde(default = "default_target_similarity")]
    target_similarity: f64,
}

#[derive(Debug, Clone, Copy, Deserialize)]
struct ObjectDetectionParamsYaml {
    #[serde(default)]
    tile_params: TileParamsYaml,
    #[serde(default = "default_bucket_delta")]
    bucket_delta: f64,
    #[serde(default)]
    trace_params: TraceParamsYaml,
    #[serde(default = "default_target_similarity")]
    target_similarity: f64,
}

#[derive(Debug, Clone, Copy, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
enum ResultsModeYaml {
    #[default]
    Absolute,
    Relative,
}

#[derive(Debug, Clone, Deserialize)]
struct ReferenceObjectYaml {
    object_id: String,
    image_path: PathBuf,
    surrounding_rectangle: RectangleYaml,
}

#[derive(Debug, Clone, Copy, Deserialize)]
struct RectangleYaml {
    top_left: Point2Yaml,
    bottom_right: Point2Yaml,
}

#[derive(Debug, Clone, Copy, Deserialize)]
struct Point2Yaml {
    x: f64,
    y: f64,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Header {
    pub stamp: Time,
    pub frame_id: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct CompressedSensorImage {
    pub header: Header,
    pub format: String,
    pub data: Vec<u8>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Point2D {
    pub x: f64,
    pub y: f64,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Pose2D {
    pub position: Point2D,
    pub theta: f64,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct BoundingBox2D {
    pub center: Pose2D,
    pub size_x: f64,
    pub size_y: f64,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct ObjectHypothesis {
    pub class_id: String,
    pub score: f64,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Point {
    pub x: f64,
    pub y: f64,
    pub z: f64,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Quaternion {
    pub x: f64,
    pub y: f64,
    pub z: f64,
    pub w: f64,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Pose {
    pub position: Point,
    pub orientation: Quaternion,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct PoseWithCovariance {
    pub pose: Pose,
    #[serde(with = "serde_big_array::BigArray")]
    pub covariance: [f64; 36],
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct ObjectHypothesisWithPose {
    pub hypothesis: ObjectHypothesis,
    pub pose: PoseWithCovariance,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Detection2D {
    pub header: Header,
    pub results: Vec<ObjectHypothesisWithPose>,
    pub bbox: BoundingBox2D,
    pub id: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Detection2DArray {
    pub header: Header,
    pub detections: Vec<Detection2D>,
}

impl Default for NodeConfig {
    fn default() -> Self {
        Self {
            name: default_node_name(),
            namespace: default_node_namespace(),
            output_frame_id: None,
            max_message_age_ms: default_max_message_age_ms(),
        }
    }
}

impl Default for BasicParamsYaml {
    fn default() -> Self {
        Self {
            do_grayscale: false,
            gradient_threshold: default_gradient_threshold(),
        }
    }
}

impl Default for TileParamsYaml {
    fn default() -> Self {
        Self {
            tile_x: default_tile_fraction(),
            tile_y: default_tile_fraction(),
        }
    }
}

impl Default for TraceParamsYaml {
    fn default() -> Self {
        Self {
            num_skeleton: default_num_skeleton(),
            close_slice_threshold: default_close_slice_threshold(),
        }
    }
}

impl Default for EyeParamsYaml {
    fn default() -> Self {
        Self {
            tile_params: TileParamsYaml::default(),
            bucket_delta: default_bucket_delta(),
            trace_params: TraceParamsYaml::default(),
            target_similarity: default_target_similarity(),
        }
    }
}

impl Default for ObjectDetectionParamsYaml {
    fn default() -> Self {
        Self {
            tile_params: TileParamsYaml::default(),
            bucket_delta: default_bucket_delta(),
            trace_params: TraceParamsYaml::default(),
            target_similarity: default_target_similarity(),
        }
    }
}

impl Ros2NodeConfig {
    pub fn load(path: &Path) -> Result<Self, Box<dyn Error>> {
        let contents = fs::read_to_string(path)?;
        let mut config: Self = serde_yaml::from_str(&contents)?;
        config.resolve_paths(path)?;
        config.validate()?;
        Ok(config)
    }

    fn resolve_paths(&mut self, path: &Path) -> Result<(), Box<dyn Error>> {
        let Some(base_dir) = path.parent() else {
            return Ok(());
        };
        if let SessionConfig::Object {
            reference_objects, ..
        } = &mut self.session
        {
            for reference_object in reference_objects {
                if reference_object.image_path.is_relative() {
                    reference_object.image_path = base_dir.join(&reference_object.image_path);
                }
            }
        }
        Ok(())
    }

    fn validate(&self) -> Result<(), Box<dyn Error>> {
        if matches!(self.session.results(), ResultsModeYaml::Relative) {
            return Err(
                "ROS2 publishing currently supports only `results: absolute` because \
                 vision_msgs/Detection2DArray expects pixel-based bounding boxes"
                    .into(),
            );
        }

        match &self.session {
            SessionConfig::Object {
                reference_objects, ..
            } => {
                for reference_object in reference_objects {
                    if !reference_object.image_path.exists() {
                        return Err(format!(
                            "reference image `{}` does not exist",
                            reference_object.image_path.display()
                        )
                        .into());
                    }
                }
            }
            SessionConfig::Ordinary { .. } | SessionConfig::Eye { .. } => {}
        }

        Name::parse(&self.topics.image)
            .map_err(|error| format!("invalid input topic `{}`: {error}", self.topics.image))?;
        Name::parse(&self.output_topic())
            .map_err(|error| format!("invalid output topic `{}`: {error}", self.output_topic()))?;

        Ok(())
    }

    #[must_use]
    pub fn output_topic(&self) -> String {
        self.topics
            .output
            .clone()
            .unwrap_or_else(|| default_output_topic(self.session.id(), self.session.kind()))
    }
}

impl SessionConfig {
    #[must_use]
    pub fn id(&self) -> &str {
        match self {
            SessionConfig::Ordinary { id, .. }
            | SessionConfig::Eye { id, .. }
            | SessionConfig::Object { id, .. } => id,
        }
    }

    #[must_use]
    pub fn kind(&self) -> RosSessionType {
        match self {
            SessionConfig::Ordinary { .. } => RosSessionType::Ordinary,
            SessionConfig::Eye { .. } => RosSessionType::Eye,
            SessionConfig::Object { .. } => RosSessionType::Object,
        }
    }

    #[must_use]
    fn results(&self) -> ResultsModeYaml {
        match self {
            SessionConfig::Ordinary { results, .. }
            | SessionConfig::Eye { results, .. }
            | SessionConfig::Object { results, .. } => *results,
        }
    }
}

impl BasicParamsYaml {
    fn into_input(self) -> BasicParamsInput {
        BasicParamsInput {
            do_grayscale: self.do_grayscale,
            gradient_threshold: self.gradient_threshold,
        }
    }
}

impl TileParamsYaml {
    fn into_input(self) -> TileParamsInput {
        TileParamsInput {
            tile_x: self.tile_x,
            tile_y: self.tile_y,
        }
    }
}

impl TraceParamsYaml {
    fn into_input(self) -> TraceParamsInput {
        TraceParamsInput {
            num_skeleton: self.num_skeleton,
            close_slice_threshold: self.close_slice_threshold,
        }
    }
}

impl EyeParamsYaml {
    fn into_input(self) -> EyeParamsInput {
        EyeParamsInput {
            tile_params: self.tile_params.into_input(),
            bucket_delta: self.bucket_delta,
            trace_params: self.trace_params.into_input(),
            target_similarity: self.target_similarity,
        }
    }
}

impl ObjectDetectionParamsYaml {
    fn into_input(self) -> ObjectDetectionParamsInput {
        ObjectDetectionParamsInput {
            tile_params: self.tile_params.into_input(),
            bucket_delta: self.bucket_delta,
            trace_params: self.trace_params.into_input(),
            target_similarity: self.target_similarity,
        }
    }
}

impl ResultsModeYaml {
    fn into_results(self) -> Results {
        match self {
            ResultsModeYaml::Absolute => Results::Absolute,
            ResultsModeYaml::Relative => Results::Relative,
        }
    }
}

impl RectangleYaml {
    fn into_rectangle(self) -> Rectangle {
        Rectangle::new(
            Vec3d::new(self.top_left.x, self.top_left.y, 0.0),
            Vec3d::new(self.bottom_right.x, self.bottom_right.y, 0.0),
        )
    }
}

impl CompressedSensorImage {
    pub fn new_jpeg(header: Header, data: Vec<u8>) -> Self {
        Self {
            header,
            format: "jpeg".to_string(),
            data,
        }
    }
}

#[must_use]
pub fn default_output_topic(session_id: &str, session_type: RosSessionType) -> String {
    let session_topic_id = sanitize_topic_segment(session_id);
    match session_type {
        RosSessionType::Ordinary | RosSessionType::Eye => {
            format!("/video_sentinel/sessions/{session_topic_id}/rectangles")
        }
        RosSessionType::Object => {
            format!("/video_sentinel/sessions/{session_topic_id}/labelled_rectangles")
        }
    }
}

pub async fn run_from_cli() -> Result<(), Box<dyn Error>> {
    let args = CliArgs::parse();
    run_from_config_path(&args.config).await
}

pub async fn run_from_config_path(path: &Path) -> Result<(), Box<dyn Error>> {
    let config = Ros2NodeConfig::load(path)?;
    run(config).await
}

pub async fn run(config: Ros2NodeConfig) -> Result<(), Box<dyn Error>> {
    let mut service = Service::new();
    configure_service(&mut service, &config)?;

    let context = Context::new()?;
    let mut node = context.new_node(
        NodeName::new(&config.node.namespace, &config.node.name)?,
        NodeOptions::new(),
    )?;

    let image_topic_name = Name::parse(&config.topics.image)?;
    let image_topic = node.create_topic(
        &image_topic_name,
        MessageTypeName::new(IMAGE_TYPE_PACKAGE, IMAGE_TYPE_NAME),
        &DEFAULT_SUBSCRIPTION_QOS,
    )?;
    let image_subscription =
        node.create_subscription::<CompressedSensorImage>(&image_topic, None)?;

    let output_topic_name = Name::parse(&config.output_topic())?;
    let output_topic = node.create_topic(
        &output_topic_name,
        MessageTypeName::new(DETECTION_TYPE_PACKAGE, DETECTION_TYPE_NAME),
        &DEFAULT_PUBLISHER_QOS,
    )?;
    let output_publisher = node.create_publisher::<Detection2DArray>(&output_topic, None)?;

    println!(
        "video-sentinel ROS2 node listening on {} and publishing detections to {}",
        config.topics.image,
        config.output_topic()
    );

    tokio::spawn(node.spinner()?.spin());

    let stream = image_subscription.async_stream();
    pin_mut!(stream);

    while let Some(message_result) = stream.next().await {
        let (image, _message_info) = message_result?;
        if is_message_stale(image.header.stamp, config.node.max_message_age_ms) {
            println!(
                "dropping stale ROS2 image older than {} ms on {}",
                config.node.max_message_age_ms, config.topics.image
            );
            continue;
        }
        let header = output_header(&image.header, config.node.output_frame_id.as_deref());
        println!(
            "received ROS2 compressed image ({} bytes) on {}",
            image.data.len(),
            config.topics.image
        );
        let wrapped_image = wrapped_rgb_image_from_sensor_image(&image)?;

        let rectangles = match service
            .get_rectangles(config.session.id(), wrapped_image)
            .await
        {
            GetRectanglesResult::Success(rectangles) => rectangles,
            GetRectanglesResult::SessionNotFound => {
                return Err(format!("session {} not found", config.session.id()).into());
            }
            GetRectanglesResult::ComputationFailed(error) => return Err(error.into()),
        };

        let detections = Detection2DArray {
            header: header.clone(),
            detections: rectangles
                .into_iter()
                .enumerate()
                .map(|(index, rectangle)| {
                    detection_from_enriched_mosaic(header.clone(), index, rectangle)
                })
                .collect(),
        };
        println!(
            "publishing {} detections to {}",
            detections.detections.len(),
            config.output_topic()
        );
        output_publisher
            .async_publish(detections)
            .await
            .map_err(|error| {
                format!(
                    "failed to publish detections to {}: {error}",
                    config.output_topic()
                )
            })?;
    }

    Ok(())
}

fn output_header(input: &Header, output_frame_id: Option<&str>) -> Header {
    Header {
        stamp: input.stamp,
        frame_id: output_frame_id
            .map(ToOwned::to_owned)
            .unwrap_or_else(|| input.frame_id.clone()),
    }
}

fn configure_service(service: &mut Service, config: &Ros2NodeConfig) -> Result<(), Box<dyn Error>> {
    match &config.session {
        SessionConfig::Ordinary {
            id,
            basic_params,
            results,
        } => match service.create_ordinary_session(
            id,
            basic_params.into_input(),
            results.into_results(),
        ) {
            CreateOrdinarySessionResult::Success => Ok(()),
            CreateOrdinarySessionResult::SessionAlreadyExists => {
                Err(format!("session {id} already exists").into())
            }
        },
        SessionConfig::Eye {
            id,
            basic_params,
            results,
            eye_params,
        } => match service.create_eye_session(
            id,
            basic_params.into_input(),
            eye_params.into_input(),
            results.into_results(),
        ) {
            CreateEyeSessionResult::Success => Ok(()),
            CreateEyeSessionResult::SessionAlreadyExists => {
                Err(format!("session {id} already exists").into())
            }
        },
        SessionConfig::Object {
            id,
            basic_params,
            results,
            object_detection_params,
            reference_objects,
        } => {
            match service.create_object_session(
                id,
                basic_params.into_input(),
                object_detection_params.into_input(),
                results.into_results(),
            ) {
                CreateObjectSessionResult::Success => {}
                CreateObjectSessionResult::SessionAlreadyExists => {
                    return Err(format!("session {id} already exists").into());
                }
            }

            for reference_object in reference_objects {
                let image = WrappedRgbImage::from_file(
                    reference_object.image_path.to_str().ok_or_else(|| {
                        format!(
                            "reference image path is not valid UTF-8: {}",
                            reference_object.image_path.display()
                        )
                    })?,
                )?;
                match service.add_object_to_be_detected_as_image(
                    id,
                    &reference_object.object_id,
                    &image,
                    reference_object.surrounding_rectangle.into_rectangle(),
                ) {
                    crate::service::AddObjectToBeDetectedResult::Success => {}
                    crate::service::AddObjectToBeDetectedResult::SessionNotFound => {
                        return Err(format!("session {id} not found while loading reference object").into());
                    }
                    crate::service::AddObjectToBeDetectedResult::SessionTypeDoesNotSupportAddingObjectToBeDetected => {
                        return Err(format!("session {id} does not support reference objects").into());
                    }
                    crate::service::AddObjectToBeDetectedResult::NoReferenceMosaicsDetected => {
                        return Err(format!(
                            "reference image {} did not produce any reference mosaics inside the configured rectangle",
                            reference_object.image_path.display()
                        )
                        .into());
                    }
                }
            }

            Ok(())
        }
    }
}

fn wrapped_rgb_image_from_sensor_image(
    image: &CompressedSensorImage,
) -> Result<WrappedRgbImage, Box<dyn Error>> {
    if !image.format.eq_ignore_ascii_case("jpeg") && !image.format.eq_ignore_ascii_case("jpg") {
        return Err(format!(
            "unsupported compressed image format `{}`; expected jpeg",
            image.format
        )
        .into());
    }

    Ok(WrappedRgbImage::from_jpeg_bytes(&image.data)?)
}

fn detection_from_enriched_mosaic(
    header: Header,
    index: usize,
    rectangle: crate::service::EnrichedMosaic,
) -> Detection2D {
    Detection2D {
        header: header.clone(),
        results: rectangle
            .label
            .clone()
            .map(|label| vec![object_hypothesis_with_pose(label, rectangle.center_of_mass)])
            .unwrap_or_default(),
        bbox: bounding_box_from_rectangle(&rectangle.bounding_box),
        id: format!("detection-{index}"),
    }
}

fn object_hypothesis_with_pose(label: String, center_of_mass: Vec3d) -> ObjectHypothesisWithPose {
    ObjectHypothesisWithPose {
        hypothesis: ObjectHypothesis {
            class_id: label,
            score: 1.0,
        },
        pose: PoseWithCovariance {
            pose: Pose {
                position: Point {
                    x: center_of_mass.x,
                    y: center_of_mass.y,
                    z: center_of_mass.z,
                },
                orientation: Quaternion {
                    x: 0.0,
                    y: 0.0,
                    z: 0.0,
                    w: 1.0,
                },
            },
            covariance: [0.0; 36],
        },
    }
}

fn bounding_box_from_rectangle(rectangle: &Rectangle) -> BoundingBox2D {
    let top_left = rectangle.get_top_left();
    let bottom_right = rectangle.get_bottom_right();
    BoundingBox2D {
        center: Pose2D {
            position: Point2D {
                x: f64::midpoint(top_left.x, bottom_right.x),
                y: f64::midpoint(top_left.y, bottom_right.y),
            },
            theta: 0.0,
        },
        size_x: rectangle.get_width(),
        size_y: rectangle.get_height(),
    }
}

fn default_node_name() -> String {
    "video_sentinel_ros2".to_string()
}

fn default_node_namespace() -> String {
    "/".to_string()
}

fn default_gradient_threshold() -> u8 {
    15
}

fn default_tile_fraction() -> f64 {
    0.1
}

fn default_num_skeleton() -> usize {
    18
}

fn default_close_slice_threshold() -> f64 {
    1e-4
}

fn default_bucket_delta() -> f64 {
    0.1
}

fn default_target_similarity() -> f64 {
    0.7
}

fn default_max_message_age_ms() -> u64 {
    250
}

fn is_message_stale(stamp: Time, max_message_age_ms: u64) -> bool {
    let now_nanos = Time::now().to_nanos();
    let stamp_nanos = stamp.to_nanos();
    if stamp_nanos > now_nanos {
        return false;
    }

    let age_nanos = (now_nanos - stamp_nanos) as u128;
    age_nanos > u128::from(max_message_age_ms) * 1_000_000
}

fn sanitize_topic_segment(segment: &str) -> String {
    let mut sanitized = String::with_capacity(segment.len());
    for character in segment.chars() {
        if character.is_ascii_alphanumeric() || character == '_' {
            sanitized.push(character);
        } else {
            sanitized.push('_');
        }
    }

    if sanitized.is_empty() {
        "session".to_string()
    } else {
        sanitized
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use opencv::prelude::{MatTraitConst, VectorToVec};

    #[test]
    fn default_topics_follow_session_kind() {
        assert_eq!(
            default_output_topic("ordinary", RosSessionType::Ordinary),
            "/video_sentinel/sessions/ordinary/rectangles"
        );
        assert_eq!(
            default_output_topic("eye", RosSessionType::Eye),
            "/video_sentinel/sessions/eye/rectangles"
        );
        assert_eq!(
            default_output_topic("object", RosSessionType::Object),
            "/video_sentinel/sessions/object/labelled_rectangles"
        );
    }

    #[test]
    fn jpeg_sensor_image_converts_to_wrapped_rgb_image() {
        let source = WrappedRgbImage::new_with_color(2, 1, [10, 20, 30]);
        let rgb = source
            .get_mat()
            .try_clone()
            .expect("wrapped image should clone for JPEG encoding");
        let mut bgr = opencv::prelude::Mat::default();
        opencv::imgproc::cvt_color(&rgb, &mut bgr, opencv::imgproc::COLOR_RGB2BGR, 0)
            .expect("RGB image should convert to BGR");
        let mut encoded = opencv::core::Vector::<u8>::new();
        opencv::imgcodecs::imencode(
            ".jpg",
            &bgr,
            &mut encoded,
            &opencv::core::Vector::<i32>::new(),
        )
        .expect("image should encode as JPEG");

        let image = CompressedSensorImage::new_jpeg(
            Header {
                stamp: Time::from_nanos(42),
                frame_id: "camera".to_string(),
            },
            encoded.to_vec(),
        );

        let wrapped = wrapped_rgb_image_from_sensor_image(&image)
            .expect("JPEG image should convert into WrappedRgbImage");
        assert_eq!(wrapped.width(), 2);
        assert_eq!(wrapped.height(), 1);
        let pixel = wrapped.get_pixel(0, 0);
        assert!((i16::from(pixel[0]) - 10).abs() <= 5);
        assert!((i16::from(pixel[1]) - 20).abs() <= 5);
        assert!((i16::from(pixel[2]) - 30).abs() <= 5);
    }

    #[test]
    fn stale_message_check_uses_small_threshold() {
        let now = Time::now().to_nanos();
        assert!(!is_message_stale(Time::from_nanos(now), 250));
        assert!(!is_message_stale(Time::from_nanos(now - 249_000_000), 250));
        assert!(is_message_stale(Time::from_nanos(now - 251_000_000), 250));
        assert!(!is_message_stale(Time::from_nanos(now + 10_000_000), 250));
    }

    #[test]
    fn config_load_resolves_reference_object_paths_relative_to_yaml() {
        let temp_dir = tempfile::tempdir().expect("tempdir should be created");
        let image_path = temp_dir.path().join("reference.jpg");
        fs::write(&image_path, b"fake").expect("reference file should be written");
        let config_path = temp_dir.path().join("session.yaml");
        fs::write(
            &config_path,
            r#"
topics:
  image: /camera/image
session:
  type: object
  id: object-session
  reference_objects:
    - object_id: ref
      image_path: reference.jpg
      surrounding_rectangle:
        top_left: { x: 0.0, y: 0.0 }
        bottom_right: { x: 10.0, y: 10.0 }
"#,
        )
        .expect("config should be written");

        let config = Ros2NodeConfig::load(&config_path).expect("config should load");
        match config.session {
            SessionConfig::Object {
                reference_objects, ..
            } => {
                assert_eq!(reference_objects.len(), 1);
                assert_eq!(reference_objects[0].image_path, image_path);
            }
            _ => panic!("expected object session"),
        }
    }
}

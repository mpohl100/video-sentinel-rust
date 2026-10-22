#![warn(clippy::pedantic)]
#![allow(
    clippy::missing_errors_doc,
    clippy::missing_panics_doc,
    clippy::too_many_lines,
    clippy::unused_async_trait_impl
)]

use crate::service::{
    CreateEyeSessionResult, CreateObjectSessionResult, CreateOrdinarySessionResult,
    GetRectanglesResult, Service,
};
use crate::slices::{Rectangle, WrappedRgbImage};
use clap::Parser;
use futures::{StreamExt, pin_mut};
use ros2_client::{
    Context, DEFAULT_PUBLISHER_QOS, DEFAULT_SUBSCRIPTION_QOS, MessageTypeName, Name, NodeName,
    NodeOptions, builtin_interfaces::Time,
};
use rs_math3d::Vec3d;
use std::error::Error;
use std::path::Path;

use super::{
    BoundingBox2D, CliArgs, CompressedSensorImage, Detection2D, Detection2DArray, Header,
    ObjectHypothesis, ObjectHypothesisWithPose, Point, Point2D, Pose, Pose2D, PoseWithCovariance,
    Quaternion, ReferenceObjectYaml, Ros2NodeConfig, RosSessionType, SessionConfig,
};

pub(super) const IMAGE_TYPE_PACKAGE: &str = "sensor_msgs";

pub(super) const IMAGE_TYPE_NAME: &str = "CompressedImage";

pub(super) const DETECTION_TYPE_PACKAGE: &str = "vision_msgs";

pub(super) const DETECTION_TYPE_NAME: &str = "Detection2DArray";

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
        NodeName::new(config.node().namespace(), config.node().name())?,
        NodeOptions::new(),
    )?;

    let image_topic_name = Name::parse(config.topics().image())?;
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
        config.topics().image(),
        config.output_topic()
    );

    tokio::spawn(node.spinner()?.spin());

    let stream = image_subscription.async_stream();
    pin_mut!(stream);

    while let Some(message_result) = stream.next().await {
        let (image, _message_info) = message_result?;
        if is_message_stale(image.header.stamp, config.node().max_message_age_ms()) {
            println!(
                "dropping stale ROS2 image older than {} ms on {}",
                config.node().max_message_age_ms(),
                config.topics().image()
            );
            continue;
        }
        let header = output_header(
            &image.header,
            config.node().output_frame_id().map(String::as_str),
        );
        println!(
            "received ROS2 compressed image ({} bytes) on {}",
            image.data.len(),
            config.topics().image()
        );
        let wrapped_image = wrapped_rgb_image_from_sensor_image(&image)?;

        let rectangles = match service
            .get_rectangles(config.session().id(), wrapped_image)
            .await
        {
            GetRectanglesResult::Success(rectangles) => rectangles,
            GetRectanglesResult::SessionNotFound => {
                return Err(format!("session {} not found", config.session().id()).into());
            }
            GetRectanglesResult::ComputationFailed(error) => return Err(error.into()),
        };

        let detections = Detection2DArray {
            header: header.clone(),
            detections: rectangles
                .iter()
                .enumerate()
                .map(|(index, rectangle)| detection_from_enriched_mosaic(&header, index, rectangle))
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

pub(super) fn output_header(input: &Header, output_frame_id: Option<&str>) -> Header {
    Header {
        stamp: input.stamp,
        frame_id: output_frame_id.map_or_else(|| input.frame_id.clone(), ToOwned::to_owned),
    }
}

pub(super) fn configure_service(
    service: &mut Service,
    config: &Ros2NodeConfig,
) -> Result<(), Box<dyn Error>> {
    match &config.session() {
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
                match reference_object {
                    ReferenceObjectYaml::ImagePath {
                        object_id,
                        image_path,
                        surrounding_rectangle,
                    } => {
                        let image = WrappedRgbImage::from_file(image_path.to_str().ok_or_else(
                            || {
                                format!(
                                    "reference image path is not valid UTF-8: {}",
                                    image_path.display()
                                )
                            },
                        )?)?;
                        match service.add_object_to_be_detected_as_image(
                            id,
                            object_id,
                            &image,
                            surrounding_rectangle.into_rectangle(),
                        ) {
                            crate::service::AddObjectToBeDetectedResult::Success => {}
                            crate::service::AddObjectToBeDetectedResult::SessionNotFound => {
                                return Err(
                                    format!(
                                        "session {id} not found while loading reference object"
                                    )
                                    .into(),
                                );
                            }
                            crate::service::AddObjectToBeDetectedResult::SessionTypeDoesNotSupportAddingObjectToBeDetected => {
                                return Err(format!("session {id} does not support reference objects").into());
                            }
                            crate::service::AddObjectToBeDetectedResult::NoReferenceMosaicsDetected => {
                                return Err(format!(
                                    "reference image {} did not produce any reference mosaics inside the configured rectangle",
                                    image_path.display()
                                )
                                .into());
                            }
                        }
                    }
                    ReferenceObjectYaml::AsciiArt {
                        object_id,
                        ascii_art,
                    } => match service.add_object_to_be_detected_as_ascii_art(
                        id,
                        object_id,
                        ascii_art,
                    ) {
                        crate::service::AddObjectToBeDetectedResult::Success => {}
                        crate::service::AddObjectToBeDetectedResult::SessionNotFound => {
                            return Err(
                                format!("session {id} not found while loading reference object")
                                    .into(),
                            );
                        }
                        crate::service::AddObjectToBeDetectedResult::SessionTypeDoesNotSupportAddingObjectToBeDetected => {
                            return Err(format!("session {id} does not support reference objects").into());
                        }
                        crate::service::AddObjectToBeDetectedResult::NoReferenceMosaicsDetected => {
                            return Err(format!(
                                "ASCII-art reference object `{object_id}` did not produce any reference mosaics"
                            )
                            .into());
                        }
                    },
                }
            }

            Ok(())
        }
    }
}

pub(super) fn wrapped_rgb_image_from_sensor_image(
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

pub(super) fn detection_from_enriched_mosaic(
    header: &Header,
    index: usize,
    rectangle: &crate::service::EnrichedMosaic,
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

pub(super) fn object_hypothesis_with_pose(
    label: String,
    center_of_mass: Vec3d,
) -> ObjectHypothesisWithPose {
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

pub(super) fn bounding_box_from_rectangle(rectangle: &Rectangle) -> BoundingBox2D {
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

pub(super) fn default_node_name() -> String {
    "video_sentinel_ros2".to_string()
}

pub(super) fn default_node_namespace() -> String {
    "/".to_string()
}

pub(super) fn default_gradient_threshold() -> u8 {
    15
}

pub(super) fn default_do_grayscale() -> bool {
    true
}

pub(super) fn default_tile_fraction() -> f64 {
    0.1
}

pub(super) fn default_num_skeleton() -> usize {
    18
}

pub(super) fn default_bucket_delta() -> f64 {
    0.05
}

pub(super) fn default_target_similarity() -> f64 {
    0.7
}

pub(super) fn default_max_message_age_ms() -> u64 {
    250
}

pub(super) fn is_message_stale(stamp: Time, max_message_age_ms: u64) -> bool {
    let now_nanos = Time::now().to_nanos();
    let stamp_nanos = stamp.to_nanos();
    if stamp_nanos > now_nanos {
        return false;
    }

    let age_nanos = u128::try_from(now_nanos - stamp_nanos).expect("age should be non-negative");
    age_nanos > u128::from(max_message_age_ms) * 1_000_000
}

pub(super) fn sanitize_topic_segment(segment: &str) -> String {
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

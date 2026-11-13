#![warn(clippy::pedantic)]
#![allow(
    clippy::missing_errors_doc,
    clippy::missing_panics_doc,
    clippy::too_many_lines,
    clippy::unused_async_trait_impl
)]

use crate::service::{
    AddColorFilterResult, CreateEyeSessionResult, CreateObjectSessionResult,
    CreateOrdinarySessionResult, GetRectanglesCalculation, GetRectanglesRequest,
    GetRectanglesResult, GetTrianglesCalculation, ProcessThreeDTriangleFrameResult, Service,
    ThreeDTriangleCalibration, ThreeDTriangleFrame,
};
use crate::slices::{Rectangle, WrappedRgbImage};
use clap::Parser;
use futures::{StreamExt, pin_mut, stream::FuturesUnordered};
use ros2_client::{
    Context, DEFAULT_PUBLISHER_QOS, DEFAULT_SUBSCRIPTION_QOS, MessageTypeName, Name, NodeName,
    NodeOptions, QosProfile,
    builtin_interfaces::Time,
    qos::{Durability, History, WhenFull},
};
use rs_math3d::Vec3d;
use std::collections::VecDeque;
use std::error::Error;
use std::future::Future;
use std::path::Path;
use std::pin::Pin;
use std::sync::{Arc, Mutex};

use super::{
    BoundingBox2D, CameraInfo, CliArgs, ColorRgba, CompressedSensorImage, Detection2D,
    Detection2DArray, DiagnosticArray, DiagnosticKeyValue, DiagnosticStatus, Header, Image, Marker,
    MatchedFrame, MeshFile, ObjectHypothesis, ObjectHypothesisWithPose, Point, Point2D, Pose,
    Pose2D, PoseWithCovariance, Quaternion, ReferenceObjectYaml, Ros2NodeConfig, RosSessionType,
    SessionConfig, TFMessage, TfBuffer, ThreeDFrameSynchronizer, Vector3,
};

pub(super) const IMAGE_TYPE_PACKAGE: &str = "sensor_msgs";

pub(super) const IMAGE_TYPE_NAME: &str = "CompressedImage";

pub(super) const DETECTION_TYPE_PACKAGE: &str = "vision_msgs";

pub(super) const DETECTION_TYPE_NAME: &str = "Detection2DArray";

pub const DIAGNOSTICS_TOPIC: &str = "/diagnostics";

const IMAGE_MESSAGE_PACKAGE: &str = "sensor_msgs";
const IMAGE_MESSAGE_NAME: &str = "Image";
const CAMERA_INFO_MESSAGE_PACKAGE: &str = "sensor_msgs";
const CAMERA_INFO_MESSAGE_NAME: &str = "CameraInfo";
const TF_MESSAGE_PACKAGE: &str = "tf2_msgs";
const TF_MESSAGE_NAME: &str = "TFMessage";
const MARKER_MESSAGE_PACKAGE: &str = "visualization_msgs";
const MARKER_MESSAGE_NAME: &str = "Marker";

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
        RosSessionType::ThreeDTriangle => {
            format!("/video_sentinel/sessions/{session_topic_id}/triangles")
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
    if matches!(config.session(), SessionConfig::ThreeDTriangle { .. }) {
        return run_three_d_triangle(config, service).await;
    }

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
    let output_publisher = std::sync::Arc::new(tokio::sync::Mutex::new(
        node.create_publisher::<Detection2DArray>(&output_topic, None)?,
    ));
    let diagnostics_topic = node.create_topic(
        &Name::parse(DIAGNOSTICS_TOPIC)?,
        MessageTypeName::new("diagnostic_msgs", "DiagnosticArray"),
        &DEFAULT_PUBLISHER_QOS,
    )?;
    let diagnostics_publisher =
        node.create_publisher::<DiagnosticArray>(&diagnostics_topic, None)?;

    println!(
        "video-sentinel ROS2 node listening on {} and publishing detections to {}",
        config.topics().image(),
        config.output_topic()
    );

    tokio::spawn(node.spinner()?.spin());

    let stream = image_subscription.async_stream();
    pin_mut!(stream);

    let mut pending = FuturesUnordered::new();
    let mut stream_ended = false;
    let mut heartbeat = tokio::time::interval(std::time::Duration::from_secs(1));
    heartbeat.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
    loop {
        if stream_ended && pending.is_empty() {
            break;
        }
        tokio::select! {
            _ = heartbeat.tick(), if !stream_ended => {
                let namespace = config.node().namespace().trim_end_matches('/');
                let diagnostic = DiagnosticArray {
                    header: Header { stamp: Time::now(), frame_id: String::new() },
                    status: vec![DiagnosticStatus {
                        level: DiagnosticStatus::OK,
                        name: format!("{namespace}/{}", config.node().name()),
                        message: "Ready".to_owned(),
                        hardware_id: String::new(),
                        values: vec![DiagnosticKeyValue {
                            key: "session_id".to_owned(),
                            value: config.session().id().to_owned(),
                        }],
                    }],
                };
                diagnostics_publisher.async_publish(diagnostic).await
                    .map_err(|error| format!("failed to publish diagnostics: {error}"))?;
            }
            message_result = stream.next(), if !stream_ended => {
                let Some(message_result) = message_result else {
                    stream_ended = true;
                    continue;
                };
                let (image, _message_info) = message_result?;
                if is_message_stale(image.header.stamp, config.node().max_message_age_ms()) {
                    println!(
                        "dropping stale ROS2 image older than {} ms on {}",
                        config.node().max_message_age_ms(),
                        config.topics().image()
                    );
                    continue;
                }
                println!(
                    "received ROS2 compressed image ({} bytes, frame {}) on {}",
                    image.data.len(),
                    image.header.frame_id,
                    config.topics().image()
                );
                let Some(request) = service.get_rectangles_request(config.session().id()) else {
                    return Err(format!("session {} not found", config.session().id()).into());
                };
                let Some((header, calculation)) = try_start_frame(
                    request,
                    image,
                    config.node().output_frame_id().map(String::as_str),
                ).await
                else {
                    continue;
                };
                let publisher = std::sync::Arc::clone(&output_publisher);
                let output_topic = config.output_topic();
                pending.push(async move {
                    let rectangles = match calculation.result().await {
                        GetRectanglesResult::Success(rectangles) => rectangles,
                        GetRectanglesResult::SessionNotFound => {
                            return Err("session not found".to_owned());
                        }
                        GetRectanglesResult::ComputationFailed(error) => return Err(error),
                    };

                    let detections_header = header.clone();
                    let detections = tokio::task::spawn_blocking(move || Detection2DArray {
                        header: detections_header.clone(),
                        detections: rectangles
                            .iter()
                            .enumerate()
                            .map(|(index, rectangle)| {
                                detection_from_enriched_mosaic(
                                    &detections_header,
                                    index,
                                    rectangle,
                                )
                            })
                            .collect(),
                    })
                    .await
                    .map_err(|error| {
                        format!("failed to construct detections for {output_topic}: {error}")
                    })?;
                    println!(
                        "publishing {} detections to {}",
                        detections.detections.len(),
                        output_topic
                    );
                    publisher
                        .lock()
                        .await
                        .async_publish(detections)
                        .await
                        .map_err(|error| {
                            format!(
                                "failed to publish detections to {output_topic}: {error}"
                            )
                        })
                });
            }
            Some(result) = pending.next(), if !pending.is_empty() => {
                result.map_err(|error| -> Box<dyn Error> { error.into() })?;
            }
        }
    }

    Ok(())
}

async fn try_start_frame(
    request: GetRectanglesRequest,
    image: CompressedSensorImage,
    output_frame_id: Option<&str>,
) -> Option<(Header, GetRectanglesCalculation)> {
    let header = output_header(&image.header, output_frame_id);
    let calculation = request
        .try_start_with(move || {
            wrapped_rgb_image_from_sensor_image(&image).map_err(|error| error.to_string())
        })
        .await?;
    Some((header, calculation))
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
            color_filters,
        } => {
            match service.create_ordinary_session(
                id,
                basic_params.into_input(),
                results.into_results(),
            ) {
                CreateOrdinarySessionResult::Success => {}
                CreateOrdinarySessionResult::SessionAlreadyExists => {
                    return Err(format!("session {id} already exists").into());
                }
            }
            for filter in color_filters {
                match service.add_color_filter(
                    id,
                    filter.color.into(),
                    filter.threshold_for_color.into(),
                ) {
                    AddColorFilterResult::Success => {}
                    AddColorFilterResult::SessionNotFound => {
                        return Err(
                            format!("session {id} not found while adding color filter").into()
                        );
                    }
                    AddColorFilterResult::SessionTypeDoesNotSupportColorFilters => {
                        return Err(format!("session {id} does not support color filters").into());
                    }
                }
            }
            Ok(())
        }
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
        SessionConfig::ThreeDTriangle { id, .. } => {
            match service.create_three_d_triangle_session(id, None) {
                crate::service::CreateThreeDTriangleSessionResult::Success => Ok(()),
                crate::service::CreateThreeDTriangleSessionResult::SessionAlreadyExists => {
                    Err(format!("session {id} already exists").into())
                }
                crate::service::CreateThreeDTriangleSessionResult::InvalidInput(error) => {
                    Err(format!("invalid 3D triangle session calibration: {error}").into())
                }
            }
        }
    }
}

async fn run_three_d_triangle(
    config: Ros2NodeConfig,
    service: Service,
) -> Result<(), Box<dyn Error>> {
    let node = config.node();
    let topics = config.topics();
    let (frame_max_time_difference_ms, max_pending_messages) = match config.session() {
        SessionConfig::ThreeDTriangle {
            frame_max_time_difference_ms,
            max_pending_messages,
            ..
        } => (*frame_max_time_difference_ms, *max_pending_messages),
        _ => return Err("3D triangle runner requires a 3D triangle session".into()),
    };

    let context = Context::new()?;
    let mut node_handle = context.new_node(
        NodeName::new(node.namespace(), node.name())?,
        NodeOptions::new(),
    )?;
    let rgb_subscription = create_subscription::<Image>(
        &mut node_handle,
        topics.image(),
        IMAGE_MESSAGE_PACKAGE,
        IMAGE_MESSAGE_NAME,
        &DEFAULT_SUBSCRIPTION_QOS,
    )?;
    let depth_subscription = create_subscription::<Image>(
        &mut node_handle,
        topics.depth().ok_or("missing depth topic")?,
        IMAGE_MESSAGE_PACKAGE,
        IMAGE_MESSAGE_NAME,
        &DEFAULT_SUBSCRIPTION_QOS,
    )?;
    let rgb_info_subscription = create_subscription::<CameraInfo>(
        &mut node_handle,
        topics
            .rgb_camera_info()
            .ok_or("missing RGB CameraInfo topic")?,
        CAMERA_INFO_MESSAGE_PACKAGE,
        CAMERA_INFO_MESSAGE_NAME,
        &DEFAULT_SUBSCRIPTION_QOS,
    )?;
    let depth_info_subscription = create_subscription::<CameraInfo>(
        &mut node_handle,
        topics
            .depth_camera_info()
            .ok_or("missing depth CameraInfo topic")?,
        CAMERA_INFO_MESSAGE_PACKAGE,
        CAMERA_INFO_MESSAGE_NAME,
        &DEFAULT_SUBSCRIPTION_QOS,
    )?;
    let static_tf_qos = tf_static_subscription_qos();
    let tf_subscription = create_subscription::<TFMessage>(
        &mut node_handle,
        topics.tf(),
        TF_MESSAGE_PACKAGE,
        TF_MESSAGE_NAME,
        &DEFAULT_SUBSCRIPTION_QOS,
    )?;
    let tf_static_subscription = create_subscription::<TFMessage>(
        &mut node_handle,
        topics.tf_static(),
        TF_MESSAGE_PACKAGE,
        TF_MESSAGE_NAME,
        &static_tf_qos,
    )?;

    let output_topic = config.output_topic();
    let output_topic_name = Name::parse(&output_topic)?;
    let marker_topic = node_handle.create_topic(
        &output_topic_name,
        MessageTypeName::new(MARKER_MESSAGE_PACKAGE, MARKER_MESSAGE_NAME),
        &DEFAULT_PUBLISHER_QOS,
    )?;
    let publisher = node_handle.create_publisher::<Marker>(&marker_topic, None)?;
    let diagnostics_topic = node_handle.create_topic(
        &Name::parse(DIAGNOSTICS_TOPIC)?,
        MessageTypeName::new("diagnostic_msgs", "DiagnosticArray"),
        &DEFAULT_PUBLISHER_QOS,
    )?;
    let diagnostics_publisher =
        node_handle.create_publisher::<DiagnosticArray>(&diagnostics_topic, None)?;

    tokio::spawn(node_handle.spinner()?.spin());

    let frame_busy_state = Arc::new(Mutex::new(false));
    let (sender, mut receiver) =
        tokio::sync::mpsc::channel(max_pending_messages.max(1).saturating_mul(5));
    tokio::spawn(relay_subscription(
        rgb_subscription,
        sender.clone(),
        Arc::clone(&frame_busy_state),
        true,
        |message| ThreeDEvent::Rgb(Ok(message)),
        |error| ThreeDEvent::Rgb(Err(error)),
    ));
    tokio::spawn(relay_subscription(
        depth_subscription,
        sender.clone(),
        Arc::clone(&frame_busy_state),
        true,
        |message| ThreeDEvent::Depth(Ok(message)),
        |error| ThreeDEvent::Depth(Err(error)),
    ));
    tokio::spawn(relay_subscription(
        rgb_info_subscription,
        sender.clone(),
        Arc::clone(&frame_busy_state),
        false,
        |message| ThreeDEvent::RgbInfo(Ok(message)),
        |error| ThreeDEvent::RgbInfo(Err(error)),
    ));
    tokio::spawn(relay_subscription(
        depth_info_subscription,
        sender.clone(),
        Arc::clone(&frame_busy_state),
        false,
        |message| ThreeDEvent::DepthInfo(Ok(message)),
        |error| ThreeDEvent::DepthInfo(Err(error)),
    ));
    tokio::spawn(relay_subscription(
        tf_subscription,
        sender.clone(),
        Arc::clone(&frame_busy_state),
        false,
        |message| ThreeDEvent::Tf(Ok(message)),
        |error| ThreeDEvent::Tf(Err(error)),
    ));
    tokio::spawn(relay_subscription(
        tf_static_subscription,
        sender,
        Arc::clone(&frame_busy_state),
        false,
        |message| ThreeDEvent::TfStatic(Ok(message)),
        |error| ThreeDEvent::TfStatic(Err(error)),
    ));

    println!(
        "video-sentinel ROS2 3D triangle session listening on RGB {}, depth {}, publishing {}",
        topics.image(),
        topics.depth().unwrap_or_default(),
        output_topic
    );
    let mut synchronizer = ThreeDFrameSynchronizer::new(max_pending_messages);
    let mut pending_frames = VecDeque::new();
    let mut tf_buffer = TfBuffer::default();
    let max_difference_ns = i64::try_from(frame_max_time_difference_ms)
        .unwrap_or(i64::MAX / 1_000_000)
        .saturating_mul(1_000_000);
    let mut heartbeat = tokio::time::interval(std::time::Duration::from_secs(1));
    heartbeat.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
    let tf_history_limit = max_pending_messages.saturating_mul(20).max(100);
    let mut active_calculation: Option<TriangleResultFuture> = None;
    loop {
        match next_three_d_runner_event(&mut receiver, &mut heartbeat, &mut active_calculation)
            .await
        {
            ThreeDRunnerEvent::Heartbeat => {
                let namespace = node.namespace().trim_end_matches('/');
                diagnostics_publisher
                    .async_publish(DiagnosticArray {
                        header: Header {
                            stamp: Time::now(),
                            frame_id: String::new(),
                        },
                        status: vec![DiagnosticStatus {
                            level: DiagnosticStatus::OK,
                            name: format!("{namespace}/{}", node.name()),
                            message: "Ready".to_owned(),
                            hardware_id: String::new(),
                            values: vec![DiagnosticKeyValue {
                                key: "session_id".to_owned(),
                                value: config.session().id().to_owned(),
                            }],
                        }],
                    })
                    .await
                    .map_err(|error| format!("failed to publish diagnostics: {error}"))?;
                continue;
            }
            ThreeDRunnerEvent::InputClosed => break,
            ThreeDRunnerEvent::Calculation(result) => {
                set_frame_processing_busy(&frame_busy_state, false);
                active_calculation = None;
                let (header, result) = result;
                if let Some(marker) = marker_from_triangle_result(header, result) {
                    publisher.async_publish(marker).await.map_err(|error| {
                        format!("failed to publish Marker to {output_topic}: {error}")
                    })?;
                }
            }
            ThreeDRunnerEvent::Input(event) => {
                let event = *event;
                if !should_admit_three_d_event(&event, active_calculation.is_some()) {
                    continue;
                }
                match event {
                    ThreeDEvent::Rgb(Ok(message)) => synchronizer.push_rgb(message),
                    ThreeDEvent::Depth(Ok(message)) => synchronizer.push_depth(message),
                    ThreeDEvent::RgbInfo(Ok(message)) => synchronizer.push_rgb_info(message),
                    ThreeDEvent::DepthInfo(Ok(message)) => synchronizer.push_depth_info(message),
                    ThreeDEvent::Tf(Ok(message)) => {
                        for transform in &message.transforms {
                            tf_buffer.insert(transform, false, tf_history_limit);
                        }
                    }
                    ThreeDEvent::TfStatic(Ok(message)) => {
                        for transform in &message.transforms {
                            tf_buffer.insert(transform, true, tf_history_limit);
                        }
                    }
                    event => {
                        eprintln!("ROS2 3D input stream error: {}", event.error_message());
                        continue;
                    }
                }
            }
        }

        while let Some(frame) = synchronizer.pop_nearest(max_difference_ns) {
            if active_calculation.is_some() {
                continue;
            }
            if pending_frames.len() >= max_pending_messages {
                pending_frames.pop_front();
            }
            pending_frames.push_back(frame);
        }
        if active_calculation.is_some() {
            pending_frames.clear();
            synchronizer.discard_image_messages();
            continue;
        }

        let mut frame_index = 0;
        while frame_index < pending_frames.len() {
            let frame = &pending_frames[frame_index];
            let transform_is_available = frame.depth.header.frame_id == frame.rgb.header.frame_id
                || tf_buffer
                    .lookup(
                        &frame.depth.header.frame_id,
                        &frame.rgb.header.frame_id,
                        frame.depth.header.stamp,
                    )
                    .is_some();
            if !transform_is_available {
                frame_index += 1;
                continue;
            }
            let frame = pending_frames
                .remove(frame_index)
                .expect("pending frame index was checked");
            if let Some(calculation) = start_three_d_frame(
                &service,
                config.session().id(),
                frame,
                &tf_buffer,
                config.node().max_message_age_ms(),
                &frame_busy_state,
            )
            .await
            {
                let (header, calculation) = calculation;
                active_calculation = Some(Box::pin(
                    async move { (header, calculation.result().await) },
                ));
                pending_frames.clear();
                synchronizer.discard_image_messages();
                break;
            }
        }
    }
    Ok(())
}

fn create_subscription<T: 'static>(
    node: &mut ros2_client::Node,
    topic: &str,
    package: &str,
    name: &str,
    qos: &QosProfile,
) -> Result<ros2_client::Subscription<T>, Box<dyn Error>> {
    let topic_name = Name::parse(topic)?;
    let topic = node.create_topic(&topic_name, MessageTypeName::new(package, name), qos)?;
    Ok(node.create_subscription::<T>(&topic, Some(qos.clone()))?)
}

enum ThreeDEvent {
    Rgb(Result<Image, String>),
    Depth(Result<Image, String>),
    RgbInfo(Result<CameraInfo, String>),
    DepthInfo(Result<CameraInfo, String>),
    Tf(Result<TFMessage, String>),
    TfStatic(Result<TFMessage, String>),
}

type TriangleResultFuture =
    Pin<Box<dyn Future<Output = (Header, ProcessThreeDTriangleFrameResult)> + Send>>;

enum ThreeDRunnerEvent {
    Heartbeat,
    Input(Box<ThreeDEvent>),
    Calculation((Header, ProcessThreeDTriangleFrameResult)),
    InputClosed,
}

async fn next_three_d_runner_event(
    receiver: &mut tokio::sync::mpsc::Receiver<ThreeDEvent>,
    heartbeat: &mut tokio::time::Interval,
    active_calculation: &mut Option<TriangleResultFuture>,
) -> ThreeDRunnerEvent {
    tokio::select! {
        _ = heartbeat.tick() => ThreeDRunnerEvent::Heartbeat,
        event = receiver.recv() => event.map_or(ThreeDRunnerEvent::InputClosed, |event| {
            ThreeDRunnerEvent::Input(Box::new(event))
        }),
        result = async {
            match active_calculation.as_mut() {
                Some(calculation) => calculation.as_mut().await,
                None => std::future::pending().await,
            }
        } => ThreeDRunnerEvent::Calculation(result),
    }
}

impl ThreeDEvent {
    fn is_rgb_or_depth_message(&self) -> bool {
        matches!(self, Self::Rgb(Ok(_)) | Self::Depth(Ok(_)))
    }

    fn error_message(&self) -> &str {
        match self {
            Self::Rgb(Err(error))
            | Self::Depth(Err(error))
            | Self::RgbInfo(Err(error))
            | Self::DepthInfo(Err(error))
            | Self::Tf(Err(error))
            | Self::TfStatic(Err(error)) => error,
            _ => "unexpected successful ROS2 message",
        }
    }
}

fn should_admit_three_d_event(event: &ThreeDEvent, calculation_is_active: bool) -> bool {
    !calculation_is_active || !event.is_rgb_or_depth_message()
}

async fn relay_subscription<T>(
    subscription: ros2_client::Subscription<T>,
    sender: tokio::sync::mpsc::Sender<ThreeDEvent>,
    frame_busy_state: Arc<Mutex<bool>>,
    drop_while_busy: bool,
    map_message: fn(T) -> ThreeDEvent,
    map_error: fn(String) -> ThreeDEvent,
) where
    T: Send + serde::de::DeserializeOwned + 'static,
{
    let stream = subscription.async_stream();
    pin_mut!(stream);
    while let Some(message) = stream.next().await {
        if drop_while_busy && !frame_is_admitted_at_receipt(&frame_busy_state) {
            continue;
        }
        let event = match message {
            Ok((message, _)) => map_message(message),
            Err(error) => map_error(error.to_string()),
        };
        if sender.send(event).await.is_err() {
            break;
        }
    }
}

fn frame_is_admitted_at_receipt(frame_busy_state: &Mutex<bool>) -> bool {
    !*frame_busy_state.lock().unwrap()
}

fn set_frame_processing_busy(frame_busy_state: &Mutex<bool>, busy: bool) {
    *frame_busy_state.lock().unwrap() = busy;
}

fn tf_static_subscription_qos() -> QosProfile {
    QosProfile::subscription_default()
        .reliability_reliable(WhenFull::DEFAULT)
        .durability(Durability::TransientLocal)
        .history(History::KeepLast { depth: 100 })
}

async fn start_three_d_frame(
    service: &Service,
    session_id: &str,
    frame: MatchedFrame,
    tf_buffer: &TfBuffer,
    max_message_age_ms: u64,
    frame_busy_state: &Mutex<bool>,
) -> Option<(Header, GetTrianglesCalculation)> {
    if is_message_stale(frame.rgb.header.stamp, max_message_age_ms)
        || is_message_stale(frame.depth.header.stamp, max_message_age_ms)
    {
        return None;
    }
    let depth_to_rgb = tf_buffer.lookup(
        &frame.depth.header.frame_id,
        &frame.rgb.header.frame_id,
        frame.depth.header.stamp,
    );
    let Some(depth_to_rgb) = depth_to_rgb else {
        eprintln!(
            "dropping RGB/depth pair: no TF transform from {} to {} at depth timestamp",
            frame.depth.header.frame_id, frame.rgb.header.frame_id
        );
        return None;
    };
    let depth_camera = match frame.depth_camera_info.camera_intrinsics() {
        Ok(camera) => camera,
        Err(error) => {
            eprintln!("dropping depth frame with invalid CameraInfo: {error}");
            return None;
        }
    };
    let rgb_camera = match frame.rgb_camera_info.camera_intrinsics() {
        Ok(camera) => camera,
        Err(error) => {
            eprintln!("dropping RGB frame with invalid CameraInfo: {error}");
            return None;
        }
    };
    let Some(request) = service.get_triangles_request(
        session_id,
        Some(ThreeDTriangleCalibration {
            depth_camera,
            rgb_camera,
            depth_to_rgb,
        }),
    ) else {
        eprintln!("3D triangle session {session_id} not found");
        return None;
    };
    let header = frame.rgb.header.clone();
    set_frame_processing_busy(frame_busy_state, true);
    let calculation = request
        .try_start_with(move || {
            let rgb = wrapped_rgb_image_from_ros_image(&frame.rgb)?;
            let depth = depth_frame_from_ros_image(&frame.depth)?;
            Ok(ThreeDTriangleFrame { rgb, depth })
        })
        .await;
    if calculation.is_none() {
        set_frame_processing_busy(frame_busy_state, false);
    }
    calculation.map(|calculation| (header, calculation))
}

fn marker_from_triangle_result(
    header: Header,
    result: ProcessThreeDTriangleFrameResult,
) -> Option<Marker> {
    match result {
        crate::service::ProcessThreeDTriangleFrameResult::Success(triangles) => {
            let mut points = Vec::with_capacity(triangles.len() * 3);
            for triangle in triangles {
                for vertex in triangle.vertices {
                    points.push(Point {
                        x: vertex[0],
                        y: vertex[1],
                        z: vertex[2],
                    });
                }
            }
            Some(Marker {
                header,
                ns: "video_sentinel".to_owned(),
                id: 0,
                marker_type: Marker::TRIANGLE_LIST,
                action: Marker::ADD,
                pose: Pose {
                    position: Point {
                        x: 0.0,
                        y: 0.0,
                        z: 0.0,
                    },
                    orientation: Quaternion {
                        x: 0.0,
                        y: 0.0,
                        z: 0.0,
                        w: 1.0,
                    },
                },
                scale: Vector3 {
                    x: 1.0,
                    y: 1.0,
                    z: 1.0,
                },
                color: ColorRgba {
                    r: 1.0,
                    g: 1.0,
                    b: 1.0,
                    a: 1.0,
                },
                lifetime: ros2_client::builtin_interfaces::Duration::zero(),
                frame_locked: false,
                points,
                colors: Vec::new(),
                texture_resource: String::new(),
                texture: CompressedSensorImage {
                    header: Header {
                        stamp: ros2_client::builtin_interfaces::Time::from_nanos(0),
                        frame_id: String::new(),
                    },
                    format: String::new(),
                    data: Vec::new(),
                },
                uv_coordinates: Vec::new(),
                text: String::new(),
                mesh_resource: String::new(),
                mesh_file: MeshFile::default(),
                mesh_use_embedded_materials: false,
            })
        }
        crate::service::ProcessThreeDTriangleFrameResult::SessionNotFound => None,
        crate::service::ProcessThreeDTriangleFrameResult::InvalidInput(error) => {
            eprintln!("dropping invalid 3D frame: {error}");
            None
        }
    }
}

fn validate_rgb_image(image: &Image) -> Result<(), String> {
    let channels = match image.encoding.as_str() {
        "rgb8" | "bgr8" => 3,
        "rgba8" | "bgra8" => 4,
        encoding => return Err(format!("unsupported RGB image encoding `{encoding}`")),
    };
    validate_image_buffer(image, channels)
}

fn wrapped_rgb_image_from_ros_image(image: &Image) -> Result<WrappedRgbImage, String> {
    validate_rgb_image(image)?;
    let channels = match image.encoding.as_str() {
        "rgb8" | "bgr8" => 3_usize,
        "rgba8" | "bgra8" => 4_usize,
        _ => unreachable!("RGB encoding was validated"),
    };
    let width = usize::try_from(image.width).map_err(|_| "RGB width overflow".to_owned())?;
    let height = usize::try_from(image.height).map_err(|_| "RGB height overflow".to_owned())?;
    let step = usize::try_from(image.step).map_err(|_| "RGB row step overflow".to_owned())?;
    let output_len = width
        .checked_mul(height)
        .and_then(|pixels| pixels.checked_mul(3))
        .ok_or_else(|| "RGB output size overflow".to_owned())?;
    let mut rgb = Vec::with_capacity(output_len);
    for y in 0..height {
        let row_start = y * step;
        for x in 0..width {
            let pixel_start = row_start + x * channels;
            let pixel = &image.data[pixel_start..pixel_start + channels];
            match image.encoding.as_str() {
                "rgb8" | "rgba8" => rgb.extend_from_slice(&pixel[..3]),
                "bgr8" | "bgra8" => rgb.extend_from_slice(&[pixel[2], pixel[1], pixel[0]]),
                _ => unreachable!("RGB encoding was validated"),
            }
        }
    }
    WrappedRgbImage::from_rgb_data(image.width, image.height, &rgb)
        .map_err(|error| format!("failed to construct RGB image: {error}"))
}

fn validate_image_buffer(image: &Image, bytes_per_pixel: u32) -> Result<(), String> {
    if image.width == 0 || image.height == 0 {
        return Err("image dimensions must be non-zero".to_owned());
    }
    let pixel_count = usize::try_from(image.width)
        .ok()
        .and_then(|width| {
            usize::try_from(image.height)
                .ok()
                .and_then(|height| width.checked_mul(height))
        })
        .ok_or_else(|| "image dimensions overflow".to_owned())?;
    if pixel_count > 64_000_000 {
        return Err("image exceeds the 64-million-pixel safety limit".to_owned());
    }
    let row_bytes = image
        .width
        .checked_mul(bytes_per_pixel)
        .ok_or_else(|| "image row length overflow".to_owned())?;
    if image.step < row_bytes {
        return Err(format!(
            "image step {} is shorter than row length {row_bytes}",
            image.step
        ));
    }
    let expected_len = usize::try_from(image.step)
        .ok()
        .and_then(|step| {
            usize::try_from(image.height)
                .ok()
                .and_then(|height| step.checked_mul(height))
        })
        .ok_or_else(|| "image data length overflow".to_owned())?;
    if image.data.len() < expected_len {
        return Err(format!(
            "image data has {} bytes, expected at least {expected_len}",
            image.data.len()
        ));
    }
    Ok(())
}

fn depth_frame_from_ros_image(image: &Image) -> Result<crate::service::DepthFrame, String> {
    let encoding = image.encoding.to_ascii_uppercase();
    let bytes_per_pixel = match encoding.as_str() {
        "16UC1" => 2,
        "32FC1" => 4,
        _ => {
            return Err(format!(
                "unsupported depth image encoding `{}`",
                image.encoding
            ));
        }
    };
    validate_image_buffer(image, bytes_per_pixel)?;
    let row_bytes = usize::try_from(image.width)
        .ok()
        .and_then(|width| width.checked_mul(bytes_per_pixel as usize))
        .ok_or_else(|| "depth row length overflow".to_owned())?;
    let step = usize::try_from(image.step).map_err(|_| "depth step overflow".to_owned())?;
    let bigendian = image.is_bigendian != 0;
    let mut metres = Vec::with_capacity(
        usize::try_from(image.width)
            .ok()
            .and_then(|width| {
                usize::try_from(image.height)
                    .ok()
                    .and_then(|height| width.checked_mul(height))
            })
            .ok_or_else(|| "depth dimensions overflow".to_owned())?,
    );
    for row in 0..image.height as usize {
        let start = row * step;
        let bytes = &image.data[start..start + row_bytes];
        match encoding.as_str() {
            "16UC1" => {
                for sample in bytes.as_chunks::<2>().0 {
                    let value = if bigendian {
                        u16::from_be_bytes([sample[0], sample[1]])
                    } else {
                        u16::from_le_bytes([sample[0], sample[1]])
                    };
                    metres.push(if value == 0 {
                        f32::NAN
                    } else {
                        f32::from(value) / 1000.0
                    });
                }
            }
            "32FC1" => {
                for sample in bytes.as_chunks::<4>().0 {
                    let sample = [sample[0], sample[1], sample[2], sample[3]];
                    metres.push(f32::from_bits(if bigendian {
                        u32::from_be_bytes(sample)
                    } else {
                        u32::from_le_bytes(sample)
                    }));
                }
            }
            _ => unreachable!(),
        }
    }
    Ok(crate::service::DepthFrame {
        width: image.width,
        height: image.height,
        metres,
    })
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mosaics::Results;
    use crate::service::BasicParamsInput;
    use std::sync::{Arc, Condvar, Mutex};

    fn image(frame_id: &str) -> CompressedSensorImage {
        CompressedSensorImage::new_jpeg(
            Header {
                stamp: Time::now(),
                frame_id: frame_id.to_owned(),
            },
            vec![0],
        )
    }

    #[test]
    fn ordinary_yaml_configures_color_filters_and_rejects_out_of_range_channels() {
        let yaml = r"
topics:
  image: /camera/image
session:
  type: ordinary
  id: colors
  color_filters:
    - color: { red: 255, green: 0, blue: 0 }
      threshold_for_color: { red: 10, green: 20, blue: 30 }
    - color: { red: 0, green: 255, blue: 0 }
      threshold_for_color: { red: 1, green: 2, blue: 3 }
";
        let config: Ros2NodeConfig = serde_yaml::from_str(yaml).unwrap();
        let mut service = Service::new();
        configure_service(&mut service, &config).unwrap();
        let filters = service
            .get_ordinary_session("colors")
            .unwrap()
            .color_filters;
        assert_eq!(filters.len(), 2);
        assert_eq!(filters[0].color.red, 255);
        assert_eq!(filters[0].threshold_for_color.green, 20);
        assert_eq!(filters[1].color.green, 255);
        assert_eq!(filters[1].threshold_for_color.blue, 3);
        assert!(
            serde_yaml::from_str::<Ros2NodeConfig>(&yaml.replace("red: 255", "red: 256")).is_err()
        );
    }

    #[tokio::test]
    async fn busy_frames_are_discarded_and_later_frame_is_admitted() {
        let mut service = Service::new();
        service.create_ordinary_session(
            "ordinary",
            BasicParamsInput {
                do_grayscale: false,
                gradient_threshold: 15,
            },
            Results::Absolute,
        );
        let (started_sender, started_receiver) = tokio::sync::oneshot::channel();
        let release = Arc::new((Mutex::new(false), Condvar::new()));
        let worker_release = Arc::clone(&release);
        let first = service
            .get_rectangles_request("ordinary")
            .unwrap()
            .try_start_with(move || {
                let _ = started_sender.send(());
                let (lock, condvar) = &*worker_release;
                let mut released = lock.lock().unwrap();
                while !*released {
                    released = condvar.wait(released).unwrap();
                }
                Ok(WrappedRgbImage::new_with_color(64, 64, [255, 255, 255]))
            })
            .await
            .unwrap();
        started_receiver.await.unwrap();

        let dropped = try_start_frame(
            service.get_rectangles_request("ordinary").unwrap(),
            image("dropped"),
            None,
        )
        .await;
        {
            let (lock, condvar) = &*release;
            *lock.lock().unwrap() = true;
            condvar.notify_all();
        }
        assert!(dropped.is_none(), "busy frames must not be published");
        assert!(matches!(
            first.result().await,
            GetRectanglesResult::Success(_)
        ));

        let (header, next) = try_start_frame(
            service.get_rectangles_request("ordinary").unwrap(),
            image("next"),
            None,
        )
        .await
        .expect("frame after completion should be admitted");
        assert_eq!(header.frame_id, "next");
        assert!(matches!(
            next.result().await,
            GetRectanglesResult::ComputationFailed(_)
        ));
    }

    #[tokio::test]
    async fn three_d_runner_services_heartbeats_and_drops_images_while_processing() {
        let (sender, mut receiver) = tokio::sync::mpsc::channel(2);
        let mut heartbeat = tokio::time::interval(std::time::Duration::from_millis(5));
        heartbeat.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
        let processing_header = Header {
            stamp: Time::now(),
            frame_id: "rgb_optical".to_owned(),
        };
        let mut active_calculation: Option<TriangleResultFuture> = Some(Box::pin(async move {
            tokio::time::sleep(std::time::Duration::from_millis(60)).await;
            (
                processing_header,
                ProcessThreeDTriangleFrameResult::Success(Vec::new()),
            )
        }));

        assert!(matches!(
            next_three_d_runner_event(&mut receiver, &mut heartbeat, &mut active_calculation).await,
            ThreeDRunnerEvent::Heartbeat
        ));
        sender
            .send(ThreeDEvent::Rgb(Ok(Image {
                header: Header {
                    stamp: Time::now(),
                    frame_id: "rgb_optical".to_owned(),
                },
                height: 1,
                width: 1,
                encoding: "rgb8".to_owned(),
                is_bigendian: 0,
                step: 3,
                data: vec![1, 2, 3],
            })))
            .await
            .expect("busy RGB frame should reach the event loop");

        let mut input_was_dropped = false;
        let mut heartbeat_count = 1;
        loop {
            match next_three_d_runner_event(&mut receiver, &mut heartbeat, &mut active_calculation)
                .await
            {
                ThreeDRunnerEvent::Heartbeat => heartbeat_count += 1,
                ThreeDRunnerEvent::Input(event) => {
                    assert!(!should_admit_three_d_event(
                        &event,
                        active_calculation.is_some()
                    ));
                    input_was_dropped = true;
                }
                ThreeDRunnerEvent::Calculation((_, result)) => {
                    assert_eq!(
                        result,
                        ProcessThreeDTriangleFrameResult::Success(Vec::new())
                    );
                    break;
                }
                ThreeDRunnerEvent::InputClosed => panic!("event sender is still alive"),
            }
        }
        assert!(input_was_dropped);
        assert!(heartbeat_count >= 2);
    }

    #[test]
    fn busy_receipt_is_dropped_before_queue_and_next_frame_is_admitted() {
        let gate = Mutex::new(true);
        let (sender, mut receiver) = tokio::sync::mpsc::channel(2);

        if frame_is_admitted_at_receipt(&gate) {
            sender.try_send("arrived-while-busy").unwrap();
        }
        assert!(receiver.try_recv().is_err());

        set_frame_processing_busy(&gate, false);
        if frame_is_admitted_at_receipt(&gate) {
            sender.try_send("arrived-after-completion").unwrap();
        }
        assert_eq!(
            receiver
                .try_recv()
                .expect("next frame should remain queued"),
            "arrived-after-completion"
        );
    }

    #[test]
    fn static_tf_subscription_matches_reliable_transient_local_publishers() {
        let qos = tf_static_subscription_qos();
        assert!(matches!(
            qos.reliability,
            ros2_client::qos::Reliability::Reliable { .. }
        ));
        assert_eq!(qos.durability, Durability::TransientLocal);
    }

    #[test]
    fn marker_conversion_preserves_rgb_header_and_triangle_geometry() {
        let header = Header {
            stamp: Time::from_nanos(42),
            frame_id: "rgb_optical".to_owned(),
        };
        let marker = marker_from_triangle_result(
            header.clone(),
            ProcessThreeDTriangleFrameResult::Success(vec![crate::service::Triangle {
                vertices: [[1.0, 2.0, 3.0], [4.0, 5.0, 6.0], [7.0, 8.0, 9.0]],
            }]),
        )
        .expect("successful triangle result should produce a Marker");
        assert_eq!(marker.marker_type, Marker::TRIANGLE_LIST);
        assert_eq!(marker.action, Marker::ADD);
        assert_eq!(marker.header, header);
        assert_eq!(
            marker.points,
            vec![
                Point {
                    x: 1.0,
                    y: 2.0,
                    z: 3.0,
                },
                Point {
                    x: 4.0,
                    y: 5.0,
                    z: 6.0,
                },
                Point {
                    x: 7.0,
                    y: 8.0,
                    z: 9.0,
                },
            ]
        );
    }

    #[test]
    fn converts_ros_rgb_encodings_and_row_padding_to_pixel_data() {
        let image = Image {
            header: Header {
                stamp: Time::from_nanos(1_000_000_000),
                frame_id: "rgb_optical".to_owned(),
            },
            height: 1,
            width: 2,
            encoding: "bgr8".to_owned(),
            is_bigendian: 0,
            step: 8,
            data: vec![0, 0, 255, 0, 255, 0, 99, 99],
        };
        let rgb = wrapped_rgb_image_from_ros_image(&image)
            .expect("padded BGR message should produce an RGB image");
        assert_eq!(rgb.get_pixel(0, 0), [255, 0, 0]);
        assert_eq!(rgb.get_pixel(1, 0), [0, 255, 0]);
    }

    #[test]
    fn decodes_ros_depth_endianness_row_stride_and_rep118_units() {
        let depth = Image {
            header: Header {
                stamp: Time::from_nanos(1_000_000_000),
                frame_id: "depth_optical".to_owned(),
            },
            height: 2,
            width: 2,
            encoding: "16UC1".to_owned(),
            is_bigendian: 1,
            step: 6,
            data: vec![0x03, 0xe8, 0, 0, 0xff, 0xff, 0, 0, 0, 0, 0xff, 0xff],
        };
        let decoded = depth_frame_from_ros_image(&depth).expect("ROS depth message should decode");
        assert_eq!(decoded.metres[0], 1.0);
        assert!(decoded.metres[1].is_nan());
        assert!(decoded.metres[2].is_nan());
        assert!(decoded.metres[3].is_nan());
    }

    #[test]
    fn rejects_invalid_ros_image_layout_and_encoding() {
        let image = Image {
            header: Header {
                stamp: Time::from_nanos(1_000_000_000),
                frame_id: "depth_optical".to_owned(),
            },
            height: 1,
            width: 2,
            encoding: "16UC1".to_owned(),
            is_bigendian: 0,
            step: 3,
            data: vec![0; 3],
        };
        assert!(
            depth_frame_from_ros_image(&image)
                .expect_err("undersized row stride should fail")
                .contains("step")
        );
    }
}

#![warn(clippy::pedantic)]
#![allow(
    clippy::missing_errors_doc,
    clippy::missing_panics_doc,
    clippy::too_many_lines,
    clippy::unused_async_trait_impl
)]

use ros2_client::Name;
use serde::Deserialize;
use std::error::Error;
use std::fs;
use std::path::Path;

use super::{
    NodeConfig, ReferenceObjectYaml, ResultsModeYaml, SessionConfig, TopicsConfig,
    default_output_topic,
};

#[derive(Debug, Clone, Deserialize)]
pub struct Ros2NodeConfig {
    #[serde(default)]
    node: NodeConfig,
    topics: TopicsConfig,
    session: SessionConfig,
}

impl Ros2NodeConfig {
    pub(super) fn node(&self) -> &NodeConfig {
        &self.node
    }

    pub(super) fn topics(&self) -> &TopicsConfig {
        &self.topics
    }

    pub(super) fn session(&self) -> &SessionConfig {
        &self.session
    }
}

impl Ros2NodeConfig {
    pub fn load(path: &Path) -> Result<Self, Box<dyn Error>> {
        let contents = fs::read_to_string(path)?;
        let mut config: Self = serde_yaml::from_str(&contents)?;
        config.resolve_paths(path);
        config.validate()?;
        Ok(config)
    }

    fn resolve_paths(&mut self, path: &Path) {
        let Some(base_dir) = path.parent() else {
            return;
        };
        if let SessionConfig::Object {
            reference_objects, ..
        } = &mut self.session
        {
            for reference_object in reference_objects {
                if let ReferenceObjectYaml::ImagePath { image_path, .. } = reference_object
                    && image_path.is_relative()
                {
                    *image_path = base_dir.join(&*image_path);
                }
            }
        }
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
                    match reference_object {
                        ReferenceObjectYaml::ImagePath { image_path, .. } => {
                            if !image_path.exists() {
                                return Err(format!(
                                    "reference image `{}` does not exist",
                                    image_path.display()
                                )
                                .into());
                            }
                        }
                        ReferenceObjectYaml::AsciiArt {
                            object_id,
                            ascii_art,
                        } => {
                            if ascii_art.trim().is_empty() {
                                return Err(format!(
                                    "ASCII-art reference object `{object_id}` must not be empty"
                                )
                                .into());
                            }
                        }
                    }
                }
            }
            SessionConfig::ThreeDTriangle {
                frame_max_time_difference_ms: _,
                max_pending_messages,
                ..
            } => {
                if *max_pending_messages == 0 {
                    return Err("3D triangle max_pending_messages must be positive".into());
                }
                if *max_pending_messages > 1_000 {
                    return Err("3D triangle max_pending_messages must not exceed 1000".into());
                }
                for (description, topic) in [
                    ("depth", self.topics.depth()),
                    ("rgb_camera_info", self.topics.rgb_camera_info()),
                    ("depth_camera_info", self.topics.depth_camera_info()),
                ] {
                    let topic = topic.ok_or_else(|| {
                        format!("3D triangle session requires topics.{description}")
                    })?;
                    Name::parse(topic).map_err(|error| {
                        format!("invalid {description} topic `{topic}`: {error}")
                    })?;
                }
                Name::parse(self.topics.tf())
                    .map_err(|error| format!("invalid TF topic: {error}"))?;
                Name::parse(self.topics.tf_static())
                    .map_err(|error| format!("invalid static TF topic: {error}"))?;
            }
            SessionConfig::Ordinary { .. } | SessionConfig::Eye { .. } => {}
        }

        Name::parse(self.topics.image())
            .map_err(|error| format!("invalid input topic `{}`: {error}", self.topics.image()))?;
        Name::parse(&self.output_topic())
            .map_err(|error| format!("invalid output topic `{}`: {error}", self.output_topic()))?;

        Ok(())
    }

    #[must_use]
    pub fn output_topic(&self) -> String {
        self.topics
            .output()
            .cloned()
            .unwrap_or_else(|| default_output_topic(self.session.id(), self.session.kind()))
    }
}

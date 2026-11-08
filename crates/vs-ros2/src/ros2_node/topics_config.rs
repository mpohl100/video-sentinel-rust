#![warn(clippy::pedantic)]
#![allow(
    clippy::missing_errors_doc,
    clippy::missing_panics_doc,
    clippy::too_many_lines,
    clippy::unused_async_trait_impl
)]

use serde::Deserialize;

fn default_tf_topic() -> String {
    "/tf".to_owned()
}

fn default_tf_static_topic() -> String {
    "/tf_static".to_owned()
}

#[derive(Debug, Clone, Deserialize)]
pub(super) struct TopicsConfig {
    image: String,
    #[serde(default)]
    output: Option<String>,
    #[serde(default)]
    depth: Option<String>,
    #[serde(default)]
    rgb_camera_info: Option<String>,
    #[serde(default)]
    depth_camera_info: Option<String>,
    #[serde(default = "default_tf_topic")]
    tf: String,
    #[serde(default = "default_tf_static_topic")]
    tf_static: String,
}

impl TopicsConfig {
    pub(super) fn image(&self) -> &String {
        &self.image
    }

    pub(super) fn output(&self) -> Option<&String> {
        self.output.as_ref()
    }

    pub(super) fn depth(&self) -> Option<&str> {
        self.depth.as_deref()
    }

    pub(super) fn rgb_camera_info(&self) -> Option<&str> {
        self.rgb_camera_info.as_deref()
    }

    pub(super) fn depth_camera_info(&self) -> Option<&str> {
        self.depth_camera_info.as_deref()
    }

    pub(super) fn tf(&self) -> &str {
        &self.tf
    }

    pub(super) fn tf_static(&self) -> &str {
        &self.tf_static
    }
}

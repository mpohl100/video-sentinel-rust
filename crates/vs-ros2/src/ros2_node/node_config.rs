#![warn(clippy::pedantic)]
#![allow(
    clippy::missing_errors_doc,
    clippy::missing_panics_doc,
    clippy::too_many_lines,
    clippy::unused_async_trait_impl
)]

use serde::Deserialize;

use super::{default_max_message_age_ms, default_node_name, default_node_namespace};

#[derive(Debug, Clone, Deserialize)]
pub(super) struct NodeConfig {
    #[serde(default = "default_node_name")]
    name: String,
    #[serde(default = "default_node_namespace")]
    namespace: String,
    #[serde(default)]
    output_frame_id: Option<String>,
    #[serde(default = "default_max_message_age_ms")]
    max_message_age_ms: u64,
}

impl NodeConfig {
    pub(super) fn name(&self) -> &String {
        &self.name
    }

    pub(super) fn namespace(&self) -> &String {
        &self.namespace
    }

    pub(super) fn output_frame_id(&self) -> Option<&String> {
        self.output_frame_id.as_ref()
    }

    pub(super) fn max_message_age_ms(&self) -> u64 {
        self.max_message_age_ms
    }
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

#![warn(clippy::pedantic)]
#![allow(
    clippy::missing_errors_doc,
    clippy::missing_panics_doc,
    clippy::too_many_lines,
    clippy::unused_async_trait_impl
)]

use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub(super) struct TopicsConfig {
    image: String,
    #[serde(default)]
    output: Option<String>,
}

impl TopicsConfig {
    pub(super) fn image(&self) -> &String {
        &self.image
    }

    pub(super) fn output(&self) -> Option<&String> {
        self.output.as_ref()
    }
}

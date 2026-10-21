#![warn(clippy::pedantic)]
#![allow(
    clippy::missing_errors_doc,
    clippy::missing_panics_doc,
    clippy::too_many_lines,
    clippy::unused_async_trait_impl
)]

use crate::service::BasicParamsInput;
use serde::Deserialize;

use super::default_gradient_threshold;

#[derive(Debug, Clone, Copy, Deserialize)]
pub(super) struct BasicParamsYaml {
    #[serde(default)]
    do_grayscale: bool,
    #[serde(default = "default_gradient_threshold")]
    gradient_threshold: u8,
}

impl Default for BasicParamsYaml {
    fn default() -> Self {
        Self {
            do_grayscale: false,
            gradient_threshold: default_gradient_threshold(),
        }
    }
}

impl BasicParamsYaml {
    pub(super) fn into_input(self) -> BasicParamsInput {
        BasicParamsInput {
            do_grayscale: self.do_grayscale,
            gradient_threshold: self.gradient_threshold,
        }
    }
}

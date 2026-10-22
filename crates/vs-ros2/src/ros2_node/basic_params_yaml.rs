#![warn(clippy::pedantic)]
#![allow(
    clippy::missing_errors_doc,
    clippy::missing_panics_doc,
    clippy::too_many_lines,
    clippy::unused_async_trait_impl
)]

use crate::service::BasicParamsInput;
use serde::Deserialize;

use super::{default_do_grayscale, default_gradient_threshold};

#[derive(Debug, Clone, Copy, Deserialize)]
pub(super) struct BasicParamsYaml {
    #[serde(default = "default_do_grayscale")]
    do_grayscale: bool,
    #[serde(default = "default_gradient_threshold")]
    gradient_threshold: u8,
}

impl Default for BasicParamsYaml {
    fn default() -> Self {
        Self {
            do_grayscale: default_do_grayscale(),
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

#[cfg(test)]
mod tests {
    use super::BasicParamsYaml;

    #[test]
    fn omitted_basic_params_match_video_cli_defaults() {
        let params = serde_yaml::from_str::<BasicParamsYaml>("{}")
            .expect("default basic params should deserialize");

        assert!(params.do_grayscale);
        assert_eq!(params.gradient_threshold, 15);
    }
}

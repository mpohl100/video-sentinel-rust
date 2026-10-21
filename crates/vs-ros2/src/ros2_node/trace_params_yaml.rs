#![warn(clippy::pedantic)]
#![allow(
    clippy::missing_errors_doc,
    clippy::missing_panics_doc,
    clippy::too_many_lines,
    clippy::unused_async_trait_impl
)]

use crate::service::TraceParamsInput;
use serde::Deserialize;

use super::{default_close_slice_threshold, default_num_skeleton};

#[derive(Debug, Clone, Copy, Deserialize)]
pub(super) struct TraceParamsYaml {
    #[serde(default = "default_num_skeleton")]
    num_skeleton: usize,
    #[serde(default = "default_close_slice_threshold")]
    close_slice_threshold: f64,
}

impl Default for TraceParamsYaml {
    fn default() -> Self {
        Self {
            num_skeleton: default_num_skeleton(),
            close_slice_threshold: default_close_slice_threshold(),
        }
    }
}

impl TraceParamsYaml {
    pub(super) fn into_input(self) -> TraceParamsInput {
        TraceParamsInput {
            num_skeleton: self.num_skeleton,
            close_slice_threshold: self.close_slice_threshold,
        }
    }
}

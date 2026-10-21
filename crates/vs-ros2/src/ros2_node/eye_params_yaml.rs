#![warn(clippy::pedantic)]
#![allow(
    clippy::missing_errors_doc,
    clippy::missing_panics_doc,
    clippy::too_many_lines,
    clippy::unused_async_trait_impl
)]

use crate::service::EyeParamsInput;
use serde::Deserialize;

use super::{TileParamsYaml, TraceParamsYaml, default_bucket_delta, default_target_similarity};

#[derive(Debug, Clone, Copy, Deserialize)]
pub(super) struct EyeParamsYaml {
    #[serde(default)]
    tile_params: TileParamsYaml,
    #[serde(default = "default_bucket_delta")]
    bucket_delta: f64,
    #[serde(default)]
    trace_params: TraceParamsYaml,
    #[serde(default = "default_target_similarity")]
    target_similarity: f64,
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

impl EyeParamsYaml {
    pub(super) fn into_input(self) -> EyeParamsInput {
        EyeParamsInput {
            tile_params: self.tile_params.into_input(),
            bucket_delta: self.bucket_delta,
            trace_params: self.trace_params.into_input(),
            target_similarity: self.target_similarity,
        }
    }
}

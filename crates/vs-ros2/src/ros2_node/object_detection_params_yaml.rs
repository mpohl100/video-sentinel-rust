#![warn(clippy::pedantic)]
#![allow(
    clippy::missing_errors_doc,
    clippy::missing_panics_doc,
    clippy::too_many_lines,
    clippy::unused_async_trait_impl
)]

use crate::service::ObjectDetectionParamsInput;
use serde::Deserialize;

use super::{TileParamsYaml, TraceParamsYaml, default_bucket_delta, default_target_similarity};

#[derive(Debug, Clone, Copy, Deserialize)]
pub(super) struct ObjectDetectionParamsYaml {
    #[serde(default)]
    tile_params: TileParamsYaml,
    #[serde(default = "default_bucket_delta")]
    bucket_delta: f64,
    #[serde(default)]
    trace_params: TraceParamsYaml,
    #[serde(default = "default_target_similarity")]
    target_similarity: f64,
}

impl Default for ObjectDetectionParamsYaml {
    fn default() -> Self {
        Self {
            tile_params: TileParamsYaml::default(),
            bucket_delta: default_bucket_delta(),
            trace_params: TraceParamsYaml::default(),
            target_similarity: default_target_similarity(),
        }
    }
}

impl ObjectDetectionParamsYaml {
    pub(super) fn into_input(self) -> ObjectDetectionParamsInput {
        ObjectDetectionParamsInput {
            tile_params: self.tile_params.into_input(),
            bucket_delta: self.bucket_delta,
            trace_params: self.trace_params.into_input(),
            target_similarity: self.target_similarity,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::ObjectDetectionParamsYaml;

    #[test]
    fn omitted_bucket_delta_uses_video_cli_default() {
        let params = serde_yaml::from_str::<ObjectDetectionParamsYaml>("{}")
            .expect("default object-detection params should deserialize");

        assert!((params.bucket_delta - 0.05).abs() < f64::EPSILON);
    }
}

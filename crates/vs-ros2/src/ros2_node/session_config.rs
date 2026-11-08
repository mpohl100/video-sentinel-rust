#![warn(clippy::pedantic)]
#![allow(
    clippy::missing_errors_doc,
    clippy::missing_panics_doc,
    clippy::too_many_lines,
    clippy::unused_async_trait_impl
)]

use serde::Deserialize;

use super::{
    BasicParamsYaml, EyeParamsYaml, ObjectDetectionParamsYaml, ReferenceObjectYaml,
    ResultsModeYaml, RosSessionType,
};

fn default_frame_max_time_difference_ms() -> u64 {
    30
}

fn default_max_pending_messages() -> usize {
    10
}

#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub(super) enum SessionConfig {
    Ordinary {
        id: String,
        #[serde(default)]
        basic_params: BasicParamsYaml,
        #[serde(default)]
        results: ResultsModeYaml,
    },
    Eye {
        id: String,
        #[serde(default)]
        basic_params: BasicParamsYaml,
        #[serde(default)]
        results: ResultsModeYaml,
        #[serde(default)]
        eye_params: EyeParamsYaml,
    },
    Object {
        id: String,
        #[serde(default)]
        basic_params: BasicParamsYaml,
        #[serde(default)]
        results: ResultsModeYaml,
        #[serde(default)]
        object_detection_params: ObjectDetectionParamsYaml,
        #[serde(default)]
        reference_objects: Vec<ReferenceObjectYaml>,
    },
    ThreeDTriangle {
        id: String,
        #[serde(
            default = "default_frame_max_time_difference_ms",
            alias = "max_timestamp_difference_ms"
        )]
        frame_max_time_difference_ms: u64,
        #[serde(default = "default_max_pending_messages")]
        max_pending_messages: usize,
    },
}

impl SessionConfig {
    #[must_use]
    pub fn id(&self) -> &str {
        match self {
            SessionConfig::Ordinary { id, .. }
            | SessionConfig::Eye { id, .. }
            | SessionConfig::Object { id, .. }
            | SessionConfig::ThreeDTriangle { id, .. } => id,
        }
    }

    #[must_use]
    pub fn kind(&self) -> RosSessionType {
        match self {
            SessionConfig::Ordinary { .. } => RosSessionType::Ordinary,
            SessionConfig::Eye { .. } => RosSessionType::Eye,
            SessionConfig::Object { .. } => RosSessionType::Object,
            SessionConfig::ThreeDTriangle { .. } => RosSessionType::ThreeDTriangle,
        }
    }

    #[must_use]
    pub(super) fn results(&self) -> ResultsModeYaml {
        match self {
            SessionConfig::Ordinary { results, .. }
            | SessionConfig::Eye { results, .. }
            | SessionConfig::Object { results, .. } => *results,
            SessionConfig::ThreeDTriangle { .. } => ResultsModeYaml::Absolute,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn frame_time_tolerance_allows_exact_match_and_accepts_legacy_key() {
        let exact: SessionConfig = serde_yaml::from_str(
            "type: three_d_triangle\nid: exact\nframe_max_time_difference_ms: 0\n",
        )
        .expect("zero frame-time tolerance is valid");
        assert!(matches!(
            exact,
            SessionConfig::ThreeDTriangle {
                frame_max_time_difference_ms: 0,
                ..
            }
        ));

        let legacy: SessionConfig = serde_yaml::from_str(
            "type: three_d_triangle\nid: legacy\nmax_timestamp_difference_ms: 7\n",
        )
        .expect("legacy key should remain readable");
        assert!(matches!(
            legacy,
            SessionConfig::ThreeDTriangle {
                frame_max_time_difference_ms: 7,
                ..
            }
        ));
    }
}

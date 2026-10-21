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
}

impl SessionConfig {
    #[must_use]
    pub fn id(&self) -> &str {
        match self {
            SessionConfig::Ordinary { id, .. }
            | SessionConfig::Eye { id, .. }
            | SessionConfig::Object { id, .. } => id,
        }
    }

    #[must_use]
    pub fn kind(&self) -> RosSessionType {
        match self {
            SessionConfig::Ordinary { .. } => RosSessionType::Ordinary,
            SessionConfig::Eye { .. } => RosSessionType::Eye,
            SessionConfig::Object { .. } => RosSessionType::Object,
        }
    }

    #[must_use]
    pub(super) fn results(&self) -> ResultsModeYaml {
        match self {
            SessionConfig::Ordinary { results, .. }
            | SessionConfig::Eye { results, .. }
            | SessionConfig::Object { results, .. } => *results,
        }
    }
}

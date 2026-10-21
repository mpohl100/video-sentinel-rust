#![warn(clippy::pedantic)]
#![allow(
    clippy::missing_errors_doc,
    clippy::missing_panics_doc,
    clippy::too_many_lines,
    clippy::unused_async_trait_impl
)]

use crate::service::ObjectSession;

use super::{BasicParamsGql, ObjectDetectionParamsGql, ReferenceObjectGql, ResultsGql};

#[derive(async_graphql::SimpleObject, Clone)]
pub struct ObjectSessionGql {
    basic_params: BasicParamsGql,
    object_detection_params: ObjectDetectionParamsGql,
    objects_to_be_detected: Vec<ReferenceObjectGql>,
    results: ResultsGql,
}

impl From<ObjectSession> for ObjectSessionGql {
    fn from(value: ObjectSession) -> Self {
        Self {
            basic_params: value.basic_params.into(),
            object_detection_params: value.object_detection_params.into(),
            objects_to_be_detected: value
                .objects_to_be_detected
                .into_iter()
                .map(Into::into)
                .collect(),
            results: value.results.into(),
        }
    }
}

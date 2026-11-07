#![warn(clippy::pedantic)]
#![allow(
    clippy::missing_errors_doc,
    clippy::missing_panics_doc,
    clippy::too_many_lines,
    clippy::unused_async_trait_impl
)]

use super::get_rectangles_request::SessionCalculation;
use crate::mosaics::Results;
use crate::object_detection::ObjectDetectionParams;
use crate::object_detection::ReferenceObject;
use crate::slices::BasicParams;
use std::sync::Arc;

#[derive(Clone)]
pub struct ObjectSession {
    pub basic_params: BasicParams,
    pub object_detection_params: ObjectDetectionParams,
    pub objects_to_be_detected: Vec<ReferenceObject>,
    pub results: Results,
    pub(super) calculation: Arc<SessionCalculation>,
}

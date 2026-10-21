#![warn(clippy::pedantic)]
#![allow(
    clippy::missing_errors_doc,
    clippy::missing_panics_doc,
    clippy::too_many_lines,
    clippy::unused_async_trait_impl
)]

use serde::{Deserialize, Serialize};

use super::{BoundingBox2D, Header, ObjectHypothesisWithPose};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Detection2D {
    pub header: Header,
    pub results: Vec<ObjectHypothesisWithPose>,
    pub bbox: BoundingBox2D,
    pub id: String,
}

#![warn(clippy::pedantic)]
#![allow(
    clippy::missing_errors_doc,
    clippy::missing_panics_doc,
    clippy::too_many_lines,
    clippy::unused_async_trait_impl
)]

use serde::{Deserialize, Serialize};

use super::Pose2D;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct BoundingBox2D {
    pub center: Pose2D,
    pub size_x: f64,
    pub size_y: f64,
}

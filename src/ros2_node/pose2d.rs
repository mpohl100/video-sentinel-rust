#![warn(clippy::pedantic)]
#![allow(
    clippy::missing_errors_doc,
    clippy::missing_panics_doc,
    clippy::too_many_lines,
    clippy::unused_async_trait_impl
)]

use serde::{Deserialize, Serialize};

use super::Point2D;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Pose2D {
    pub position: Point2D,
    pub theta: f64,
}

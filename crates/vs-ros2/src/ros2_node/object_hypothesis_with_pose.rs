#![warn(clippy::pedantic)]
#![allow(
    clippy::missing_errors_doc,
    clippy::missing_panics_doc,
    clippy::too_many_lines,
    clippy::unused_async_trait_impl
)]

use serde::{Deserialize, Serialize};

use super::{ObjectHypothesis, PoseWithCovariance};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct ObjectHypothesisWithPose {
    pub hypothesis: ObjectHypothesis,
    pub pose: PoseWithCovariance,
}

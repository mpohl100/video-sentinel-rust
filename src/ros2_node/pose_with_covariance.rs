#![warn(clippy::pedantic)]
#![allow(
    clippy::missing_errors_doc,
    clippy::missing_panics_doc,
    clippy::too_many_lines,
    clippy::unused_async_trait_impl
)]

use serde::{Deserialize, Serialize};

use super::Pose;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct PoseWithCovariance {
    pub pose: Pose,
    #[serde(with = "serde_big_array::BigArray")]
    pub covariance: [f64; 36],
}

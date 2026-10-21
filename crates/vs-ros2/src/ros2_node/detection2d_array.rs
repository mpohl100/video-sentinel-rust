#![warn(clippy::pedantic)]
#![allow(
    clippy::missing_errors_doc,
    clippy::missing_panics_doc,
    clippy::too_many_lines,
    clippy::unused_async_trait_impl
)]

use serde::{Deserialize, Serialize};

use super::{Detection2D, Header};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Detection2DArray {
    pub header: Header,
    pub detections: Vec<Detection2D>,
}

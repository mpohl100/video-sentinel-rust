#![warn(clippy::pedantic)]
#![allow(
    clippy::missing_errors_doc,
    clippy::missing_panics_doc,
    clippy::too_many_lines,
    clippy::unused_async_trait_impl
)]

use serde::{Deserialize, Serialize};

use super::Header;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct CompressedSensorImage {
    pub header: Header,
    pub format: String,
    pub data: Vec<u8>,
}

impl CompressedSensorImage {
    #[must_use]
    pub fn new_jpeg(header: Header, data: Vec<u8>) -> Self {
        Self {
            header,
            format: "jpeg".to_string(),
            data,
        }
    }
}

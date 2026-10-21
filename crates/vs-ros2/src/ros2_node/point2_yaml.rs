#![warn(clippy::pedantic)]
#![allow(
    clippy::missing_errors_doc,
    clippy::missing_panics_doc,
    clippy::too_many_lines,
    clippy::unused_async_trait_impl
)]

use serde::Deserialize;

#[derive(Debug, Clone, Copy, Deserialize)]
pub(super) struct Point2Yaml {
    x: f64,
    y: f64,
}

impl Point2Yaml {
    pub(super) fn x(&self) -> f64 {
        self.x
    }

    pub(super) fn y(&self) -> f64 {
        self.y
    }
}

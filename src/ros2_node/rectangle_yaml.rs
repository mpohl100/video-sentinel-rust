#![warn(clippy::pedantic)]
#![allow(
    clippy::missing_errors_doc,
    clippy::missing_panics_doc,
    clippy::too_many_lines,
    clippy::unused_async_trait_impl
)]

use crate::slices::Rectangle;
use rs_math3d::Vec3d;
use serde::Deserialize;

use super::Point2Yaml;

#[derive(Debug, Clone, Copy, Deserialize)]
pub(super) struct RectangleYaml {
    top_left: Point2Yaml,
    bottom_right: Point2Yaml,
}

impl RectangleYaml {
    pub(super) fn into_rectangle(self) -> Rectangle {
        Rectangle::new(
            Vec3d::new(self.top_left.x(), self.top_left.y(), 0.0),
            Vec3d::new(self.bottom_right.x(), self.bottom_right.y(), 0.0),
        )
    }
}

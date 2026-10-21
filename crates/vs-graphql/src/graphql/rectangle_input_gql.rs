#![warn(clippy::pedantic)]
#![allow(
    clippy::missing_errors_doc,
    clippy::missing_panics_doc,
    clippy::too_many_lines,
    clippy::unused_async_trait_impl
)]

use crate::slices::Rectangle;
use async_graphql::InputObject;
use rs_math3d::Vec3d;

use super::Vec3InputGql;

#[derive(InputObject, Clone)]
pub struct RectangleInputGql {
    top_left: Vec3InputGql,
    bottom_right: Vec3InputGql,
}

impl From<RectangleInputGql> for Rectangle {
    fn from(value: RectangleInputGql) -> Self {
        Self::new(
            Vec3d::new(
                value.top_left.x(),
                value.top_left.y(),
                value.top_left.z().unwrap_or(0.0),
            ),
            Vec3d::new(
                value.bottom_right.x(),
                value.bottom_right.y(),
                value.bottom_right.z().unwrap_or(0.0),
            ),
        )
    }
}

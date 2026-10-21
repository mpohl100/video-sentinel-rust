#![warn(clippy::pedantic)]
#![allow(
    clippy::missing_errors_doc,
    clippy::missing_panics_doc,
    clippy::too_many_lines,
    clippy::unused_async_trait_impl
)]

use crate::slices::Rectangle;

use super::Vec3Gql;

#[derive(async_graphql::SimpleObject, Clone)]
pub struct RectangleGql {
    top_left: Vec3Gql,
    bottom_right: Vec3Gql,
}

impl From<Rectangle> for RectangleGql {
    fn from(value: Rectangle) -> Self {
        Self {
            top_left: value.get_top_left().into(),
            bottom_right: value.get_bottom_right().into(),
        }
    }
}

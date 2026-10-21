#![warn(clippy::pedantic)]
#![allow(
    clippy::missing_errors_doc,
    clippy::missing_panics_doc,
    clippy::too_many_lines,
    clippy::unused_async_trait_impl
)]

#[derive(async_graphql::SimpleObject, Clone)]
pub struct PointGql {
    x: f64,
    y: f64,
}

impl From<crate::service::Point> for PointGql {
    fn from(value: crate::service::Point) -> Self {
        Self {
            x: value.x,
            y: value.y,
        }
    }
}

#![warn(clippy::pedantic)]
#![allow(
    clippy::missing_errors_doc,
    clippy::missing_panics_doc,
    clippy::too_many_lines,
    clippy::unused_async_trait_impl
)]

use rs_math3d::Vec3d;

#[derive(async_graphql::SimpleObject, Clone)]
pub struct Vec3Gql {
    x: f64,
    y: f64,
    z: f64,
}

impl From<Vec3d> for Vec3Gql {
    fn from(value: Vec3d) -> Self {
        Self {
            x: value.x,
            y: value.y,
            z: value.z,
        }
    }
}

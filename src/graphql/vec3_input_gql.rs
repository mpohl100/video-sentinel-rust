#![warn(clippy::pedantic)]
#![allow(
    clippy::missing_errors_doc,
    clippy::missing_panics_doc,
    clippy::too_many_lines,
    clippy::unused_async_trait_impl
)]

use async_graphql::InputObject;

#[derive(InputObject, Clone)]
pub struct Vec3InputGql {
    x: f64,
    y: f64,
    z: Option<f64>,
}

impl Vec3InputGql {
    pub(super) fn x(&self) -> f64 {
        self.x
    }

    pub(super) fn y(&self) -> f64 {
        self.y
    }

    pub(super) fn z(&self) -> Option<f64> {
        self.z
    }
}

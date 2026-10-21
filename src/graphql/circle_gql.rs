#![warn(clippy::pedantic)]
#![allow(
    clippy::missing_errors_doc,
    clippy::missing_panics_doc,
    clippy::too_many_lines,
    clippy::unused_async_trait_impl
)]

use crate::service::Circle;

use super::Vec3Gql;

#[derive(async_graphql::SimpleObject, Clone)]
pub struct CircleGql {
    center: Vec3Gql,
    radius: f64,
}

impl From<Circle> for CircleGql {
    fn from(value: Circle) -> Self {
        Self {
            center: value.center.into(),
            radius: value.radius,
        }
    }
}

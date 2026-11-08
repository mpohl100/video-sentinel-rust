#![warn(clippy::pedantic)]
#![allow(
    clippy::missing_errors_doc,
    clippy::missing_panics_doc,
    clippy::too_many_lines,
    clippy::unused_async_trait_impl
)]

use super::Vec3Gql;
use vs_service::service::Triangle;

#[allow(clippy::struct_field_names)]
#[derive(async_graphql::SimpleObject, Clone)]
pub struct TriangleGql {
    vertex_a: Vec3Gql,
    vertex_b: Vec3Gql,
    vertex_c: Vec3Gql,
}

impl From<Triangle> for TriangleGql {
    fn from(value: Triangle) -> Self {
        Self {
            vertex_a: value.vertices[0].into(),
            vertex_b: value.vertices[1].into(),
            vertex_c: value.vertices[2].into(),
        }
    }
}

#![warn(clippy::pedantic)]
#![allow(
    clippy::missing_errors_doc,
    clippy::missing_panics_doc,
    clippy::too_many_lines,
    clippy::unused_async_trait_impl
)]

use async_graphql::InputObject;
use vs_service::service::RigidTransform;

#[derive(InputObject, Clone)]
pub struct RigidTransformInputGql {
    translation_x: f64,
    translation_y: f64,
    translation_z: f64,
    rotation_x: f64,
    rotation_y: f64,
    rotation_z: f64,
    rotation_w: f64,
}

impl From<RigidTransformInputGql> for RigidTransform {
    fn from(value: RigidTransformInputGql) -> Self {
        Self {
            translation: [
                value.translation_x,
                value.translation_y,
                value.translation_z,
            ],
            rotation_xyzw: [
                value.rotation_x,
                value.rotation_y,
                value.rotation_z,
                value.rotation_w,
            ],
        }
    }
}

#![warn(clippy::pedantic)]
#![allow(
    clippy::missing_errors_doc,
    clippy::missing_panics_doc,
    clippy::too_many_lines,
    clippy::unused_async_trait_impl
)]

use async_graphql::InputObject;
use vs_service::service::CameraIntrinsics;

#[derive(InputObject, Clone)]
pub struct CameraIntrinsicsInputGql {
    width: u32,
    height: u32,
    fx: f64,
    fy: f64,
    cx: f64,
    cy: f64,
    #[graphql(default = "plumb_bob")]
    distortion_model: String,
    #[graphql(default)]
    distortion: Vec<f64>,
}

impl From<CameraIntrinsicsInputGql> for CameraIntrinsics {
    fn from(value: CameraIntrinsicsInputGql) -> Self {
        Self {
            width: value.width,
            height: value.height,
            fx: value.fx,
            fy: value.fy,
            cx: value.cx,
            cy: value.cy,
            distortion_model: value.distortion_model,
            distortion: value.distortion,
        }
    }
}

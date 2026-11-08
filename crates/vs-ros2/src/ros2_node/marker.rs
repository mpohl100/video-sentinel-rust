#![warn(clippy::pedantic)]
#![allow(
    clippy::missing_errors_doc,
    clippy::missing_panics_doc,
    clippy::too_many_lines,
    clippy::unused_async_trait_impl
)]

use ros2_client::builtin_interfaces::Duration;
use serde::{Deserialize, Serialize};

use super::{CompressedSensorImage, Header, Point, Pose, Vector3};

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
pub struct UVCoordinate {
    pub u: f32,
    pub v: f32,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
pub struct MeshFile {
    pub filename: String,
    pub data: Vec<u8>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct ColorRgba {
    pub r: f32,
    pub g: f32,
    pub b: f32,
    pub a: f32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Marker {
    pub header: Header,
    pub ns: String,
    pub id: i32,
    #[serde(rename = "type")]
    pub marker_type: i32,
    pub action: i32,
    pub pose: Pose,
    pub scale: Vector3,
    pub color: ColorRgba,
    pub lifetime: Duration,
    pub frame_locked: bool,
    pub points: Vec<Point>,
    pub colors: Vec<ColorRgba>,
    pub texture_resource: String,
    pub texture: CompressedSensorImage,
    pub uv_coordinates: Vec<UVCoordinate>,
    pub text: String,
    pub mesh_resource: String,
    pub mesh_file: MeshFile,
    pub mesh_use_embedded_materials: bool,
}

impl Marker {
    pub const ADD: i32 = 0;
    pub const TRIANGLE_LIST: i32 = 11;
}

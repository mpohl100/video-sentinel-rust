#![warn(clippy::pedantic)]
#![allow(
    clippy::missing_errors_doc,
    clippy::missing_panics_doc,
    clippy::too_many_lines,
    clippy::unused_async_trait_impl
)]

use crate::mosaics::Results;
use crate::slices::Color;
use crate::slices::Rectangle;
use rs_math3d::Vec3d;

use super::{Circle, RgbColor, SliceLine};

#[derive(Clone)]
pub struct EnrichedMosaic {
    pub bounding_box: Rectangle,
    pub bounding_circle: Circle,
    pub color: Color,
    pub label: Option<String>,
    pub area: f64,
    pub center_of_mass: Vec3d,
    pub slice_matrix: Vec<SliceLine>,
    pub average_color: RgbColor,
    pub results: Results,
}

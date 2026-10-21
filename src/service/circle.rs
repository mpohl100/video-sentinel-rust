#![warn(clippy::pedantic)]
#![allow(
    clippy::missing_errors_doc,
    clippy::missing_panics_doc,
    clippy::too_many_lines,
    clippy::unused_async_trait_impl
)]

use rs_math3d::Vec3d;

#[derive(Clone, Copy)]
/// Bounding circle returned with each `EnrichedMosaic`.
/// The center is exported in the global coordinate system representation.
/// In `Results::Relative`, the values remain normalized (0..1) before export.
pub struct Circle {
    pub center: Vec3d,
    pub radius: f64,
}

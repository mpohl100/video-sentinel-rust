#![warn(clippy::pedantic)]
#![allow(
    clippy::missing_errors_doc,
    clippy::missing_panics_doc,
    clippy::too_many_lines,
    clippy::unused_async_trait_impl
)]

use rs_math3d::Vec3d;

#[derive(Clone, Copy)]
pub struct Point {
    pub point: Vec3d,
}

impl PartialEq for Point {
    fn eq(&self, other: &Self) -> bool {
        let epsilon = 1e-6;
        (self.point.x - other.point.x).abs() < epsilon
            && (self.point.y - other.point.y).abs() < epsilon
            && (self.point.z - other.point.z).abs() < epsilon
    }
}

impl Point {
    #[must_use]
    pub fn new(point: Vec3d) -> Self {
        Self { point }
    }

    #[must_use]
    pub fn get_x(&self) -> f64 {
        self.point.x
    }

    #[must_use]
    pub fn get_y(&self) -> f64 {
        self.point.y
    }
}

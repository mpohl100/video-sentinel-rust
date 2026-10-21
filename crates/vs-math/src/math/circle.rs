#![warn(clippy::pedantic)]
#![allow(
    clippy::missing_errors_doc,
    clippy::missing_panics_doc,
    clippy::too_many_lines,
    clippy::unused_async_trait_impl
)]

use rs_math3d::{FloatVector, Vec3d};

use super::Rectangle;

#[derive(Clone)]
pub struct Circle {
    center: Vec3d,
    radius: f64,
}

impl Circle {
    pub(super) fn center(&self) -> Vec3d {
        self.center
    }

    pub(super) fn radius(&self) -> f64 {
        self.radius
    }
}

impl PartialEq for Circle {
    fn eq(&self, other: &Self) -> bool {
        let center_diff = self.center - other.center;
        (center_diff.length() < 1e-6) && (self.radius - other.radius).abs() < 1e-6
    }
}

impl Circle {
    #[must_use]
    pub fn new(center: Vec3d, radius: f64) -> Self {
        Self { center, radius }
    }

    #[must_use]
    pub fn intersects(&self, other: &Circle) -> bool {
        let center_diff = self.center - other.center;
        center_diff.length() < (self.radius + other.radius)
    }

    #[must_use]
    pub fn get_bounding_box(&self) -> Rectangle {
        let top_left = Vec3d::new(
            self.center.x - self.radius,
            self.center.y - self.radius,
            0.0,
        );
        let bottom_right = Vec3d::new(
            self.center.x + self.radius,
            self.center.y + self.radius,
            0.0,
        );
        Rectangle::new(top_left, bottom_right)
    }

    #[must_use]
    pub fn get_area(&self) -> f64 {
        std::f64::consts::PI * self.radius * self.radius
    }

    #[must_use]
    pub fn get_center(&self) -> Vec3d {
        self.center
    }

    #[must_use]
    pub fn get_radius(&self) -> f64 {
        self.radius
    }
}

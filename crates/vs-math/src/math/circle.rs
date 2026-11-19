#![warn(clippy::pedantic)]
#![allow(
    clippy::missing_errors_doc,
    clippy::missing_panics_doc,
    clippy::too_many_lines,
    clippy::unused_async_trait_impl
)]

use crate::math::Line;
use crate::math::RegionedAngle;
use crate::math::get_circle_line_intersection_points;
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

#[derive(Clone)]
pub struct PartialCircle {
    circle: Circle,
    start_angle: RegionedAngle,
    end_angle: RegionedAngle,
}

impl PartialCircle {
    #[must_use]
    pub fn new(circle: Circle, start_angle: RegionedAngle, end_angle: RegionedAngle) -> Self {
        Self {
            circle,
            start_angle,
            end_angle,
        }
    }

    pub fn get_intersection_points(&self, line: &super::Line) -> Vec<Vec3d> {
        let intersection_points_raw = get_circle_line_intersection_points(&self.circle, &line);
        intersection_points_raw
            .iter()
            .filter(|p| {
                let vec = **p - self.circle.center();
                let arcsin_angle_degrees =
                    (vec.y / self.circle.radius()).asin() * 180.0 / std::f64::consts::PI;
                let angle = if vec.x >= 0.0 {
                    arcsin_angle_degrees
                } else {
                    180.0 - arcsin_angle_degrees
                };
                let regioned_angle = RegionedAngle::new(angle, 0.0, 360.0);
                regioned_angle.get_angle() >= self.start_angle.get_angle()
                    && regioned_angle.get_angle() <= self.end_angle.get_angle()
            }).map(|p| *p)
            .collect()
    }

    #[must_use]
    pub fn get_circle(&self) -> &Circle {
        &self.circle
    }

    #[must_use]
    pub fn get_start_angle(&self) -> RegionedAngle {
        self.start_angle
    }

    #[must_use]
    pub fn get_end_angle(&self) -> RegionedAngle {
        self.end_angle
    }
}

#![warn(clippy::pedantic)]
#![allow(
    clippy::missing_errors_doc,
    clippy::missing_panics_doc,
    clippy::too_many_lines,
    clippy::unused_async_trait_impl
)]

use rs_math3d::Vector;
use rs_math3d::{CrossProduct, FloatVector, Vec3d, Vector3};

use super::{Point, RegionedAngle};

#[derive(Clone, Copy)]
pub struct Line {
    start: Vec3d,
    end: Vec3d,
}

impl Line {
    pub(super) fn start(&self) -> Vec3d {
        self.start
    }

    pub(super) fn end(&self) -> Vec3d {
        self.end
    }
}

impl PartialEq for Line {
    fn eq(&self, other: &Self) -> bool {
        let start_diff = self.start - other.start;
        let end_diff = self.end - other.end;
        start_diff.length() < 1e-6 && end_diff.length() < 1e-6
    }
}

impl Line {
    #[must_use]
    pub fn new(start: Vec3d, end: Vec3d) -> Self {
        Self { start, end }
    }

    #[must_use]
    pub fn intersects(&self, other: &Line) -> bool {
        let self_start = self.start;
        let self_delta = self.end - self.start;
        let other_start = other.start;
        let other_delta = other.end - other.start;

        let self_cross_other = Vector3::<f64>::cross(&self_delta, &other_delta);
        let start_delta = other_start - self_start;

        if self_cross_other.length() < 1e-6 {
            // Lines are parallel
            return false;
        }

        let self_ratio = Vector3::<f64>::cross(&start_delta, &other_delta).z / self_cross_other.z;
        let other_ratio = Vector3::<f64>::cross(&start_delta, &self_delta).z / self_cross_other.z;

        (0.0..=1.0).contains(&self_ratio) && (0.0..=1.0).contains(&other_ratio)
    }

    #[must_use]
    pub fn get_intersection_point(&self, other: &Line) -> Point {
        let self_start = self.start;
        let self_delta = self.end - self.start;
        let other_start = other.start;
        let other_delta = other.end - other.start;

        let self_cross_other = Vector3::<f64>::cross(&self_delta, &other_delta);
        let start_delta = other_start - self_start;

        if self_cross_other.length() < 1e-6 {
            // Lines are parallel, return the midpoint of the overlapping segment as the intersection point
            return Point::new((self.start + self.end) / 2.0);
        }

        let self_ratio = Vector3::<f64>::cross(&start_delta, &other_delta).z / self_cross_other.z;
        Point::new(self_start + self_delta * self_ratio)
    }

    #[must_use]
    pub fn angle_between(&self, other: &Line) -> RegionedAngle {
        let v1 = self.end - self.start;
        let v2 = other.end - other.start;
        // calculate the angle between v1 and v2 using the dot product
        let dot_product = Vector3::<f64>::dot(&v1, &v2);
        let v1_length = v1.length();
        let v2_length = v2.length();
        if v1_length < 1e-6 || v2_length < 1e-6 {
            return RegionedAngle {
                angle_degrees: 0.0,
                min_degrees: -180.0,
                max_degrees: 180.0,
            }; // avoid division by zero, treat zero-length vectors as having zero angle between them
        }
        let cos_angle = dot_product / (v1_length * v2_length);
        RegionedAngle {
            angle_degrees: cos_angle.acos().to_degrees(),
            min_degrees: -180.0,
            max_degrees: 180.0,
        }
    }

    #[must_use]
    pub fn get_start(&self) -> Point {
        Point::new(self.start)
    }

    #[must_use]
    pub fn get_end(&self) -> Point {
        Point::new(self.end)
    }
}

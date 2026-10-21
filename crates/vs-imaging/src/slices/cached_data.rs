#![warn(clippy::pedantic)]
#![allow(
    clippy::missing_errors_doc,
    clippy::missing_panics_doc,
    clippy::too_many_lines,
    clippy::unused_async_trait_impl
)]

use crate::math::CoordinatedCircle;
use crate::math::CoordinatedPoint;
use crate::math::CoordinatedRectangle;
use rs_math3d::Vec3d;

#[derive(Clone)]
pub struct CachedData {
    bounding_box: CoordinatedRectangle,
    bounding_circle: CoordinatedCircle,
    center_of_mass: CoordinatedPoint,
    area: f64,
    average_color: Vec3d,
}

impl CachedData {
    #[must_use]
    pub fn new(
        bounding_box: CoordinatedRectangle,
        bounding_circle: CoordinatedCircle,
        center_of_mass: CoordinatedPoint,
        area: f64,
        average_color: Vec3d,
    ) -> Self {
        Self {
            bounding_box,
            bounding_circle,
            center_of_mass,
            area,
            average_color,
        }
    }

    #[must_use]
    pub fn get_bounding_box(&self) -> CoordinatedRectangle {
        self.bounding_box.clone()
    }

    #[must_use]
    pub fn get_bounding_circle(&self) -> CoordinatedCircle {
        self.bounding_circle.clone()
    }

    #[must_use]
    pub fn get_center_of_mass(&self) -> CoordinatedPoint {
        self.center_of_mass.clone()
    }

    #[must_use]
    pub fn get_area(&self) -> f64 {
        self.area
    }

    #[must_use]
    pub fn get_average_color_vec(&self) -> Vec3d {
        self.average_color
    }
}

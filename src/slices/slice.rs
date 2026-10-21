#![warn(clippy::pedantic)]
#![allow(
    clippy::missing_errors_doc,
    clippy::missing_panics_doc,
    clippy::too_many_lines,
    clippy::unused_async_trait_impl
)]

use crate::math::AnonymizedCoordinateSystem;
use crate::math::CoordinateSystem;
use crate::math::CoordinatedPoint;
use crate::math::CoordinatedRectangle;
use rs_math3d::{FloatVector, Vec3d};

#[derive(Clone)]
pub struct Slice {
    global_rectangle: CoordinatedRectangle,
}

impl PartialEq for Slice {
    fn eq(&self, other: &Self) -> bool {
        let self_start_global = self.global_rectangle.get_top_left();
        let self_end_global = self.global_rectangle.get_top_right();
        let other_start_global = other.global_rectangle.get_top_left();
        let other_end_global = other.global_rectangle.get_top_right();
        let start_diff =
            (self_start_global.get_local_point() - other_start_global.get_local_point()).length();
        let end_diff =
            (self_end_global.get_local_point() - other_end_global.get_local_point()).length();
        start_diff < 1e-6 && end_diff < 1e-6
    }
}

impl Slice {
    #[must_use]
    pub fn new(start: &CoordinatedPoint, end: &CoordinatedPoint) -> Self {
        let global_coordinate_system = AnonymizedCoordinateSystem::Direct(CoordinateSystem::new(
            Vec3d::new(0.0, 0.0, 0.0),
            Vec3d::new(1.0, 0.0, 0.0),
            Vec3d::new(0.0, 1.0, 0.0),
        ));
        let global_start = start.convert_to(global_coordinate_system.clone());
        let global_end = end.convert_to(global_coordinate_system.clone());
        let adjusted_end = CoordinatedPoint::new(
            global_coordinate_system.clone(),
            Vec3d::new(
                global_end.get_local_point().x,
                global_start.get_local_point().y + 1.0,
                0.0,
            ),
        );
        Self {
            global_rectangle: CoordinatedRectangle::new(global_start, adjusted_end),
        }
    }

    #[must_use]
    pub fn get_global_rectangle(&self) -> &CoordinatedRectangle {
        &self.global_rectangle
    }

    #[must_use]
    pub fn get_start(&self) -> &CoordinatedPoint {
        self.global_rectangle.get_top_left()
    }

    #[must_use]
    pub fn get_end(&self) -> &CoordinatedPoint {
        self.global_rectangle.get_top_right()
    }

    #[must_use]
    pub fn convert_to_global(&self) -> Slice {
        let global_coordinate_system = AnonymizedCoordinateSystem::Direct(CoordinateSystem::new(
            Vec3d::new(0.0, 0.0, 0.0),
            Vec3d::new(1.0, 0.0, 0.0),
            Vec3d::new(0.0, 1.0, 0.0),
        ));
        Slice {
            global_rectangle: CoordinatedRectangle::new(
                self.global_rectangle
                    .get_top_left()
                    .convert_to(global_coordinate_system.clone()),
                self.global_rectangle
                    .get_top_right()
                    .convert_to(global_coordinate_system.clone()),
            ),
        }
    }

    #[must_use]
    pub fn convert_to(&self, coordinate_system: &AnonymizedCoordinateSystem) -> Slice {
        Slice {
            global_rectangle: CoordinatedRectangle::new(
                self.global_rectangle
                    .get_top_left()
                    .convert_to(coordinate_system.clone()),
                self.global_rectangle
                    .get_top_right()
                    .convert_to(coordinate_system.clone()),
            ),
        }
    }
}

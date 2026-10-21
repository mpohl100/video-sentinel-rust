#![warn(clippy::pedantic)]
#![allow(
    clippy::missing_errors_doc,
    clippy::missing_panics_doc,
    clippy::too_many_lines,
    clippy::unused_async_trait_impl
)]

use rs_math3d::{FloatVector, Vec3d};

use super::{AnonymizedCoordinateSystem, CoordinateSystem, RegionedAngle};

#[derive(Clone)]
pub struct CoordinatedPoint {
    coordinate_system: AnonymizedCoordinateSystem,
    local_coordinates: Vec3d,
}

impl CoordinatedPoint {
    pub(super) fn coordinate_system(&self) -> &AnonymizedCoordinateSystem {
        &self.coordinate_system
    }

    pub(super) fn local_coordinates(&self) -> Vec3d {
        self.local_coordinates
    }
}

impl PartialEq for CoordinatedPoint {
    fn eq(&self, other: &Self) -> bool {
        let global_self = self.coordinate_system.to_global(self);
        let global_other = other.coordinate_system.to_global(other);
        let diff = global_self.get_local_point() - global_other.get_local_point();
        diff.length() < 1e-6
    }
}

impl CoordinatedPoint {
    #[must_use]
    pub fn new(coordinate_system: AnonymizedCoordinateSystem, local_coordinates: Vec3d) -> Self {
        Self {
            coordinate_system,
            local_coordinates,
        }
    }

    #[must_use]
    pub fn get_coordinate_system(&self) -> AnonymizedCoordinateSystem {
        self.coordinate_system.clone()
    }

    #[must_use]
    pub fn to_global_point(&self) -> CoordinatedPoint {
        let global_coordinate_system =
            AnonymizedCoordinateSystem::new_direct(CoordinateSystem::new(
                Vec3d::new(0.0, 0.0, 0.0),
                Vec3d::new(1.0, 0.0, 0.0),
                Vec3d::new(0.0, 1.0, 0.0),
            ));
        self.convert_to(global_coordinate_system)
    }

    #[must_use]
    pub fn get_local_point(&self) -> Vec3d {
        self.local_coordinates
    }

    pub fn set_y(&mut self, y: f64) {
        self.local_coordinates.y = y;
    }

    pub fn set_x(&mut self, x: f64) {
        self.local_coordinates.x = x;
    }

    pub fn set_z(&mut self, z: f64) {
        self.local_coordinates.z = z;
    }

    #[must_use]
    pub fn convert_to(&self, coordinate_system: AnonymizedCoordinateSystem) -> CoordinatedPoint {
        let global_point = self.coordinate_system.to_global(self).get_local_point();
        let local_coordinates = coordinate_system
            .convert_from_global(&CoordinatedPoint::new(
                AnonymizedCoordinateSystem::new_direct(CoordinateSystem::new(
                    Vec3d::new(0.0, 0.0, 0.0),
                    Vec3d::new(1.0, 0.0, 0.0),
                    Vec3d::new(0.0, 1.0, 0.0),
                )),
                global_point,
            ))
            .get_local_point();
        CoordinatedPoint::new(coordinate_system, local_coordinates)
    }

    #[must_use]
    pub fn plus(&self, other: Vec3d) -> CoordinatedPoint {
        let new_local = self.local_coordinates + other;
        CoordinatedPoint::new(self.coordinate_system.clone(), new_local)
    }

    #[must_use]
    pub fn rotate(&self, around: &CoordinatedPoint, angle: RegionedAngle) -> CoordinatedPoint {
        let around_local = around.convert_to(self.coordinate_system.clone());
        let translated_x = self.local_coordinates.x - around_local.local_coordinates.x;
        let translated_y = self.local_coordinates.y - around_local.local_coordinates.y;
        let cos_angle = angle.radians().cos();
        let sin_angle = angle.radians().sin();
        let rotated_x = translated_x * cos_angle - translated_y * sin_angle;
        let rotated_y = translated_x * sin_angle + translated_y * cos_angle;
        let new_local = Vec3d::new(
            rotated_x + around_local.local_coordinates.x,
            rotated_y + around_local.local_coordinates.y,
            0.0,
        );
        CoordinatedPoint::new(self.coordinate_system.clone(), new_local)
    }

    #[must_use]
    pub fn distance_to(&self, other: &CoordinatedPoint) -> f64 {
        let global_self = self.coordinate_system.to_global(self);
        let global_other = other.coordinate_system.to_global(other);
        (global_self.get_local_point() - global_other.get_local_point()).length()
    }

    #[must_use]
    pub fn get_x(&self) -> f64 {
        self.local_coordinates.x
    }

    #[must_use]
    pub fn get_y(&self) -> f64 {
        self.local_coordinates.y
    }

    #[must_use]
    pub fn get_z(&self) -> f64 {
        self.local_coordinates.z
    }
}

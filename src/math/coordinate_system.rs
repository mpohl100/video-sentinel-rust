#![warn(clippy::pedantic)]
#![allow(
    clippy::missing_errors_doc,
    clippy::missing_panics_doc,
    clippy::too_many_lines,
    clippy::unused_async_trait_impl
)]

use rs_math3d::Vector;
use rs_math3d::{CrossProduct, Vec3d, Vector3};

use super::{AnonymizedCoordinateSystem, CoordinatedPoint, RegionedAngle};

#[derive(Clone, Copy)]
pub struct CoordinateSystem {
    origin: Vec3d,
    x_axis: Vec3d,
    y_axis: Vec3d,
}

impl CoordinateSystem {
    pub(super) fn origin(&self) -> Vec3d {
        self.origin
    }

    pub(super) fn x_axis(&self) -> Vec3d {
        self.x_axis
    }

    pub(super) fn y_axis(&self) -> Vec3d {
        self.y_axis
    }
}

impl CoordinateSystem {
    #[must_use]
    pub fn new(origin: Vec3d, x_axis: Vec3d, y_axis: Vec3d) -> Self {
        Self {
            origin,
            x_axis,
            y_axis,
        }
    }

    pub fn rotate(&mut self, angle: RegionedAngle) {
        let angle_radians = angle.radians();
        let cos_angle = angle_radians.cos();
        let sin_angle = angle_radians.sin();
        let rotated_primary_axis = Vec3d::new(
            self.x_axis.x * cos_angle - self.y_axis.x * sin_angle,
            self.x_axis.y * cos_angle - self.y_axis.y * sin_angle,
            0.0,
        );
        let rotated_secondary_axis = Vec3d::new(
            self.x_axis.x * sin_angle + self.y_axis.x * cos_angle,
            self.x_axis.y * sin_angle + self.y_axis.y * cos_angle,
            0.0,
        );
        self.x_axis = rotated_primary_axis;
        self.y_axis = rotated_secondary_axis;
    }

    #[must_use]
    pub fn to_local(self, point: &CoordinatedPoint) -> CoordinatedPoint {
        let global_point = point.coordinate_system().to_global(point);
        let local_coordinates = self.to_local_coordinates(global_point.get_local_point());
        CoordinatedPoint::new(AnonymizedCoordinateSystem::Direct(self), local_coordinates)
    }

    #[must_use]
    pub fn convert_from_global(&self, point: Vec3d) -> CoordinatedPoint {
        let local_coordinates = self.to_local_coordinates(point);
        CoordinatedPoint::new(AnonymizedCoordinateSystem::Direct(*self), local_coordinates)
    }

    #[must_use]
    pub fn to_global(self, point: &CoordinatedPoint) -> Vec3d {
        let local = point.local_coordinates();
        self.origin + self.x_axis * local.x + self.y_axis * local.y
    }

    pub(super) fn to_local_coordinates(self, global: Vec3d) -> Vec3d {
        let relative = global - self.origin;
        let x = Vector3::<f64>::dot(&relative, &self.x_axis)
            / Vector3::<f64>::dot(&self.x_axis, &self.x_axis);
        let y = Vector3::<f64>::dot(&relative, &self.y_axis)
            / Vector3::<f64>::dot(&self.y_axis, &self.y_axis);
        Vec3d::new(x, y, 0.0)
    }

    #[must_use]
    pub fn get_angle_between(&self, other: &AnonymizedCoordinateSystem) -> RegionedAngle {
        let self_x_axis = self.x_axis;
        let other_x_axis = match other {
            AnonymizedCoordinateSystem::Direct(cs) => cs.x_axis,
            AnonymizedCoordinateSystem::Indirect(wrapped_cs) => {
                wrapped_cs.coordinate_system().lock().unwrap().x_axis
            }
        };
        let dot = Vector3::<f64>::dot(&self_x_axis, &other_x_axis);
        let cross = Vector3::<f64>::cross(&self_x_axis, &other_x_axis);

        RegionedAngle::new(cross.z.atan2(dot).to_degrees(), -180.0, 180.0)
    }

    pub fn align_x_axis_with(&mut self, other: &AnonymizedCoordinateSystem) {
        let angle_between = self.get_angle_between(other);
        let angle_to_rotate = RegionedAngle::new(
            -angle_between.angle_degrees,
            angle_between.min_degrees,
            angle_between.max_degrees,
        );
        self.rotate(angle_to_rotate);
    }
}

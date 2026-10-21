#![warn(clippy::pedantic)]
#![allow(
    clippy::missing_errors_doc,
    clippy::missing_panics_doc,
    clippy::too_many_lines,
    clippy::unused_async_trait_impl
)]

use rs_math3d::Vec3d;

use super::{AnonymizedCoordinateSystem, CoordinatedPoint, CoordinatedRegionedAngle};

#[derive(Clone)]
pub struct PolarCoordinates {
    radius: f64,
    angle: CoordinatedRegionedAngle,
}

impl PolarCoordinates {
    #[must_use]
    pub fn new(radius: f64, angle: CoordinatedRegionedAngle) -> Self {
        Self { radius, angle }
    }

    #[must_use]
    pub fn convert_to(&self, coordinate_system: AnonymizedCoordinateSystem) -> PolarCoordinates {
        let new_angle = self.angle.convert_to(coordinate_system);
        PolarCoordinates::new(self.radius, new_angle)
    }

    #[must_use]
    pub fn to_cartesian(&self) -> CoordinatedPoint {
        let angle_radians = self.angle.get_regioned_angle().radians();
        let x = self.radius * angle_radians.cos();
        let y = self.radius * angle_radians.sin();
        CoordinatedPoint::new(self.angle.get_coordinate_system(), Vec3d::new(x, y, 0.0))
    }

    #[must_use]
    pub fn get_radius(&self) -> f64 {
        self.radius
    }

    #[must_use]
    pub fn get_angle(&self) -> CoordinatedRegionedAngle {
        self.angle.clone()
    }

    #[must_use]
    pub fn get_coordinate_system(&self) -> AnonymizedCoordinateSystem {
        self.angle.get_coordinate_system()
    }
}

#![warn(clippy::pedantic)]
#![allow(
    clippy::missing_errors_doc,
    clippy::missing_panics_doc,
    clippy::too_many_lines,
    clippy::unused_async_trait_impl
)]

use super::{AnonymizedCoordinateSystem, CoordinatedLine, RegionedAngle};

#[derive(Clone)]
pub struct CoordinatedRegionedAngle {
    coordinate_system: AnonymizedCoordinateSystem,
    regioned_angle: RegionedAngle,
}

impl CoordinatedRegionedAngle {
    #[must_use]
    pub fn new(
        coordinate_system: AnonymizedCoordinateSystem,
        regioned_angle: RegionedAngle,
    ) -> Self {
        Self {
            coordinate_system,
            regioned_angle,
        }
    }

    #[must_use]
    pub fn new_from_lines(
        line1: &CoordinatedLine,
        line2: &CoordinatedLine,
        min_degrees: f64,
        max_degrees: f64,
    ) -> Self {
        let global_line1 = line1.to_global_line();
        let global_line2 = line2.to_global_line();
        let angle_radians = global_line1.angle_between(&global_line2).radians();
        let angle_degrees = angle_radians.to_degrees();
        let regioned_angle = RegionedAngle::new(angle_degrees, min_degrees, max_degrees);
        CoordinatedRegionedAngle::new(line1.start().coordinate_system().clone(), regioned_angle)
    }

    #[must_use]
    pub fn convert_to(
        &self,
        coordinate_system: AnonymizedCoordinateSystem,
    ) -> CoordinatedRegionedAngle {
        let angle_between_coordinate_systems =
            self.coordinate_system.get_angle_between(&coordinate_system);
        let new_regioned_angle = self.regioned_angle.add_angle(RegionedAngle::new(
            -angle_between_coordinate_systems.angle_degrees,
            self.regioned_angle.min_degrees,
            self.regioned_angle.max_degrees,
        ));
        CoordinatedRegionedAngle::new(coordinate_system, new_regioned_angle)
    }

    #[must_use]
    pub fn get_regioned_angle(&self) -> RegionedAngle {
        self.regioned_angle
    }

    #[must_use]
    pub fn get_coordinate_system(&self) -> AnonymizedCoordinateSystem {
        self.coordinate_system.clone()
    }

    #[must_use]
    pub fn get_angle_degrees(&self) -> f64 {
        self.regioned_angle.angle_degrees
    }

    #[must_use]
    pub fn get_min_degrees(&self) -> f64 {
        self.regioned_angle.min_degrees
    }

    #[must_use]
    pub fn get_max_degrees(&self) -> f64 {
        self.regioned_angle.max_degrees
    }
}

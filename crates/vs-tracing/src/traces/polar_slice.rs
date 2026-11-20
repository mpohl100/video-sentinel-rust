#![warn(clippy::pedantic)]
#![allow(
    clippy::missing_errors_doc,
    clippy::missing_panics_doc,
    clippy::too_many_lines,
    clippy::unused_async_trait_impl
)]

use vs_math::math::RegionedAngle;

use crate::math::PolarCoordinates;
use crate::math::CoordinatedRegionedAngle;
use crate::ring::RingSector;

#[derive(Clone)]
pub(super) struct PolarSlice {
    start: PolarCoordinates,
    end: PolarCoordinates,
}

impl PolarSlice {
    pub(super) fn new(start: PolarCoordinates, end: PolarCoordinates) -> Self {
        if start.get_radius() < end.get_radius() {
            PolarSlice { start, end }
        } else {
            PolarSlice {
                start: end,
                end: start,
            }
        }
    }

    pub(super) fn get_start(&self) -> &PolarCoordinates {
        &self.start
    }

    pub(super) fn get_end(&self) -> &PolarCoordinates {
        &self.end
    }

    pub fn get_ring_sector(&self, delta_angle: CoordinatedRegionedAngle) -> RingSector {
        let radiuses = (self.start.get_radius(), self.end.get_radius());
        let half_angle = delta_angle.get_angle_degrees() / 2.0;
        let negative_half_angle = RegionedAngle::new(-half_angle, -180.0, 180.0);
        let positive_half_angle = RegionedAngle::new(half_angle, -180.0, 180.0);
        let coordinated_start_angle = self.start.get_angle().add_angle(negative_half_angle);
        let coordinated_end_angle = self.start.get_angle().add_angle(positive_half_angle);
        let angles = (coordinated_start_angle, coordinated_end_angle);

        RingSector::new_from_radiuses_and_angles(radiuses, angles)
    }
}

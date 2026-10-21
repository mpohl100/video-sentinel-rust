#![warn(clippy::pedantic)]
#![allow(
    clippy::missing_errors_doc,
    clippy::missing_panics_doc,
    clippy::too_many_lines,
    clippy::unused_async_trait_impl
)]

use rs_math3d::Vec3d;

use super::{AnonymizedCoordinateSystem, CoordinateSystem, CoordinatedPoint, Line};

#[derive(Clone)]
pub struct CoordinatedLine {
    start: CoordinatedPoint,
    end: CoordinatedPoint,
}

impl CoordinatedLine {
    pub(super) fn start(&self) -> &CoordinatedPoint {
        &self.start
    }
}

impl CoordinatedLine {
    #[must_use]
    pub fn new(start: CoordinatedPoint, end: CoordinatedPoint) -> Self {
        Self { start, end }
    }

    #[must_use]
    pub fn length(&self) -> f64 {
        self.start.distance_to(&self.end)
    }

    #[must_use]
    pub fn convert_to(&self, coordinate_system: AnonymizedCoordinateSystem) -> CoordinatedLine {
        CoordinatedLine {
            start: self.start.convert_to(coordinate_system.clone()),
            end: self.end.convert_to(coordinate_system),
        }
    }

    #[must_use]
    pub fn get_line(&self) -> Line {
        self.to_global_line()
    }

    #[must_use]
    pub fn to_global_line(&self) -> Line {
        let global_start = self.start.coordinate_system().to_global(&self.start);
        let global_end = self.end.coordinate_system().to_global(&self.end);
        Line::new(global_start.get_local_point(), global_end.get_local_point())
    }

    #[must_use]
    pub fn get_intersection_point(&self, other: &CoordinatedLine) -> Option<CoordinatedPoint> {
        let global_line1 = self.to_global_line();
        let global_line2 = other.to_global_line();
        let global_coordinate_system =
            AnonymizedCoordinateSystem::new_direct(CoordinateSystem::new(
                Vec3d::new(0.0, 0.0, 0.0),
                Vec3d::new(1.0, 0.0, 0.0),
                Vec3d::new(0.0, 1.0, 0.0),
            ));
        if global_line1.intersects(&global_line2) {
            // For simplicity, we will return the midpoint of the intersection as the intersection point
            let intersection_point = global_line1.get_intersection_point(&global_line2);
            let global_intersection_point =
                CoordinatedPoint::new(global_coordinate_system.clone(), intersection_point.point);
            let target_coordinate_system = self.start.coordinate_system().clone();
            let intersection_point =
                target_coordinate_system.convert_from_global(&global_intersection_point);
            Some(intersection_point)
        } else {
            None
        }
    }

    #[must_use]
    pub fn intersects(&self, other: &CoordinatedLine) -> bool {
        let global_line1 = self.to_global_line();
        let global_line2 = other.to_global_line();
        global_line1.intersects(&global_line2)
    }

    #[must_use]
    pub fn get_start(&self) -> CoordinatedPoint {
        self.start.clone()
    }

    #[must_use]
    pub fn get_end(&self) -> CoordinatedPoint {
        self.end.clone()
    }
}

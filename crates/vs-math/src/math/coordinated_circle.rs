#![warn(clippy::pedantic)]
#![allow(
    clippy::missing_errors_doc,
    clippy::missing_panics_doc,
    clippy::too_many_lines,
    clippy::unused_async_trait_impl
)]

use rs_math3d::{FloatVector, Vec3d};

use super::{
    AnonymizedCoordinateSystem, Circle, CoordinateSystem, CoordinatedLine, CoordinatedPoint,
    CoordinatedRectangle, get_circle_line_intersection_points,
};

#[derive(Clone)]
pub struct CoordinatedCircle {
    center: CoordinatedPoint,
    radius: f64,
}

impl CoordinatedCircle {
    #[must_use]
    pub fn new(center: CoordinatedPoint, radius: f64) -> Self {
        Self { center, radius }
    }

    #[must_use]
    pub fn convert_to(
        &self,
        wrapped_coordinate_system: &AnonymizedCoordinateSystem,
    ) -> CoordinatedCircle {
        CoordinatedCircle {
            center: self.center.convert_to(wrapped_coordinate_system.clone()),
            radius: self.radius,
        }
    }

    #[must_use]
    pub fn to_global_circle(&self) -> Circle {
        let global_center = self.center.coordinate_system().to_global(&self.center);
        Circle::new(global_center.get_local_point(), self.radius)
    }

    #[must_use]
    pub fn intersects(&self, other: &CoordinatedCircle) -> bool {
        let global_circle1 = self.to_global_circle();
        let global_circle2 = other.to_global_circle();
        global_circle1.intersects(&global_circle2)
    }

    #[must_use]
    pub fn get_center(&self) -> CoordinatedPoint {
        self.center.clone()
    }

    #[must_use]
    pub fn get_radius(&self) -> f64 {
        self.radius
    }

    #[must_use]
    pub fn get_bounding_box(&self) -> CoordinatedRectangle {
        let top_left = CoordinatedPoint::new(
            self.center.coordinate_system().clone(),
            Vec3d::new(
                self.center.local_coordinates().x - self.radius,
                self.center.local_coordinates().y - self.radius,
                0.0,
            ),
        );
        let bottom_right = CoordinatedPoint::new(
            self.center.coordinate_system().clone(),
            Vec3d::new(
                self.center.local_coordinates().x + self.radius,
                self.center.local_coordinates().y + self.radius,
                0.0,
            ),
        );
        CoordinatedRectangle::new(top_left, bottom_right)
    }

    #[must_use]
    pub fn get_area(&self) -> f64 {
        std::f64::consts::PI * self.radius * self.radius
    }

    #[must_use]
    pub fn contains_point(&self, point: &CoordinatedPoint) -> bool {
        let global_center = self.center.coordinate_system().to_global(&self.center);
        let global_point = point.coordinate_system().to_global(point);
        (global_center.get_local_point() - global_point.get_local_point()).length() <= self.radius
    }

    #[must_use]
    pub fn get_intersection_line(&self, line: &CoordinatedLine) -> Option<CoordinatedLine> {
        let global_circle = self.to_global_circle();
        let global_line = line.to_global_line();
        let intersection_points_raw =
            get_circle_line_intersection_points(&global_circle, &global_line);
        let global_coordinate_system =
            AnonymizedCoordinateSystem::new_direct(CoordinateSystem::new(
                Vec3d::new(0.0, 0.0, 0.0),
                Vec3d::new(1.0, 0.0, 0.0),
                Vec3d::new(0.0, 1.0, 0.0),
            ));
        let intersection_points = intersection_points_raw
            .iter()
            .map(|p| CoordinatedPoint::new(global_coordinate_system.clone(), *p))
            .collect::<Vec<_>>();
        if intersection_points.len() == 2 {
            let target_coordinate_system = line.start().coordinate_system().clone();
            Some(CoordinatedLine::new(
                target_coordinate_system.convert_from_global(&intersection_points[0]),
                target_coordinate_system.convert_from_global(&intersection_points[1]),
            ))
        } else {
            None
        }
    }
}

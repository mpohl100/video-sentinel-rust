#![warn(clippy::pedantic)]
#![allow(
    clippy::missing_errors_doc,
    clippy::missing_panics_doc,
    clippy::too_many_lines,
    clippy::unused_async_trait_impl
)]

use rs_math3d::Vec3d;

use super::{
    AnonymizedCoordinateSystem, CoordinateSystem, CoordinatedLine, CoordinatedPoint, Rectangle,
};

#[derive(Clone)]
pub struct CoordinatedRectangle {
    points: Vec<CoordinatedPoint>,
    cached_top_right: Option<CoordinatedPoint>,
    cached_bottom_left: Option<CoordinatedPoint>,
}

impl CoordinatedRectangle {
    #[must_use]
    pub fn new(top_left: CoordinatedPoint, bottom_right: CoordinatedPoint) -> Self {
        let points = vec![top_left, bottom_right];
        let mut rect = Self {
            points,
            cached_top_right: None,
            cached_bottom_left: None,
        };
        rect.calculate_top_right();
        rect.calculate_bottom_left();
        rect
    }

    #[must_use]
    pub fn new_from_rectangle(
        rectangle: &Rectangle,
        coordinate_system: &AnonymizedCoordinateSystem,
    ) -> Self {
        let global_coordinate_system =
            AnonymizedCoordinateSystem::new_direct(CoordinateSystem::new(
                Vec3d::new(0.0, 0.0, 0.0),
                Vec3d::new(1.0, 0.0, 0.0),
                Vec3d::new(0.0, 1.0, 0.0),
            ));
        let points = rectangle
            .points()
            .iter()
            .map(|point| {
                let coordinated_point =
                    CoordinatedPoint::new(global_coordinate_system.clone(), point.point);
                coordinated_point.convert_to(coordinate_system.clone())
            })
            .collect();
        let mut rect = Self {
            points,
            cached_top_right: None,
            cached_bottom_left: None,
        };
        rect.calculate_top_right();
        rect.calculate_bottom_left();
        rect
    }

    #[must_use]
    pub fn convert_to(
        &self,
        coordinate_system: &AnonymizedCoordinateSystem,
    ) -> CoordinatedRectangle {
        let mut rect = CoordinatedRectangle {
            points: self
                .points
                .iter()
                .map(|point| point.convert_to(coordinate_system.clone()))
                .collect(),
            cached_top_right: None,
            cached_bottom_left: None,
        };
        rect.calculate_top_right();
        rect.calculate_bottom_left();
        rect
    }

    #[must_use]
    pub fn get_local_rectangle(&self) -> Rectangle {
        let tl = self.points[0].get_local_point();
        let br = self.points[1].get_local_point();
        Rectangle::new(tl, br)
    }

    #[must_use]
    pub fn to_global_rectangle(&self) -> Rectangle {
        let tl = self.points[0].to_global_point().get_local_point();
        let br = self.points[1].to_global_point().get_local_point();
        Rectangle::new(tl, br)
    }

    #[must_use]
    pub fn overlaps(&self, other: &CoordinatedRectangle) -> bool {
        let global_rectangle1 = self.to_global_rectangle();
        let global_rectangle2 = other.to_global_rectangle();
        global_rectangle1.overlaps(&global_rectangle2)
    }

    #[must_use]
    pub fn intersects(&self, other: &CoordinatedRectangle) -> bool {
        let global_rectangle1 = self.to_global_rectangle();
        let global_rectangle2 = other.to_global_rectangle();
        global_rectangle1.intersects(&global_rectangle2)
    }

    #[must_use]
    pub fn get_intersection_line(&self, line: &CoordinatedLine) -> Option<CoordinatedLine> {
        let global_rectangle = self.to_global_rectangle();
        let global_line = line.to_global_line();
        let global_coordinate_system =
            AnonymizedCoordinateSystem::new_direct(CoordinateSystem::new(
                Vec3d::new(0.0, 0.0, 0.0),
                Vec3d::new(1.0, 0.0, 0.0),
                Vec3d::new(0.0, 1.0, 0.0),
            ));
        global_rectangle
            .get_intersection_line(global_line)
            .map(|intersection_line| {
                CoordinatedLine::new(
                    CoordinatedPoint::new(
                        global_coordinate_system.clone(),
                        intersection_line.start(),
                    ),
                    CoordinatedPoint::new(
                        global_coordinate_system.clone(),
                        intersection_line.end(),
                    ),
                )
            })
    }

    #[must_use]
    pub fn get_top_left(&self) -> &CoordinatedPoint {
        &self.points[0]
    }

    #[must_use]
    pub fn get_top_right(&self) -> &CoordinatedPoint {
        self.cached_top_right.as_ref().unwrap()
    }

    fn calculate_top_right(&mut self) {
        if self.cached_top_right.is_none() {
            let top_right = CoordinatedPoint::new(
                self.points[0].coordinate_system().clone(),
                Vec3d::new(
                    self.points[1].local_coordinates().x,
                    self.points[0].local_coordinates().y,
                    0.0,
                ),
            );
            self.cached_top_right = Some(top_right);
        }
    }

    #[must_use]
    pub fn get_bottom_left(&self) -> &CoordinatedPoint {
        self.cached_bottom_left.as_ref().unwrap()
    }

    fn calculate_bottom_left(&mut self) {
        if self.cached_bottom_left.is_none() {
            let bottom_left = CoordinatedPoint::new(
                self.points[0].coordinate_system().clone(),
                Vec3d::new(
                    self.points[0].local_coordinates().x,
                    self.points[1].local_coordinates().y,
                    0.0,
                ),
            );
            self.cached_bottom_left = Some(bottom_left);
        }
    }

    #[must_use]
    pub fn get_bottom_right(&self) -> &CoordinatedPoint {
        &self.points[1]
    }
}

#![warn(clippy::pedantic)]
#![allow(
    clippy::missing_errors_doc,
    clippy::missing_panics_doc,
    clippy::too_many_lines,
    clippy::unused_async_trait_impl
)]

use crate::math::CoordinateSystem;
use crate::{
    math::{
        AnonymizedCoordinateSystem, CoordinatedCircle, CoordinatedPoint, CoordinatedRectangle,
        Rectangle,
    },
    slices::{Rectangle as SliceRectangle, RelativeRectangle},
};
use rs_math3d::Vec3d;

use super::{CachedRelativeData, WrappedMosaic};

#[derive(Clone)]
pub(super) struct RelativeMosaic {
    mosaic: WrappedMosaic,
    absolute_rectangle: Rectangle,
    cached_relative_data: Option<CachedRelativeData>,
}

impl RelativeMosaic {
    pub fn new(mosaic: WrappedMosaic, absolute_rectangle: Rectangle) -> Self {
        Self {
            mosaic,
            absolute_rectangle,
            cached_relative_data: None,
        }
    }

    pub fn get_bounding_box(&mut self) -> CoordinatedRectangle {
        self.calculate_cached_relative_data();
        self.cached_relative_data
            .as_ref()
            .unwrap()
            .get_bounding_box()
    }

    pub fn get_bounding_circle(&mut self) -> CoordinatedCircle {
        self.calculate_cached_relative_data();
        self.cached_relative_data
            .as_ref()
            .unwrap()
            .get_bounding_circle()
    }

    pub fn get_center_of_mass(&mut self) -> CoordinatedPoint {
        self.calculate_cached_relative_data();
        self.cached_relative_data
            .as_ref()
            .unwrap()
            .get_center_of_mass()
    }

    pub fn get_area(&mut self) -> f64 {
        self.calculate_cached_relative_data();
        self.cached_relative_data.as_ref().unwrap().get_area()
    }

    pub fn get_average_color(&mut self) -> Vec3d {
        self.calculate_cached_relative_data();
        self.cached_relative_data
            .as_ref()
            .unwrap()
            .get_average_color_vec()
    }

    pub fn get_absolute_rectangle(&self) -> Rectangle {
        self.absolute_rectangle.clone()
    }

    pub fn get_mosaic(&self) -> WrappedMosaic {
        self.mosaic.clone()
    }

    fn calculate_cached_relative_data(&mut self) {
        if self.cached_relative_data.is_some() {
            return;
        }
        let relative_coordinate_system = AnonymizedCoordinateSystem::Direct(CoordinateSystem::new(
            Vec3d::new(0.0, 0.0, 0.0),
            Vec3d::new(1.0, 0.0, 0.0),
            Vec3d::new(0.0, 1.0, 0.0),
        ));
        let top_left = self.absolute_rectangle.get_top_left();
        let bottom_right = self.absolute_rectangle.get_bottom_right();
        let width = bottom_right.x - top_left.x;
        let height = bottom_right.y - top_left.y;
        assert!(width > 0.0, "Absolute rectangle width must be positive");
        assert!(height > 0.0, "Absolute rectangle height must be positive");
        let absolute_area = width * height;
        let relative_bounding_box =
            self.get_relative_bounding_box(relative_coordinate_system.clone());
        let bounding_circle = self.mosaic.get_bounding_circle();
        let relative_circle_center = Self::map_to_relative_point(
            &bounding_circle.get_center(),
            &relative_coordinate_system,
            top_left,
            width,
            height,
        );
        let radius = bounding_circle.get_radius();
        let circle_center_global = bounding_circle
            .get_center()
            .convert_to(relative_coordinate_system.clone())
            .get_local_point();
        let horizontal_relative_radius = Self::map_to_relative_point(
            &CoordinatedPoint::new(
                relative_coordinate_system.clone(),
                circle_center_global + Vec3d::new(radius, 0.0, 0.0),
            ),
            &relative_coordinate_system,
            top_left,
            width,
            height,
        )
        .distance_to(&relative_circle_center);
        let vertical_relative_radius = Self::map_to_relative_point(
            &CoordinatedPoint::new(
                relative_coordinate_system.clone(),
                circle_center_global + Vec3d::new(0.0, radius, 0.0),
            ),
            &relative_coordinate_system,
            top_left,
            width,
            height,
        )
        .distance_to(&relative_circle_center);
        let relative_bounding_circle = CoordinatedCircle::new(
            relative_circle_center,
            horizontal_relative_radius.max(vertical_relative_radius),
        );
        let relative_center_of_mass = Self::map_to_relative_point(
            &self.mosaic.get_center_of_mass(),
            &relative_coordinate_system,
            top_left,
            width,
            height,
        );
        let relative_area = self.mosaic.get_area() / absolute_area;
        self.cached_relative_data = Some(CachedRelativeData::new(
            relative_bounding_box,
            relative_bounding_circle,
            relative_center_of_mass,
            relative_area,
            self.mosaic.get_average_color(),
        ));
    }

    fn map_to_relative_point(
        point: &CoordinatedPoint,
        relative_coordinate_system: &AnonymizedCoordinateSystem,
        top_left: Vec3d,
        width: f64,
        height: f64,
    ) -> CoordinatedPoint {
        let converted_point = point.convert_to(relative_coordinate_system.clone());
        CoordinatedPoint::new(
            relative_coordinate_system.clone(),
            Vec3d::new(
                (converted_point.get_x() - top_left.x) / width,
                (converted_point.get_y() - top_left.y) / height,
                0.0,
            ),
        )
    }

    fn get_relative_bounding_box(
        &self,
        relative_coordinate_system: AnonymizedCoordinateSystem,
    ) -> CoordinatedRectangle {
        let bounding_box = SliceRectangle::new_from_math_rectangle(
            &self.mosaic.get_bounding_box().to_global_rectangle(),
        );
        let absolute_rectangle = SliceRectangle::new_from_math_rectangle(&self.absolute_rectangle);
        let unit_scale_reference =
            SliceRectangle::new(Vec3d::new(0.0, 0.0, 0.0), Vec3d::new(0.0, 0.0, 0.0));
        let relative_rectangle =
            RelativeRectangle::new_from_rectangles(bounding_box, absolute_rectangle);
        let relative_bounding_box =
            relative_rectangle.multiply_with_rectangle(unit_scale_reference);
        CoordinatedRectangle::new(
            CoordinatedPoint::new(
                relative_coordinate_system.clone(),
                relative_bounding_box.get_top_left(),
            ),
            CoordinatedPoint::new(
                relative_coordinate_system,
                relative_bounding_box.get_bottom_right(),
            ),
        )
    }
}

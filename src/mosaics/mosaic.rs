#![warn(clippy::pedantic)]
#![allow(
    clippy::missing_errors_doc,
    clippy::missing_panics_doc,
    clippy::too_many_lines,
    clippy::unused_async_trait_impl
)]

use crate::{
    math::{CoordinatedCircle, CoordinatedPoint, CoordinatedRectangle},
    slices::{CachedData, SliceMatrix},
};
use rs_math3d::Vec3d;

#[derive(Clone)]
pub(super) struct Mosaic {
    slice_matrix: SliceMatrix,
    cached_data: Option<CachedData>,
}

impl Mosaic {
    pub(super) fn slice_matrix(&self) -> &SliceMatrix {
        &self.slice_matrix
    }
}

impl Mosaic {
    pub fn new(slice_matrix: SliceMatrix) -> Self {
        Self {
            slice_matrix,
            cached_data: None,
        }
    }

    pub fn get_bounding_box(&mut self) -> CoordinatedRectangle {
        self.calculate_cached_data();
        self.cached_data.as_ref().unwrap().get_bounding_box()
    }

    pub fn get_bounding_circle(&mut self) -> CoordinatedCircle {
        self.calculate_cached_data();
        self.cached_data.as_ref().unwrap().get_bounding_circle()
    }

    pub fn get_center_of_mass(&mut self) -> CoordinatedPoint {
        self.calculate_cached_data();
        self.cached_data.as_ref().unwrap().get_center_of_mass()
    }

    pub fn get_area(&mut self) -> f64 {
        self.calculate_cached_data();
        self.cached_data.as_ref().unwrap().get_area()
    }

    pub fn deduce_longest_distance_point(
        &self,
        point: &CoordinatedPoint,
    ) -> Option<CoordinatedPoint> {
        self.slice_matrix.deduce_longest_distance_point(point)
    }

    fn calculate_cached_data(&mut self) {
        if self.cached_data.is_some() {
            return;
        }
        self.cached_data = Some(self.slice_matrix.calculate_cached_data());
    }

    pub(super) fn contains_point(&self, point: &CoordinatedPoint) -> bool {
        // convert the point to the global coordinate system and check if it is contained in the mosaic
        self.slice_matrix.contains_point(point)
    }

    pub(super) fn get_average_color(&mut self) -> Vec3d {
        self.calculate_cached_data();
        self.cached_data.as_ref().unwrap().get_average_color_vec()
    }
}

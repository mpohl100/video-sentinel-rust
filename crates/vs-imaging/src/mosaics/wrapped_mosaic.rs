#![warn(clippy::pedantic)]
#![allow(
    clippy::missing_errors_doc,
    clippy::missing_panics_doc,
    clippy::too_many_lines,
    clippy::unused_async_trait_impl
)]

use crate::{
    math::{CoordinatedCircle, CoordinatedPoint, CoordinatedRectangle},
    slices::SliceMatrix,
};
use rs_math3d::Vec3d;
use std::sync::{Arc, Mutex};

use super::Mosaic;

#[derive(Clone)]
pub struct WrappedMosaic {
    mosaic: Arc<Mutex<Mosaic>>,
}

impl WrappedMosaic {
    #[must_use]
    pub fn new(slice_matrix: SliceMatrix) -> Self {
        Self {
            mosaic: Arc::new(Mutex::new(Mosaic::new(slice_matrix))),
        }
    }

    #[must_use]
    pub fn get_bounding_box(&self) -> CoordinatedRectangle {
        let mut mosaic = self.mosaic.lock().unwrap();
        mosaic.get_bounding_box()
    }

    #[must_use]
    pub fn get_bounding_circle(&self) -> CoordinatedCircle {
        let mut mosaic = self.mosaic.lock().unwrap();
        mosaic.get_bounding_circle()
    }

    #[must_use]
    pub fn get_center_of_mass(&self) -> CoordinatedPoint {
        let mut mosaic = self.mosaic.lock().unwrap();
        mosaic.get_center_of_mass()
    }

    #[must_use]
    pub fn get_area(&self) -> f64 {
        let mut mosaic = self.mosaic.lock().unwrap();
        mosaic.get_area()
    }

    #[must_use]
    pub fn contains_point(&self, point: &CoordinatedPoint) -> bool {
        let mosaic = self.mosaic.lock().unwrap();
        mosaic.contains_point(point)
    }

    #[must_use]
    pub fn deduce_longest_distance_point(
        &self,
        point: &CoordinatedPoint,
    ) -> Option<CoordinatedPoint> {
        let mosaic = self.mosaic.lock().unwrap();
        mosaic.deduce_longest_distance_point(point)
    }
    #[must_use]
    pub fn get_slice_matrix(&self) -> SliceMatrix {
        let mosaic = self.mosaic.lock().unwrap();
        mosaic.slice_matrix().clone()
    }

    #[must_use]
    pub fn get_average_color(&self) -> Vec3d {
        let mut mosaic = self.mosaic.lock().unwrap();
        mosaic.get_average_color()
    }
}

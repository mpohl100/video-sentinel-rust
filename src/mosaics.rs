use crate::{
    math::CoordinatedCircle,
    math::CoordinatedPoint,
    math::CoordinatedRectangle,
    slices::{CachedData, SliceMatrix},
};

use rs_math3d::Vec3d;

use std::sync::{Arc, Mutex};

#[derive(Clone)]
struct Mosaic {
    slice_matrix: SliceMatrix,
    cached_data: Option<CachedData>,
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
        point: CoordinatedPoint,
    ) -> Option<CoordinatedPoint> {
        self.slice_matrix.deduce_longest_distance_point(point)
    }

    fn calculate_cached_data(&mut self) {
        if self.cached_data.is_some() {
            return;
        }
        self.cached_data = Some(self.slice_matrix.calculate_cached_data());
    }

    fn contains_point(&self, point: CoordinatedPoint) -> bool {
        // convert the point to the global coordinate system and check if it is contained in the mosaic
        self.slice_matrix.contains_point(point)
    }

    fn get_average_color(&mut self) -> Vec3d {
        self.calculate_cached_data();
        self.cached_data.as_ref().unwrap().get_average_color_vec()
    }
}

#[derive(Clone)]
pub struct WrappedMosaic {
    mosaic: Arc<Mutex<Mosaic>>,
}

impl WrappedMosaic {
    pub fn new(slice_matrix: SliceMatrix) -> Self {
        Self {
            mosaic: Arc::new(Mutex::new(Mosaic::new(slice_matrix))),
        }
    }

    pub fn get_bounding_box(&self) -> CoordinatedRectangle {
        let mut mosaic = self.mosaic.lock().unwrap();
        mosaic.get_bounding_box()
    }

    pub fn get_bounding_circle(&self) -> CoordinatedCircle {
        let mut mosaic = self.mosaic.lock().unwrap();
        mosaic.get_bounding_circle()
    }

    pub fn get_center_of_mass(&self) -> CoordinatedPoint {
        let mut mosaic = self.mosaic.lock().unwrap();
        mosaic.get_center_of_mass()
    }

    pub fn get_area(&self) -> f64 {
        let mut mosaic = self.mosaic.lock().unwrap();
        mosaic.get_area()
    }

    pub fn contains_point(&self, point: CoordinatedPoint) -> bool {
        let mosaic = self.mosaic.lock().unwrap();
        mosaic.contains_point(point)
    }

    pub fn deduce_longest_distance_point(
        &self,
        point: CoordinatedPoint,
    ) -> Option<CoordinatedPoint> {
        let mosaic = self.mosaic.lock().unwrap();
        mosaic.deduce_longest_distance_point(point)
    }
    pub fn get_slice_matrix(&self) -> SliceMatrix {
        let mosaic = self.mosaic.lock().unwrap();
        mosaic.slice_matrix.clone()
    }

    pub fn get_average_color(&self) -> Vec3d {
        let mut mosaic = self.mosaic.lock().unwrap();
        mosaic.get_average_color()
    }
}

pub fn deduce_mosaics(slice_matrices: Vec<SliceMatrix>) -> Vec<WrappedMosaic> {
    slice_matrices
        .into_iter()
        .map(|slice_matrix| WrappedMosaic::new(slice_matrix))
        .collect()
}

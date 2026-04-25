use crate::slices::{CachedData, SliceMatrix};

use std::sync::{Arc, Mutex};

use rs_math3d::Vec3d;

#[derive(Clone)]
struct Shape {
    slice_matrix: SliceMatrix,
    cached_data: Option<CachedData>,
}

impl Shape {
    pub fn new(slice_matrix: SliceMatrix) -> Self {
        Self {
            slice_matrix,
            cached_data: None,
        }
    }

    pub fn get_bounding_box(&mut self) -> crate::math::Rectangle {
        self.calculate_cached_data();
        self.cached_data.as_ref().unwrap().get_bounding_box()
    }

    pub fn get_bounding_circle(&mut self) -> crate::math::Circle {
        self.calculate_cached_data();
        self.cached_data.as_ref().unwrap().get_bounding_circle()
    }

    pub fn get_center_of_mass(&mut self) -> Vec3d {
        self.calculate_cached_data();
        self.cached_data.as_ref().unwrap().get_center_of_mass()
    }

    fn calculate_cached_data(&mut self) {
        if self.cached_data.is_some() {
            return;
        }
        self.cached_data = Some(self.slice_matrix.calculate_cached_data());
    }
}

#[derive(Clone)]
pub struct WrappedShape {
    shape: Arc<Mutex<Shape>>,
}

impl WrappedShape {
    pub fn new(slice_matrix: SliceMatrix) -> Self {
        Self {
            shape: Arc::new(Mutex::new(Shape::new(slice_matrix))),
        }
    }

    pub fn get_bounding_box(&self) -> crate::math::Rectangle {
        let mut shape = self.shape.lock().unwrap();
        shape.get_bounding_box()
    }

    pub fn get_bounding_circle(&self) -> crate::math::Circle {
        let mut shape = self.shape.lock().unwrap();
        shape.get_bounding_circle()
    }

    pub fn get_center_of_mass(&self) -> Vec3d {
        let mut shape = self.shape.lock().unwrap();
        shape.get_center_of_mass()
    }
}

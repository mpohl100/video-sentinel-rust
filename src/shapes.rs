use crate::{
    math::CoordinatedPoint,
    slices::{CachedData, SliceMatrix},
};

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

    pub fn get_area(&mut self) -> f64 {
        self.calculate_cached_data();
        self.cached_data.as_ref().unwrap().get_area()
    }

    fn calculate_cached_data(&mut self) {
        if self.cached_data.is_some() {
            return;
        }
        self.cached_data = Some(self.slice_matrix.calculate_cached_data());
    }

    fn contains_point(&self, point: CoordinatedPoint) -> bool {
        // convert the point to the global coordinate system and check if it is contained in the shape
        self.slice_matrix.contains_point(point)
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

    pub fn get_area(&self) -> f64 {
        let mut shape = self.shape.lock().unwrap();
        shape.get_area()
    }

    pub fn contains_point(&self, point: CoordinatedPoint) -> bool {
        let shape = self.shape.lock().unwrap();
        shape.contains_point(point)
    }
}

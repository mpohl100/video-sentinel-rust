use crate::slices::{CachedData, SliceMatrix};

use rs_math3d::Vec3d;

#[derive(Clone)]
struct Shape{
    slice_matrix: SliceMatrix,
    cached_data: Option<CachedData>,
}

impl Shape {
    pub fn new(slice_matrix: SliceMatrix) -> Self {
        Self { slice_matrix, cached_data: None }
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

     fn calculate_cached_data(&mut self){
        if self.cached_data.is_some() {
            return;
        }
        self.cached_data = Some(self.slice_matrix.calculate_cached_data());
     }
}


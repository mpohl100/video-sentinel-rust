use std::collections::BTreeMap;

use crate::shapes::WrappedShape;
use crate::slices::Rectangle;

pub struct BucketedShapesPerSection {
    rectangle: Rectangle,
    bucket: BTreeMap<i64, Vec<WrappedShape>>,
    delta: f64,
}

impl BucketedShapesPerSection {
    pub fn new(rectangle: Rectangle, delta: f64) -> Self {
        BucketedShapesPerSection {
            rectangle,
            bucket: BTreeMap::new(),
            delta,
        }
    }

    pub fn add_shape(&mut self, shape: WrappedShape) {
        let bounding_box = Rectangle::new_from_math_rectangle(shape.get_bounding_box());
        if bounding_box.overlaps(&self.rectangle) {
            self.bucket
                .entry(self.get_bucket_key(shape.clone()))
                .or_insert_with(Vec::new)
                .push(shape);
        }
    }

    fn get_bucket_key(&self, shape: WrappedShape) -> i64 {
        let bounding_circle_area = shape.get_bounding_circle().get_area();
        let shape_area = shape.get_area();
        if bounding_circle_area == 0.0 {
            0
        } else {
            ((shape_area / bounding_circle_area) / self.delta).floor() as i64
        }
    }

    pub fn get_potentially_similar_shapes(&self, shape: WrappedShape) -> Vec<WrappedShape> {
        let bucket_key = self.get_bucket_key(shape);
        let mut similar_shapes = Vec::new();
        for key in bucket_key - 1..=bucket_key + 1 {
            if let Some(shapes) = self.bucket.get(&key) {
                similar_shapes.extend(shapes.clone());
            }
        }
        similar_shapes
    }
}

struct BucketedShapes {
    sections: Vec<BucketedShapesPerSection>,
}

impl BucketedShapes {
    pub fn new(rectangles: Vec<Rectangle>, delta: f64) -> Self {
        let sections = rectangles
            .into_iter()
            .map(|rect| BucketedShapesPerSection::new(rect, delta))
            .collect();
        BucketedShapes { sections }
    }

    pub fn add_shape(&mut self, shape: WrappedShape) {
        for section in &mut self.sections {
            section.add_shape(shape.clone());
        }
    }

    pub fn get_potentially_similar_shapes(&self, shape: WrappedShape) -> Vec<WrappedShape> {
        let mut similar_shapes = Vec::new();
        for section in &self.sections {
            similar_shapes.extend(section.get_potentially_similar_shapes(shape.clone()));
        }
        similar_shapes
    }
}

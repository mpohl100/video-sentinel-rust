use std::collections::BTreeMap;

use crate::slices::Rectangle;
use crate::mosaics::WrappedMosaic;

pub struct BucketedMosaicsPerSection {
    rectangle: Rectangle,
    bucket: BTreeMap<i64, Vec<WrappedMosaic>>,
    delta: f64,
}

impl BucketedMosaicsPerSection {
    pub fn new(rectangle: Rectangle, delta: f64) -> Self {
        BucketedMosaicsPerSection {
            rectangle,
            bucket: BTreeMap::new(),
            delta,
        }
    }

    pub fn add_mosaic(&mut self, mosaic: WrappedMosaic) {
        let bounding_box =
            Rectangle::new_from_math_rectangle(mosaic.get_bounding_box());
        if bounding_box.overlaps(&self.rectangle) {
            self.bucket
                .entry(self.get_bucket_key(&mosaic))
                .or_insert_with(Vec::new)
                .push(mosaic);
        }
    }

    fn get_bucket_key(&self, mosaic: &WrappedMosaic) -> i64 {
        let bounding_circle_area = mosaic.get_bounding_circle().get_area();
        let mosaic_area = mosaic.get_area();
        if bounding_circle_area == 0.0 {
            0
        } else {
            ((mosaic_area / bounding_circle_area) / self.delta).floor() as i64
        }
    }

    pub fn get_potentially_similar_mosaics(&self, mosaic: &WrappedMosaic) -> Vec<WrappedMosaic> {
        let bucket_key = self.get_bucket_key(mosaic);
        let mut similar_mosaics = Vec::new();
        for key in bucket_key - 1..=bucket_key + 1 {
            if let Some(mosaics) = self.bucket.get(&key) {
                similar_mosaics.extend(mosaics.clone());
            }
        }
        similar_mosaics
    }
}

pub struct BucketedMosaics {
    sections: Vec<BucketedMosaicsPerSection>,
}

impl BucketedMosaics {
    pub fn new(rectangles: Vec<Rectangle>, delta: f64) -> Self {
        let sections = rectangles
            .into_iter()
            .map(|rect| BucketedMosaicsPerSection::new(rect, delta))
            .collect();
        BucketedMosaics { sections }
    }

    pub fn add_mosaic(&mut self, mosaic: WrappedMosaic) {
        for section in &mut self.sections {
            section.add_mosaic(mosaic.clone());
        }
    }

    pub fn get_potentially_similar_mosaics(&self, mosaic: &WrappedMosaic) -> Vec<WrappedMosaic> {
        let mut similar_mosaics = Vec::new();
        for section in self.get_overlapping_sections(Rectangle::new_from_math_rectangle(
            mosaic.get_bounding_box(),
        )) {
            similar_mosaics.extend(section.get_potentially_similar_mosaics(mosaic));
        }
        similar_mosaics
    }

    fn get_overlapping_sections(&self, bounding_box: Rectangle) -> Vec<&BucketedMosaicsPerSection> {
        self.sections
            .iter()
            .filter(|section| section.rectangle.overlaps(&bounding_box))
            .collect()
    }
}

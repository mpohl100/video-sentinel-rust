use rs_math3d::Vec3d;

use crate::mosaics::WrappedMosaic;
use crate::slices::{ColoredRectangle, Rectangle};
use crate::eye::{calculate_rectangles_of_bucketed_mosaics, ImageDecompositionParams};
use crate::bucketed_mosaics::BucketedMosaics;
use crate::traces::TraceParams;

#[derive(Clone)]
pub struct ReferenceObject {
    mosaics: Vec<WrappedMosaic>
}

impl ReferenceObject {
    pub fn new(mosaics: Vec<WrappedMosaic>) -> Self {
        let mut mosaics = mosaics;
        mosaics.sort_by(|a, b| a.get_bounding_box().get_area().partial_cmp(&b.get_bounding_box().get_area()).unwrap());
        mosaics.reverse();
        ReferenceObject { mosaics }
    }

    pub fn get_mosaics(&self, until_index: usize) -> Vec<WrappedMosaic> {
        self.mosaics[..until_index].to_vec()
    }

    pub fn get_surrounding_bounding_box(&self) -> Rectangle {
        let mut min_x = f64::INFINITY;
        let mut min_y = f64::INFINITY;
        let mut max_x = f64::NEG_INFINITY;
        let mut max_y = f64::NEG_INFINITY;

        for mosaic in &self.mosaics {
            let bounding_box = mosaic.get_bounding_box();
            min_x = min_x.min(bounding_box.get_top_left().x);
            min_y = min_y.min(bounding_box.get_top_left().y);
            max_x = max_x.max(bounding_box.get_bottom_right().x);
            max_y = max_y.max(bounding_box.get_bottom_right().y);
        }
        Rectangle::new(Vec3d::new(min_x, min_y, 0.0), Vec3d::new(max_x, max_y, 0.0))
    }
}

pub struct ObjectDetectionParams {
    image_decomposition_params: ImageDecompositionParams,
    bucket_delta: f64,
    trace_params: TraceParams,
    target_similarity: f64,
}

pub fn detect_objects(reference_object: ReferenceObject, mosaics: Vec<WrappedMosaic>, object_detection_params: ObjectDetectionParams) -> Vec<ColoredRectangle> {
    let rectangles =
        calculate_rectangles_of_bucketed_mosaics(object_detection_params.image_decomposition_params);
    let mut bucketed_mosaics = BucketedMosaics::new(rectangles, object_detection_params.bucket_delta);
    
    
    let result = Vec::new();
    result 
}
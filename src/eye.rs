use crate::slices::{ColoredRectangle, Rectangle};
use crate::slices::{Color};
use crate::mosaics::WrappedMosaic;
use crate::traces::{Trace, TraceParams};
use crate::bucketed_mosaics::BucketedMosaics;

use rs_math3d::Vec3d;

pub struct ImageDecompositionParams {
    width: usize,
    height: usize,
    slice_width: usize,
    slice_height: usize,
}

pub struct EyeParams {
    image_decomposition_params: ImageDecompositionParams,
    bucket_delta: f64,
    trace_params: TraceParams,
    target_similarity: f64,
}

pub fn deduce_rectangles(previous_mosaics: Vec<WrappedMosaic>, next_mosaics: Vec<WrappedMosaic>, eye_params: EyeParams) -> Vec<ColoredRectangle> {
    let rectangles = calculate_rectangles_of_bucketed_mosaics(eye_params.image_decomposition_params);
    let mut bucketed_mosaics = BucketedMosaics::new(rectangles, eye_params.bucket_delta);
    for mosaic in previous_mosaics.into_iter(){
        bucketed_mosaics.add_mosaic(mosaic);
    }
    let mut results = Vec::new();
    for next_mosaic in next_mosaics.into_iter() {
        let potentially_similar_mosaics = bucketed_mosaics.get_potentially_similar_mosaics(&next_mosaic);
        let mut current_color = Color::Red;
        for previous_mosaic in potentially_similar_mosaics.into_iter() {
            if are_mosaics_similar(&previous_mosaic, &next_mosaic, eye_params.trace_params.clone(), eye_params.target_similarity) {
                let color = deduce_color(Rectangle::new_from_math_rectangle(previous_mosaic.get_bounding_box()), Rectangle::new_from_math_rectangle(next_mosaic.get_bounding_box()));
                if current_color != Color::Blue {
                    current_color = color;
                }
            }
        }
        results.push(ColoredRectangle::new(
            Rectangle::new_from_math_rectangle(next_mosaic.get_bounding_box()),
            current_color,
        ));
    }
    results
}

fn deduce_color(previous_bounding_box: Rectangle, next_bounding_box: Rectangle) -> Color {
    match previous_bounding_box.overlaps(&next_bounding_box) {
        true => Color::Blue,
        false => Color::Blue,
    }
}

fn are_mosaics_similar(mosaic1: &WrappedMosaic, mosaic2: &WrappedMosaic, trace_params: TraceParams, target_similarity: f64) -> bool {
    let trace_1 = Trace::new_from_mosaic(mosaic1.clone(), trace_params.clone());
    let trace_2 = Trace::new_from_mosaic(mosaic2.clone(), trace_params.clone());
    let result = trace_1.compare_with(target_similarity, &trace_2);
    result >= target_similarity
}

fn calculate_rectangles_of_bucketed_mosaics(image_decomposition_params: ImageDecompositionParams) -> Vec<Rectangle> {
    let mut rectangles = Vec::new();
    for y in (0..image_decomposition_params.height).step_by(image_decomposition_params.slice_height) {
        for x in (0..image_decomposition_params.width).step_by(image_decomposition_params.slice_width) {
            rectangles.push(Rectangle::new_from_dims(
                Vec3d::new(x as f64, y as f64, 0.0),
                image_decomposition_params.slice_width as f64,
                image_decomposition_params.slice_height as f64,
            ));
        }
    }
    rectangles
}
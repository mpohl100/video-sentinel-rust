use crate::bucketed_mosaics::BucketedMosaics;
use crate::mosaics::WrappedMosaic;
use crate::slices::Color;
use crate::slices::{ColoredRectangle, Rectangle};
use crate::traces::{Trace, TraceParams};

use rs_math3d::Vec3d;

#[derive(Clone, PartialEq)]
pub struct TileParams {
    image_width: usize,
    image_height: usize,
    tile_width: usize,
    tile_height: usize,
}

impl TileParams {
    pub fn new(image_width: usize, image_height: usize, tile_width: usize, tile_height: usize) -> Self {
        TileParams {
            image_width,
            image_height,
            tile_width,
            tile_height,
        }
    }
}

#[derive(Clone, PartialEq)]
pub struct EyeParams {
    pub image_decomposition_params: TileParams,
    pub bucket_delta: f64,
    pub trace_params: TraceParams,
    pub target_similarity: f64,
}

impl EyeParams {
    pub fn new(
        image_decomposition_params: TileParams,
        bucket_delta: f64,
        trace_params: TraceParams,
        target_similarity: f64,
    ) -> Self {
        EyeParams {
            image_decomposition_params,
            bucket_delta,
            trace_params,
            target_similarity,
        }
    }
}

pub fn deduce_bucketed_mosaics(
    mosaics: Vec<WrappedMosaic>,
    image_decomposition_params: TileParams,
    bucket_delta: f64,
) -> BucketedMosaics {
    let rectangles =
        calculate_rectangles_of_bucketed_mosaics(image_decomposition_params);
    let mut bucketed_mosaics = BucketedMosaics::new(rectangles, bucket_delta);
    for mosaic in mosaics.into_iter() {
        bucketed_mosaics.add_mosaic(mosaic);
    }
    bucketed_mosaics
}

pub fn deduce_rectangles(
    previous_bucketed_mosaics: BucketedMosaics,
    next_mosaics: Vec<WrappedMosaic>,
    eye_params: EyeParams,
) -> Vec<ColoredRectangle> {
    let mut results = Vec::new();
    for next_mosaic in next_mosaics.into_iter() {
        let potentially_similar_mosaics =
            previous_bucketed_mosaics.get_potentially_similar_mosaics(&next_mosaic);
        let mut current_color = Color::Red;
        for previous_mosaic in potentially_similar_mosaics.into_iter() {
            if are_mosaics_similar(
                &previous_mosaic,
                &next_mosaic,
                eye_params.trace_params.clone(),
                eye_params.target_similarity,
            ) {
                let color = deduce_color(
                    Rectangle::new_from_math_rectangle(
                        previous_mosaic.get_bounding_box().to_global_rectangle(),
                    ),
                    Rectangle::new_from_math_rectangle(
                        next_mosaic.get_bounding_box().to_global_rectangle(),
                    ),
                );
                if current_color != Color::Blue {
                    current_color = color;
                }
            }
        }
        results.push(ColoredRectangle::new(
            Rectangle::new_from_math_rectangle(
                next_mosaic.get_bounding_box().to_global_rectangle(),
            ),
            current_color,
            vec![next_mosaic.clone()],
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

fn are_mosaics_similar(
    mosaic1: &WrappedMosaic,
    mosaic2: &WrappedMosaic,
    trace_params: TraceParams,
    target_similarity: f64,
) -> bool {
    let trace_1 = Trace::new_from_mosaic(mosaic1.clone(), trace_params.clone());
    let trace_2 = Trace::new_from_mosaic(mosaic2.clone(), trace_params.clone());
    let result = trace_1.compare_with(target_similarity, &trace_2);
    result >= target_similarity
}

pub fn calculate_rectangles_of_bucketed_mosaics(
    image_decomposition_params: TileParams,
) -> Vec<Rectangle> {
    let mut rectangles = Vec::new();
    for y in (0..image_decomposition_params.image_height).step_by(image_decomposition_params.tile_height)
    {
        for x in
            (0..image_decomposition_params.image_width).step_by(image_decomposition_params.tile_width)
        {
            rectangles.push(Rectangle::new_from_dims(
                Vec3d::new(x as f64, y as f64, 0.0),
                image_decomposition_params.tile_width as f64,
                image_decomposition_params.tile_height as f64,
            ));
        }
    }
    rectangles
}

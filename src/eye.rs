use crate::bucketed_mosaics::BucketedMosaics;
use crate::math::Rectangle as MathRectangle;
use crate::mosaics::WrappedMosaic;
use crate::mosaics::WrappedRelativeMosaic;
use crate::slices::Color;
use crate::slices::{ColoredRectangle, Rectangle, RelativeRectangle, WrappedRelativeRectangle};
use crate::traces::{Trace, TraceParams};

use rs_math3d::Vec3d;

#[derive(Clone, PartialEq)]
pub struct TileParams {
    relative_tile_x: f64,
    relative_tile_y: f64,
}

impl TileParams {
    pub fn new(relative_tile_x: f64, relative_tile_y: f64) -> Self {
        assert!(relative_tile_x > 0.0, "relative_tile_x must be positive");
        assert!(relative_tile_y > 0.0, "relative_tile_y must be positive");
        assert!(relative_tile_x <= 1.0, "relative_tile_x must be <= 1.0");
        assert!(relative_tile_y <= 1.0, "relative_tile_y must be <= 1.0");
        TileParams {
            relative_tile_x,
            relative_tile_y,
        }
    }
}

#[derive(Clone, PartialEq)]
pub struct EyeParams {
    pub tile_params: TileParams,
    pub bucket_delta: f64,
    pub trace_params: TraceParams,
    pub target_similarity: f64,
}

impl EyeParams {
    pub fn new(
        tile_params: TileParams,
        bucket_delta: f64,
        trace_params: TraceParams,
        target_similarity: f64,
    ) -> Self {
        EyeParams {
            tile_params,
            bucket_delta,
            trace_params,
            target_similarity,
        }
    }
}

pub fn deduce_bucketed_mosaics(
    mosaics: Vec<WrappedMosaic>,
    surrounding_rectangle: Rectangle,
    tile_params: TileParams,
    bucket_delta: f64,
) -> BucketedMosaics {
    let rectangles = calculate_rectangles_of_bucketed_mosaics(tile_params);
    let mut bucketed_mosaics = BucketedMosaics::new(rectangles, bucket_delta);
    let absolute_rectangle = MathRectangle::new(
        surrounding_rectangle.get_top_left(),
        surrounding_rectangle.get_bottom_right(),
    );
    for mosaic in mosaics.into_iter() {
        bucketed_mosaics.add_mosaic(WrappedRelativeMosaic::new(
            mosaic,
            absolute_rectangle.clone(),
        ));
    }
    bucketed_mosaics
}

pub fn deduce_rectangles(
    previous_bucketed_mosaics: BucketedMosaics,
    next_mosaics: Vec<WrappedMosaic>,
    eye_params: EyeParams,
    surrounding_rectangle: Rectangle,
) -> Vec<ColoredRectangle> {
    let mut results = Vec::new();
    let absolute_rectangle = MathRectangle::new(
        surrounding_rectangle.get_top_left(),
        surrounding_rectangle.get_bottom_right(),
    );
    for next_mosaic in next_mosaics.into_iter() {
        let wrapped_next_mosaic =
            WrappedRelativeMosaic::new(next_mosaic.clone(), absolute_rectangle.clone());
        let potentially_similar_mosaics = previous_bucketed_mosaics
            .get_potentially_similar_mosaics(&wrapped_next_mosaic)
            .into_iter()
            .map(|wrapped_relative_mosaic| wrapped_relative_mosaic.get_mosaic())
            .collect::<Vec<_>>();
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
    tile_params: TileParams,
) -> Vec<WrappedRelativeRectangle> {
    let mut rectangles = Vec::new();
    // Rectangle width/height are inclusive (+1), so identical points represent a unit scale.
    let unit_scale_reference = Rectangle::new(Vec3d::new(0.0, 0.0, 0.0), Vec3d::new(0.0, 0.0, 0.0));
    let mut y = 0.0;
    while y < 1.0 {
        let mut x = 0.0;
        while x < 1.0 {
            let rectangle = Rectangle::new_from_dims(
                Vec3d::new(x, y, 0.0),
                tile_params.relative_tile_x.min(1.0 - x),
                tile_params.relative_tile_y.min(1.0 - y),
            );
            rectangles.push(WrappedRelativeRectangle::new(
                RelativeRectangle::new_from_rectangles(rectangle, unit_scale_reference.clone()),
            ));
            x += tile_params.relative_tile_x;
        }
        y += tile_params.relative_tile_y;
    }
    rectangles
}

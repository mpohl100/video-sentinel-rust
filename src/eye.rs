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

    pub fn relative_tile_x(&self) -> f64 {
        self.relative_tile_x
    }

    pub fn relative_tile_y(&self) -> f64 {
        self.relative_tile_y
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::math::{CoordinatedPoint, WrappedCoordinateSystem};
    use crate::mosaics::WrappedMosaic;
    use crate::slices::{AnnotatedSlice, Slice, SliceLine, SliceMatrix, WrappedRgbImage};
    use image::{ImageBuffer, Rgb};

    const EPSILON: f64 = 1e-8;

    fn assert_float_eq(actual: f64, expected: f64) {
        assert!(
            (actual - expected).abs() <= EPSILON,
            "expected {expected}, got {actual}"
        );
    }

    fn global_coordinate_system() -> WrappedCoordinateSystem {
        WrappedCoordinateSystem::new(
            Vec3d::new(0.0, 0.0, 0.0),
            Vec3d::new(1.0, 0.0, 0.0),
            Vec3d::new(0.0, 1.0, 0.0),
        )
    }

    fn point(x: f64, y: f64) -> CoordinatedPoint {
        CoordinatedPoint::new(global_coordinate_system(), Vec3d::new(x, y, 0.0))
    }

    fn annotated_slice(x1: f64, y: f64, x2: f64, line_number: usize) -> AnnotatedSlice {
        AnnotatedSlice::new(Slice::new(point(x1, y), point(x2, y)), line_number)
    }

    fn mosaic_from_ranges(ranges: &[(usize, f64, f64)], color: [u8; 3]) -> WrappedMosaic {
        let image = WrappedRgbImage::new(ImageBuffer::from_pixel(32, 32, Rgb(color)));
        let mut matrix = SliceMatrix::new(image);
        for (line_number, start, end) in ranges {
            matrix.add(SliceLine::new(
                *line_number,
                vec![annotated_slice(*start, *line_number as f64, *end, *line_number)],
            ));
        }
        WrappedMosaic::new(matrix)
    }

    fn sample_surrounding_rectangle() -> Rectangle {
        Rectangle::new(Vec3d::new(0.0, 0.0, 0.0), Vec3d::new(20.0, 20.0, 0.0))
    }

    #[test]
    fn tile_params_and_eye_params_store_constructor_values() {
        let tile_params = TileParams::new(0.25, 0.5);
        let eye_params = EyeParams::new(tile_params.clone(), 0.2, TraceParams::new(18, 0.3), 0.85);

        assert_float_eq(tile_params.relative_tile_x(), 0.25);
        assert_float_eq(tile_params.relative_tile_y(), 0.5);
        assert!(eye_params.tile_params == tile_params);
        assert_float_eq(eye_params.bucket_delta, 0.2);
        assert_eq!(eye_params.trace_params.num_skeleton(), 18);
        assert_float_eq(eye_params.trace_params.close_slice_threshold(), 0.3);
        assert_float_eq(eye_params.target_similarity, 0.85);
    }

    #[test]
    #[should_panic(expected = "relative_tile_x must be positive")]
    fn tile_params_reject_non_positive_x() {
        let _ = TileParams::new(0.0, 0.5);
    }

    #[test]
    #[should_panic(expected = "relative_tile_y must be <= 1.0")]
    fn tile_params_reject_too_large_y() {
        let _ = TileParams::new(0.5, 1.5);
    }

    #[test]
    fn calculate_rectangles_of_bucketed_mosaics_returns_grid_count() {
        let rectangles = calculate_rectangles_of_bucketed_mosaics(TileParams::new(0.5, 0.5));

        assert_eq!(rectangles.len(), 4);
    }

    #[test]
    fn deduce_bucketed_mosaics_makes_inserted_mosaic_retrievable() {
        let surrounding_rectangle = sample_surrounding_rectangle();
        let mosaic = mosaic_from_ranges(&[(3, 3.0, 5.0), (4, 3.0, 5.0), (5, 3.0, 5.0)], [40, 50, 60]);
        let bucketed = deduce_bucketed_mosaics(
            vec![mosaic.clone()],
            surrounding_rectangle.clone(),
            TileParams::new(0.5, 0.5),
            0.25,
        );
        let wrapped = WrappedRelativeMosaic::new(
            mosaic,
            MathRectangle::new(
                surrounding_rectangle.get_top_left(),
                surrounding_rectangle.get_bottom_right(),
            ),
        );

        let similar = bucketed.get_all_similar_mosaics(&wrapped);

        assert_eq!(similar.len(), 1);
        assert_float_eq(similar[0].get_area(), wrapped.get_area());
        assert_float_eq(
            similar[0].get_bounding_box().to_global_rectangle().get_area(),
            wrapped.get_bounding_box().to_global_rectangle().get_area(),
        );
    }

    #[test]
    fn deduce_rectangles_marks_identical_mosaics_blue() {
        let surrounding_rectangle = sample_surrounding_rectangle();
        let previous = mosaic_from_ranges(&[(3, 3.0, 5.0), (4, 3.0, 5.0), (5, 3.0, 5.0)], [70, 80, 90]);
        let next = previous.clone();
        let bucketed = deduce_bucketed_mosaics(
            vec![previous],
            surrounding_rectangle.clone(),
            TileParams::new(0.5, 0.5),
            0.25,
        );

        let rectangles = deduce_rectangles(
            bucketed,
            vec![next.clone()],
            EyeParams::new(TileParams::new(0.5, 0.5), 0.25, TraceParams::new(12, 0.2), 0.9),
            surrounding_rectangle,
        );

        assert_eq!(rectangles.len(), 1);
        assert!(rectangles[0].get_color() == Color::Blue);
        assert_float_eq(
            rectangles[0].get_rectangle().get_area(),
            Rectangle::new_from_math_rectangle(next.get_bounding_box().to_global_rectangle()).get_area(),
        );
    }

    #[test]
    fn deduce_rectangles_keeps_red_when_similarity_threshold_is_unreachable() {
        let surrounding_rectangle = sample_surrounding_rectangle();
        let previous = mosaic_from_ranges(&[(3, 3.0, 5.0), (4, 3.0, 5.0), (5, 3.0, 5.0)], [120, 20, 20]);
        let next = mosaic_from_ranges(&[(8, 8.0, 10.0), (9, 8.0, 10.0), (10, 8.0, 10.0)], [120, 20, 20]);
        let bucketed = deduce_bucketed_mosaics(
            vec![previous],
            surrounding_rectangle.clone(),
            TileParams::new(0.5, 0.5),
            0.25,
        );

        let rectangles = deduce_rectangles(
            bucketed,
            vec![next],
            EyeParams::new(TileParams::new(0.5, 0.5), 0.25, TraceParams::new(12, 0.2), 1.1),
            surrounding_rectangle,
        );

        assert_eq!(rectangles.len(), 1);
        assert!(rectangles[0].get_color() == Color::Red);
    }

    #[test]
    fn deduce_color_currently_returns_blue_for_both_overlap_cases() {
        let overlapping = deduce_color(
            Rectangle::new(Vec3d::new(0.0, 0.0, 0.0), Vec3d::new(4.0, 4.0, 0.0)),
            Rectangle::new(Vec3d::new(2.0, 2.0, 0.0), Vec3d::new(6.0, 6.0, 0.0)),
        );
        let disjoint = deduce_color(
            Rectangle::new(Vec3d::new(0.0, 0.0, 0.0), Vec3d::new(4.0, 4.0, 0.0)),
            Rectangle::new(Vec3d::new(10.0, 10.0, 0.0), Vec3d::new(12.0, 12.0, 0.0)),
        );

        assert!(overlapping == Color::Blue);
        assert!(disjoint == Color::Blue);
    }

    #[test]
    fn are_mosaics_similar_matches_trace_comparison_behavior() {
        let mosaic = mosaic_from_ranges(&[(2, 2.0, 4.0), (3, 2.0, 4.0), (4, 2.0, 4.0)], [30, 30, 30]);

        assert!(are_mosaics_similar(&mosaic, &mosaic, TraceParams::new(16, 0.2), 0.9));
        assert!(!are_mosaics_similar(&mosaic, &mosaic, TraceParams::new(16, 0.2), 1.1));
    }
}

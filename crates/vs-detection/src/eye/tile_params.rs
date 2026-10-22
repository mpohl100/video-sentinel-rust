#![warn(clippy::pedantic)]
#![allow(
    clippy::missing_errors_doc,
    clippy::missing_panics_doc,
    clippy::too_many_lines,
    clippy::unused_async_trait_impl
)]

pub use crate::bucketed_mosaics::TileParams;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::eye::funcs::are_mosaics_similar;
    use crate::eye::funcs::deduce_color;

    use crate::eye::*;
    use crate::math::CoordinateSystem;
    use crate::math::Rectangle as MathRectangle;
    use crate::math::{AnonymizedCoordinateSystem, CoordinatedPoint};
    use crate::mosaics::AnonymizedMosaic;
    use crate::mosaics::WrappedRelativeMosaic;
    use crate::mosaics::{Results, WrappedMosaic};
    use crate::slices::Color;
    use crate::slices::Rectangle;
    use crate::slices::{AnnotatedSlice, Slice, SliceLine, SliceMatrix, WrappedRgbImage};
    use crate::traced_mosaics::TracedRelativeMosaic;
    use crate::traces::TraceParams;
    use rs_math3d::Vec3d;

    const EPSILON: f64 = 1e-8;

    fn assert_float_eq(actual: f64, expected: f64) {
        assert!(
            (actual - expected).abs() <= EPSILON,
            "expected {expected}, got {actual}"
        );
    }

    fn global_coordinate_system() -> AnonymizedCoordinateSystem {
        AnonymizedCoordinateSystem::Direct(CoordinateSystem::new(
            Vec3d::new(0.0, 0.0, 0.0),
            Vec3d::new(1.0, 0.0, 0.0),
            Vec3d::new(0.0, 1.0, 0.0),
        ))
    }

    fn point(x: f64, y: f64) -> CoordinatedPoint {
        CoordinatedPoint::new(global_coordinate_system(), Vec3d::new(x, y, 0.0))
    }

    fn annotated_slice(x1: f64, y: f64, x2: f64, line_number: usize) -> AnnotatedSlice {
        AnnotatedSlice::new(Slice::new(&point(x1, y), &point(x2, y)), line_number)
    }

    fn mosaic_from_ranges(ranges: &[(usize, f64, f64)], color: [u8; 3]) -> AnonymizedMosaic {
        let image = WrappedRgbImage::new_with_color(32, 32, color);
        let mut matrix = SliceMatrix::new(image);
        for (line_number, start, end) in ranges {
            matrix.add(SliceLine::new(
                *line_number,
                vec![annotated_slice(
                    *start,
                    f64_from_usize(*line_number),
                    *end,
                    *line_number,
                )],
            ));
        }
        AnonymizedMosaic::new(Results::Absolute, WrappedMosaic::new(matrix), None)
    }

    fn sample_surrounding_rectangle() -> Rectangle {
        Rectangle::new(Vec3d::new(0.0, 0.0, 0.0), Vec3d::new(20.0, 20.0, 0.0))
    }

    #[test]
    fn tile_params_and_eye_params_store_constructor_values() {
        let tile_params = TileParams::new(0.25, 0.5);
        let eye_params = EyeParams::new(tile_params, 0.2, TraceParams::new(18), 0.85);

        assert_float_eq(tile_params.relative_tile_x(), 0.25);
        assert_float_eq(tile_params.relative_tile_y(), 0.5);
        assert!(eye_params.tile_params == tile_params);
        assert_float_eq(eye_params.bucket_delta, 0.2);
        assert_eq!(eye_params.trace_params.num_skeleton(), 18);
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

    #[tokio::test(flavor = "multi_thread")]
    async fn deduce_bucketed_mosaics_makes_inserted_mosaic_retrievable() {
        let surrounding_rectangle = sample_surrounding_rectangle();
        let mosaic =
            mosaic_from_ranges(&[(3, 3.0, 5.0), (4, 3.0, 5.0), (5, 3.0, 5.0)], [40, 50, 60]);
        let traced_params = TraceParams::new(12);
        let bucketed = deduce_bucketed_mosaics(
            vec![mosaic.clone()],
            surrounding_rectangle,
            TileParams::new(0.5, 0.5),
            0.25,
            traced_params,
        )
        .await
        .expect("deduce_bucketed_mosaics should succeed");
        let wrapped = TracedRelativeMosaic::new(
            WrappedRelativeMosaic::new(
                mosaic.get_mosaic(),
                MathRectangle::new(
                    surrounding_rectangle.get_top_left(),
                    surrounding_rectangle.get_bottom_right(),
                ),
            ),
            traced_params,
        );

        let similar = bucketed.get_all_similar_mosaics(&wrapped);

        assert_eq!(similar.len(), 1);
        assert_float_eq(
            similar[0].get_relative_mosaic().get_area(),
            wrapped.get_relative_mosaic().get_area(),
        );
        assert_float_eq(
            similar[0]
                .get_relative_mosaic()
                .get_bounding_box()
                .to_global_rectangle()
                .get_area(),
            wrapped
                .get_relative_mosaic()
                .get_bounding_box()
                .to_global_rectangle()
                .get_area(),
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn deduce_rectangles_marks_identical_mosaics_blue() {
        let surrounding_rectangle = sample_surrounding_rectangle();
        let previous =
            mosaic_from_ranges(&[(3, 3.0, 5.0), (4, 3.0, 5.0), (5, 3.0, 5.0)], [70, 80, 90]);
        let next = previous.clone();
        let bucketed = deduce_bucketed_mosaics(
            vec![previous],
            surrounding_rectangle,
            TileParams::new(0.5, 0.5),
            0.25,
            TraceParams::new(12),
        )
        .await
        .expect("deduce_bucketed_mosaics should succeed");

        let rectangles = deduce_rectangles(
            bucketed,
            vec![next.clone()],
            EyeParams::new(TileParams::new(0.5, 0.5), 0.25, TraceParams::new(12), 0.9),
            surrounding_rectangle,
        )
        .await
        .expect("deduce_rectangles should succeed");

        assert_eq!(rectangles.len(), 1);
        assert!(rectangles[0].get_color() == Color::Red);
        assert_float_eq(
            rectangles[0].get_rectangle().get_area(),
            Rectangle::new_from_math_rectangle(&next.get_bounding_box().to_global_rectangle())
                .get_area(),
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn deduce_rectangles_keeps_red_when_similarity_threshold_is_unreachable() {
        let surrounding_rectangle = sample_surrounding_rectangle();
        let previous = mosaic_from_ranges(
            &[(3, 3.0, 5.0), (4, 3.0, 5.0), (5, 3.0, 5.0)],
            [120, 20, 20],
        );
        let next = mosaic_from_ranges(
            &[(8, 8.0, 10.0), (9, 8.0, 10.0), (10, 8.0, 10.0)],
            [120, 20, 20],
        );
        let bucketed = deduce_bucketed_mosaics(
            vec![previous],
            surrounding_rectangle,
            TileParams::new(0.5, 0.5),
            0.25,
            TraceParams::new(12),
        )
        .await
        .expect("deduce_bucketed_mosaics should succeed");

        let rectangles = deduce_rectangles(
            bucketed,
            vec![next],
            EyeParams::new(TileParams::new(0.5, 0.5), 0.25, TraceParams::new(12), 1.1),
            surrounding_rectangle,
        )
        .await
        .expect("deduce_rectangles should succeed");

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
        let mosaic =
            mosaic_from_ranges(&[(2, 2.0, 4.0), (3, 2.0, 4.0), (4, 2.0, 4.0)], [30, 30, 30]);

        let trace_params = TraceParams::new(16);
        let traced_relative_mosaic = TracedRelativeMosaic::new(
            WrappedRelativeMosaic::new(
                mosaic.get_mosaic(),
                MathRectangle::new(Vec3d::new(0.0, 0.0, 0.0), Vec3d::new(4.0, 4.0, 0.0)),
            ),
            trace_params,
        );
        let similar_at_point_nine =
            are_mosaics_similar(&traced_relative_mosaic, &traced_relative_mosaic, 0.9);
        let similar_at_one_point_one =
            are_mosaics_similar(&traced_relative_mosaic, &traced_relative_mosaic, 1.1);

        assert!(similar_at_point_nine);
        assert!(!similar_at_one_point_one);
    }
}

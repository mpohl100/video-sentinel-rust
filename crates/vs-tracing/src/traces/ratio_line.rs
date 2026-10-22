#![warn(clippy::pedantic)]
#![allow(
    clippy::missing_errors_doc,
    clippy::missing_panics_doc,
    clippy::too_many_lines,
    clippy::unused_async_trait_impl
)]

use super::PolarSlice;

#[derive(Clone)]
pub(super) struct RatioLine {
    slices: Vec<PolarSlice>,
}

impl RatioLine {
    pub(super) fn new(slices: Vec<PolarSlice>) -> Self {
        Self { slices }
    }

    pub(super) fn slices(&self) -> &Vec<PolarSlice> {
        &self.slices
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::math::AnonymizedCoordinateSystem;
    use crate::math::CoordinateSystem;
    use crate::traces::funcs::combine_close_slices;
    use crate::traces::funcs::compare_lines;
    use crate::traces::funcs::get_overlaps;

    use crate::math::CoordinatedPoint;
    use crate::math::CoordinatedRegionedAngle;
    use crate::math::PolarCoordinates;

    use crate::math::RegionedAngle;
    use crate::mosaics::WrappedMosaic;
    use crate::slices::{AnnotatedSlice, Slice, SliceLine, SliceMatrix, WrappedRgbImage};
    use crate::traces::*;

    use rs_math3d::Vec3d;

    const EPSILON: f64 = 1e-8;
    const TRACE_PROBE_EPSILON: f64 = 1e-4;

    fn assert_float_eq(actual: f64, expected: f64) {
        assert!(
            (actual - expected).abs() <= EPSILON,
            "expected {expected}, got {actual}"
        );
    }

    fn assert_trace_float_eq(actual: f64, expected: f64) {
        assert!(
            (actual - expected).abs() <= TRACE_PROBE_EPSILON,
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

    fn regioned_angle(degrees: f64) -> CoordinatedRegionedAngle {
        CoordinatedRegionedAngle::new(
            global_coordinate_system(),
            RegionedAngle::new(degrees, 0.0, 360.0),
        )
    }

    fn polar(radius: f64, degrees: f64) -> PolarCoordinates {
        PolarCoordinates::new(radius, regioned_angle(degrees))
    }

    fn polar_slice(start_radius: f64, end_radius: f64) -> PolarSlice {
        PolarSlice::new(polar(start_radius, 0.0), polar(end_radius, 0.0))
    }

    fn ratio_line(intervals: &[(f64, f64)]) -> RatioLine {
        RatioLine {
            slices: intervals
                .iter()
                .map(|(start, end)| polar_slice(*start, *end))
                .collect(),
        }
    }

    fn solid_image() -> WrappedRgbImage {
        WrappedRgbImage::new_with_color(64, 64, [255, 255, 255])
    }

    fn blank_image(width: u32, height: u32) -> WrappedRgbImage {
        WrappedRgbImage::new_with_color(width, height, [0, 0, 0])
    }

    fn annotated_slice(x1: f64, y: usize, x2: f64) -> AnnotatedSlice {
        AnnotatedSlice::new(
            Slice::new(&point(x1, f64_from_usize(y)), &point(x2, f64_from_usize(y))),
            y,
        )
    }

    fn mosaic_from_lines(lines: &[(usize, &[(f64, f64)])]) -> WrappedMosaic {
        let mut matrix = SliceMatrix::new(solid_image());
        for (line_number, ranges) in lines {
            let slices = ranges
                .iter()
                .map(|(start, end)| annotated_slice(*start, *line_number, *end))
                .collect();
            matrix.add(SliceLine::new(*line_number, slices));
        }
        WrappedMosaic::new(matrix)
    }

    fn add_horizontal_slice(
        slice_matrix: &mut SliceMatrix,
        line_number: usize,
        start_x: f64,
        end_x: f64,
    ) {
        let slice = Slice::new(
            &point(start_x, f64_from_usize(line_number)),
            &point(end_x, f64_from_usize(line_number)),
        );
        slice_matrix.add(SliceLine::new(
            line_number,
            vec![AnnotatedSlice::new(slice, line_number)],
        ));
    }

    fn rectangle_slice_matrix(
        width: u32,
        height: u32,
        top_left: Vec3d,
        bottom_right: Vec3d,
    ) -> SliceMatrix {
        let mut slice_matrix = SliceMatrix::new(blank_image(width, height));
        let start_x = top_left.x;
        let end_x = bottom_right.x - 1.0;

        for y in usize_from_f64(top_left.y)..usize_from_f64(bottom_right.y) {
            add_horizontal_slice(&mut slice_matrix, y, start_x, end_x);
        }

        slice_matrix
    }

    fn circle_slice_matrix(width: u32, height: u32, center: Vec3d, radius: f64) -> SliceMatrix {
        let mut slice_matrix = SliceMatrix::new(blank_image(width, height));
        let start_y = usize_from_f64((center.y - radius).floor().max(0.0));
        let end_y = usize_from_f64((center.y + radius).ceil().min(f64::from(height)));
        let max_x = f64::from(width.saturating_sub(1));

        for y in start_y..end_y {
            let dy = f64_from_usize(y) - center.y;
            let x_offset = (radius * radius - dy * dy).max(0.0).sqrt();
            let start_x = (center.x - x_offset).ceil().max(0.0);
            let end_x = (center.x + x_offset).floor().min(max_x);
            if start_x <= end_x {
                add_horizontal_slice(&mut slice_matrix, y, start_x, end_x);
            }
        }

        slice_matrix
    }

    fn trace_from_slice_matrices(slice_matrices: Vec<SliceMatrix>, params: TraceParams) -> Trace {
        let mosaics: Vec<_> = slice_matrices.into_iter().map(WrappedMosaic::new).collect();
        Trace::new_from_mosaics(&mosaics, params)
    }

    fn assert_trace_matches_expected_lines(trace: &Trace, expected: &[Option<(f64, f64)>]) {
        assert_eq!(trace.ratio_lines().len(), expected.len());

        for (index, expected_slice) in expected.iter().enumerate() {
            let ratio_line = &trace.ratio_lines()[index];
            match expected_slice {
                Some((start_radius, end_radius)) => {
                    assert_eq!(ratio_line.slices.len(), 1, "line {index}");
                    assert_trace_float_eq(
                        ratio_line.slices[0].get_start().get_radius(),
                        *start_radius,
                    );
                    assert_trace_float_eq(ratio_line.slices[0].get_end().get_radius(), *end_radius);
                }
                None => {
                    assert!(ratio_line.slices.is_empty(), "line {index}");
                }
            }
        }
    }

    fn square_mosaic() -> WrappedMosaic {
        mosaic_from_lines(&[
            (0, &[(0.0, 4.0)]),
            (1, &[(0.0, 4.0)]),
            (2, &[(0.0, 4.0)]),
            (3, &[(0.0, 4.0)]),
            (4, &[(0.0, 4.0)]),
        ])
    }

    fn translated_square_mosaic() -> WrappedMosaic {
        mosaic_from_lines(&[
            (10, &[(20.0, 24.0)]),
            (11, &[(20.0, 24.0)]),
            (12, &[(20.0, 24.0)]),
            (13, &[(20.0, 24.0)]),
            (14, &[(20.0, 24.0)]),
        ])
    }

    fn weighted_center_mosaics() -> Vec<WrappedMosaic> {
        vec![
            mosaic_from_lines(&[(0, &[(0.0, 0.0)])]),
            mosaic_from_lines(&[(2, &[(4.0, 6.0)])]),
        ]
    }

    #[test]
    fn polar_slice_methods_return_constructor_values() {
        let slice = PolarSlice::new(polar(0.25, 45.0), polar(0.75, 135.0));

        assert_float_eq(slice.get_start().get_radius(), 0.25);
        assert_float_eq(slice.get_end().get_radius(), 0.75);
        assert_float_eq(slice.get_start().get_angle().get_angle_degrees(), 45.0);
        assert_float_eq(slice.get_end().get_angle().get_angle_degrees(), 135.0);
    }

    #[test]
    fn ratio_related_types_can_be_constructed_with_expected_values() {
        let ratio = Ratio::new(0.1, 0.9);
        let tagged_ratio = TaggedRatio::new(ratio.clone(), Tag::Empty, Tag::Filled);
        let line = ratio_line(&[(0.1, 0.4), (0.6, 0.8)]);

        assert_float_eq(ratio.from(), 0.1);
        assert_float_eq(ratio.to(), 0.9);
        assert_float_eq(tagged_ratio.ratio().from(), 0.1);
        assert_eq!(tagged_ratio.left_tag(), Tag::Empty);
        assert_eq!(tagged_ratio.right_tag(), Tag::Filled);
        assert_eq!(line.slices.len(), 2);
    }

    #[test]
    fn trace_params_methods_return_constructor_values() {
        let params = TraceParams::new(36);

        assert_eq!(params.num_skeleton(), 36);
    }

    #[test]
    fn dump_details_includes_ratio_line_and_slice_information() {
        let trace = Trace::new(
            vec![ratio_line(&[(0.1, 0.4)]), ratio_line(&[(0.6, 0.8)])],
            0.0,
            0.0,
        );

        let dump = trace.dump_details();

        assert!(dump.contains("Trace { ratio_line_count: 2 }"));
        assert!(dump.contains("ratio_line[0] { slice_count: 1 }"));
        assert!(dump.contains("slice[0] { start_radius: 0.10000000"));
        assert!(dump.contains("end_radius: 0.40000000"));
    }

    #[test]
    fn get_overlaps_splits_intervals_and_marks_membership() {
        let line1 = ratio_line(&[(0.2, 0.4)]);
        let line2 = ratio_line(&[(0.3, 0.5)]);

        let overlaps = get_overlaps(&line1, &line2);

        assert_eq!(overlaps.len(), 5);
        assert_float_eq(overlaps[0].ratio().from(), 0.0);
        assert_float_eq(overlaps[0].ratio().to(), 0.2);
        assert_eq!(overlaps[0].left_tag(), Tag::Empty);
        assert_eq!(overlaps[0].right_tag(), Tag::Empty);
        assert_float_eq(overlaps[2].ratio().from(), 0.3);
        assert_float_eq(overlaps[2].ratio().to(), 0.4);
        assert_eq!(overlaps[2].left_tag(), Tag::Filled);
        assert_eq!(overlaps[2].right_tag(), Tag::Filled);
        assert_float_eq(overlaps[4].ratio().from(), 0.5);
        assert_float_eq(overlaps[4].ratio().to(), 1.0);
    }

    #[test]
    fn compare_lines_handles_empty_identical_and_partial_cases() {
        let empty = RatioLine { slices: Vec::new() };
        let identical_left = ratio_line(&[(0.2, 0.4)]);
        let identical_right = ratio_line(&[(0.2, 0.4)]);
        let partial = ratio_line(&[(0.3, 0.5)]);

        assert_float_eq(compare_lines(&empty, &empty), 1.0);
        assert_float_eq(compare_lines(&empty, &identical_left), 0.0);
        assert_float_eq(compare_lines(&identical_left, &identical_right), 1.0);
        assert_float_eq(
            compare_lines(&identical_left, &partial),
            -0.999_999_999_999_998_9,
        );
    }

    #[test]
    fn ratio_line_similarity_matches_cpp_hundred_percent_case() {
        let ratio_line_1 = ratio_line(&[(0.05, 0.45), (0.55, 0.95)]);
        let ratio_line_2 = ratio_line(&[(0.05, 0.45), (0.55, 0.95)]);

        assert_float_eq(compare_lines(&ratio_line_1, &ratio_line_2), 1.0);
    }

    #[test]
    fn ratio_line_similarity_matches_cpp_ninety_percent_case() {
        let ratio_line_1 = ratio_line(&[(0.05, 0.45), (0.6, 1.0)]);
        let ratio_line_2 = ratio_line(&[(0.05, 0.45), (0.55, 0.95)]);

        assert_float_eq(
            compare_lines(&ratio_line_1, &ratio_line_2),
            0.866_666_666_666_666_7,
        );
    }

    #[test]
    fn ratio_line_similarity_matches_cpp_eighty_percent_case() {
        let ratio_line_1 = ratio_line(&[(0.0, 0.4), (0.6, 1.0)]);
        let ratio_line_2 = ratio_line(&[(0.05, 0.45), (0.55, 0.95)]);

        assert_float_eq(
            compare_lines(&ratio_line_1, &ratio_line_2),
            0.714_285_714_285_714_3,
        );
    }

    #[test]
    fn trace_compare_with_rotates_ratio_lines_to_find_best_alignment() {
        let trace1 = Trace::new(
            vec![
                ratio_line(&[(0.1, 0.2)]),
                ratio_line(&[(0.3, 0.4)]),
                ratio_line(&[(0.5, 0.6)]),
            ],
            0.0,
            0.0,
        );
        let trace2 = Trace::new(
            vec![
                ratio_line(&[(0.3, 0.4)]),
                ratio_line(&[(0.5, 0.6)]),
                ratio_line(&[(0.1, 0.2)]),
            ],
            0.0,
            0.0,
        );

        assert_float_eq(trace1.compare_with(0.99, &trace2), 0.0);
    }

    #[test]
    fn combine_close_slices_merges_adjacent_entries_and_splits_at_none() {
        let slices = vec![
            Some(polar_slice(0.1, 0.2)),
            Some(polar_slice(0.24, 0.3)),
            None,
            Some(polar_slice(0.6, 0.7)),
        ];

        let combined = combine_close_slices(&slices);

        assert_eq!(combined.len(), 2);
        assert_float_eq(combined[0].get_start().get_radius(), 0.1);
        assert_float_eq(combined[0].get_end().get_radius(), 0.3);
        assert_float_eq(combined[1].get_start().get_radius(), 0.6);
        assert_float_eq(combined[1].get_end().get_radius(), 0.7);
    }

    #[test]
    fn calculate_center_of_mass_weights_mosaics_by_area() {
        let mosaics = weighted_center_mosaics();

        let center = calculate_center_of_mass(&mosaics);

        assert_float_eq(center.get_x(), 3.75);
        assert_float_eq(center.get_y(), 1.5);
        assert_float_eq(center.get_z(), 0.0);
    }

    #[test]
    fn deduce_longest_radius_returns_farthest_distance_from_center() {
        let mosaics = weighted_center_mosaics();
        let center = calculate_center_of_mass(&mosaics);

        let radius = deduce_longest_radius(&mosaics, &center);

        assert_float_eq(radius, 4.038_873_605_350_878);
    }

    #[test]
    fn calculate_total_surrounding_circle_area_uses_farthest_point_from_weighted_center() {
        let mosaics = weighted_center_mosaics();

        let area = calculate_total_surrounding_circle_area(&mosaics);

        assert_float_eq(
            area,
            std::f64::consts::PI * 4.038_873_605_350_878 * 4.038_873_605_350_878,
        );
    }

    #[test]
    fn deduce_slices_from_mosaic_produces_ordered_finite_slices() {
        let mosaic = square_mosaic();
        let coordinate_system = AnonymizedCoordinateSystem::Direct(CoordinateSystem::new(
            mosaic.get_center_of_mass().get_local_point(),
            Vec3d::new(1.0, 0.0, 0.0),
            Vec3d::new(0.0, 1.0, 0.0),
        ));

        let slices = deduce_slices_from_mosaic(
            std::slice::from_ref(&mosaic),
            &CoordinatedRegionedAngle::new(coordinate_system, RegionedAngle::new(45.0, 0.0, 360.0)),
            mosaic.get_bounding_circle().get_radius(),
        );

        for slice in slices {
            assert!(slice.get_start().get_radius().is_finite());
            assert!(slice.get_end().get_radius() >= slice.get_start().get_radius());
            assert!(slice.get_end().get_radius().is_finite());
        }
    }

    #[test]
    fn trace_from_slice_matrix_square_matches_probe_slice_counts_and_radii() {
        let trace = trace_from_slice_matrices(
            vec![rectangle_slice_matrix(
                50,
                50,
                Vec3d::new(15.0, 15.0, 0.0),
                Vec3d::new(35.0, 35.0, 0.0),
            )],
            TraceParams::new(36),
        );

        let expected = [
            Some((0.0, 0.741_535_78)),
            Some((0.0, 0.752_975_16)),
            Some((0.0, 0.789_125_89)),
            Some((0.0, 0.856_251_76)),
            Some((0.0, 0.968_006_21)),
            Some((0.0, 0.968_006_21)),
            Some((0.0, 0.856_251_76)),
            Some((0.0, 0.789_125_89)),
            Some((0.0, 0.752_975_16)),
            Some((0.0, 0.741_535_78)),
            Some((0.0, 0.752_975_16)),
            Some((0.0, 0.789_125_89)),
            Some((0.0, 0.856_251_76)),
            Some((0.0, 0.968_006_21)),
            Some((0.0, 0.968_006_21)),
            Some((0.0, 0.856_251_76)),
            Some((0.0, 0.789_125_89)),
            Some((0.0, 0.752_975_16)),
            Some((0.0, 0.741_535_78)),
            Some((0.0, 0.752_975_16)),
            Some((0.0, 0.789_125_89)),
            Some((0.0, 0.856_251_76)),
            Some((0.0, 0.968_006_21)),
            Some((0.0, 0.875_815_14)),
            Some((0.0, 0.774_703_98)),
            Some((0.0, 0.713_971_05)),
            Some((0.0, 0.681_263_24)),
            Some((0.0, 0.670_913_32)),
            Some((0.0, 0.681_263_24)),
            Some((0.0, 0.713_971_05)),
            Some((0.0, 0.774_703_98)),
            Some((0.0, 0.875_815_14)),
            Some((0.0, 0.968_006_21)),
            Some((0.0, 0.856_251_76)),
            Some((0.0, 0.789_125_89)),
            Some((0.0, 0.752_975_16)),
        ];

        assert_trace_matches_expected_lines(&trace, &expected);
    }

    #[test]
    fn trace_from_slice_matrix_circle_matches_probe_slice_counts_and_radii() {
        let trace = trace_from_slice_matrices(
            vec![circle_slice_matrix(
                50,
                50,
                Vec3d::new(25.0, 25.0, 0.0),
                25.0,
            )],
            TraceParams::new(36),
        );

        let expected = [
            Some((0.0, 0.962_872_13)),
            Some((0.0, 0.977_725_98)),
            Some((0.0, 0.983_701_34)),
            Some((0.0, 0.978_477_56)),
            Some((0.0, 1.005_680_49)),
            Some((0.0, 1.005_680_49)),
            Some((0.0, 0.978_477_56)),
            Some((0.0, 0.983_701_34)),
            Some((0.0, 0.977_725_98)),
            Some((0.0, 0.962_872_13)),
            Some((0.000_498_84, 0.977_725_98)),
            Some((0.000_522_79, 0.983_701_34)),
            Some((0.000_567_26, 0.978_477_56)),
            Some((0.000_641_30, 1.005_680_49)),
            Some((0.000_764_27, 1.004_397_90)),
            Some((0.000_982_52, 0.977_343_04)),
            Some((0.001_436_35, 0.982_655_76)),
            Some((0.002_829_06, 0.976_728_30)),
            Some((0.0, 0.961_889_60)),
            Some((0.0, 0.976_728_30)),
            Some((0.0, 0.982_655_76)),
            Some((0.0, 0.977_343_04)),
            Some((0.0, 0.954_145_94)),
            Some((0.0, 0.954_145_94)),
            Some((0.0, 0.922_903_11)),
            Some((0.0, 0.941_689_99)),
            Some((0.0, 0.937_639_22)),
            Some((0.0, 0.961_889_60)),
            Some((0.002_829_06, 0.937_639_22)),
            Some((0.001_436_35, 0.941_689_99)),
            Some((0.000_982_52, 0.924_868_15)),
            Some((0.000_764_27, 0.958_971_53)),
            Some((0.000_641_30, 0.955_428_53)),
            Some((0.000_567_26, 0.978_477_56)),
            Some((0.000_522_79, 0.983_701_34)),
            Some((0.000_498_84, 0.977_725_98)),
        ];

        assert_trace_matches_expected_lines(&trace, &expected);
    }

    #[test]
    fn trace_from_slice_matrix_rectangle_matches_probe_slice_counts_and_radii() {
        let trace = trace_from_slice_matrices(
            vec![rectangle_slice_matrix(
                50,
                50,
                Vec3d::new(15.0, 15.0, 0.0),
                Vec3d::new(25.0, 35.0, 0.0),
            )],
            TraceParams::new(36),
        );

        let expected = [
            Some((0.0, 0.481_456_92)),
            Some((0.0, 0.488_884_17)),
            Some((0.0, 0.512_355_75)),
            Some((0.0, 0.555_938_57)),
            Some((0.0, 0.612_829_64)),
            Some((0.0, 0.742_769_62)),
            Some((0.0, 0.960_257_52)),
            Some((0.0, 0.978_133_71)),
            Some((0.0, 0.933_324_32)),
            Some((0.0, 0.919_145_03)),
            Some((0.0, 0.933_324_32)),
            Some((0.0, 0.978_133_71)),
            Some((0.0, 0.960_257_52)),
            Some((0.0, 0.742_769_62)),
            Some((0.0, 0.612_829_64)),
            Some((0.0, 0.555_938_57)),
            Some((0.0, 0.512_355_75)),
            Some((0.0, 0.488_884_17)),
            Some((0.0, 0.481_456_92)),
            Some((0.0, 0.488_884_17)),
            Some((0.0, 0.512_355_75)),
            Some((0.0, 0.555_938_57)),
            Some((0.0, 0.612_829_64)),
            Some((0.0, 0.742_769_62)),
            Some((0.0, 0.960_257_52)),
            Some((0.0, 0.884_978_12)),
            Some((0.0, 0.844_436_29)),
            Some((0.0, 0.831_607_41)),
            Some((0.0, 0.844_436_29)),
            Some((0.0, 0.884_978_12)),
            Some((0.0, 0.960_257_52)),
            Some((0.0, 0.742_769_62)),
            Some((0.0, 0.612_829_64)),
            Some((0.0, 0.555_938_57)),
            Some((0.0, 0.512_355_75)),
            Some((0.0, 0.488_884_17)),
        ];

        assert_trace_matches_expected_lines(&trace, &expected);
    }

    #[test]
    fn trace_from_slice_matrix_pair_matches_probe_slice_counts_and_radii() {
        let trace = trace_from_slice_matrices(
            vec![
                rectangle_slice_matrix(
                    80,
                    50,
                    Vec3d::new(10.0, 10.0, 0.0),
                    Vec3d::new(30.0, 30.0, 0.0),
                ),
                rectangle_slice_matrix(
                    80,
                    50,
                    Vec3d::new(50.0, 10.0, 0.0),
                    Vec3d::new(70.0, 30.0, 0.0),
                ),
            ],
            TraceParams::new(24),
        );

        let expected = [
            Some((0.303_388_98, 0.974_038_31)),
            Some((0.314_091_39, 1.008_398_67)),
            Some((0.351_292_51, 0.670_649_33)),
            Some((0.429_056_81, 0.474_220_69)),
            None,
            None,
            None,
            None,
            None,
            Some((0.429_056_81, 0.474_220_69)),
            Some((0.351_292_51, 0.670_649_33)),
            Some((0.314_091_39, 1.008_398_67)),
            Some((0.303_388_98, 0.974_038_31)),
            Some((0.314_091_39, 1.008_398_67)),
            Some((0.351_292_51, 0.606_777_97)),
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            Some((0.351_292_51, 0.606_777_97)),
            Some((0.314_091_39, 1.008_398_67)),
        ];

        assert_trace_matches_expected_lines(&trace, &expected);
    }

    #[test]
    fn trace_new_from_mosaic_builds_requested_number_of_ratio_lines() {
        let trace = Trace::new_from_mosaic(&square_mosaic(), TraceParams::new(18));
        let self_similarity = trace.compare_with(0.0, &trace.clone());

        assert_eq!(trace.ratio_lines().len(), 18);
        assert!(
            trace
                .ratio_lines()
                .iter()
                .any(|line| !line.slices.is_empty())
        );
        assert!(self_similarity >= 1.0);
    }

    #[test]
    fn trace_compare_with_returns_zero_when_target_similarity_is_unreachable() {
        let trace = Trace::new_from_mosaic(&square_mosaic(), TraceParams::new(18));
        let self_similarity = trace.compare_with(0.0, &trace.clone());
        let unreachable_similarity = trace.compare_with(self_similarity + EPSILON, &trace.clone());

        assert_float_eq(unreachable_similarity, 1.0);
    }

    #[test]
    fn trace_new_from_mosaics_combines_multiple_mosaics_and_self_matches() {
        let combined_mosaics = vec![square_mosaic(), translated_square_mosaic()];
        let combined = Trace::new_from_mosaics(&combined_mosaics, TraceParams::new(18));
        let same_family_mosaics = vec![translated_square_mosaic(), square_mosaic()];
        let same_family = Trace::new_from_mosaics(&same_family_mosaics, TraceParams::new(18));
        let similarity = combined.compare_with(0.0, &same_family);
        let unreachable_similarity = combined.compare_with(similarity + EPSILON, &same_family);

        assert_eq!(combined.ratio_lines().len(), 18);
        assert!(similarity >= 1.0);
        assert_float_eq(unreachable_similarity, 1.0);
    }
}

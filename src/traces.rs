use crate::math::CoordinatedLine;
use crate::math::CoordinatedPoint;
use crate::math::CoordinatedRectangle;
use crate::math::CoordinatedRegionedAngle;
use crate::math::PolarCoordinates;
use crate::math::Rectangle;
use crate::math::RegionedAngle;
use crate::math::WrappedCoordinateSystem;
use crate::mosaics::WrappedMosaic;

use rs_math3d::Vec3d;

#[derive(Clone)]
struct PolarSlice {
    start: PolarCoordinates,
    end: PolarCoordinates,
}

impl PolarSlice {
    fn new(start: PolarCoordinates, end: PolarCoordinates) -> Self {
        if start.get_radius() < end.get_radius() {
            PolarSlice { start, end }
        } else {
            PolarSlice { start: end, end: start }
        }
    }

    fn get_start(&self) -> &PolarCoordinates {
        &self.start
    }

    fn get_end(&self) -> &PolarCoordinates {
        &self.end
    }
}

#[derive(Clone)]
struct RatioLine {
    slices: Vec<PolarSlice>,
}

#[derive(Clone, PartialEq, PartialOrd)]
pub struct TraceParams {
    num_skeleton: usize,
    close_slice_threshold: f64,
}

impl TraceParams {
    pub fn new(num_skeleton: usize, close_slice_threshold: f64) -> Self {
        TraceParams {
            num_skeleton,
            close_slice_threshold,
        }
    }

    pub fn num_skeleton(&self) -> usize {
        self.num_skeleton
    }

    pub fn close_slice_threshold(&self) -> f64 {
        self.close_slice_threshold
    }
}

#[derive(Clone)]
pub struct Trace {
    ratio_lines: Vec<RatioLine>,
}

impl Trace {
    pub fn new_from_mosaic(mosaic: WrappedMosaic, params: TraceParams) -> Self {
        let global_coordinate_system = WrappedCoordinateSystem::new(
            Vec3d::new(0.0, 0.0, 0.0),
            Vec3d::new(1.0, 0.0, 0.0),
            Vec3d::new(0.0, 1.0, 0.0),
        );
        let ratio_lines = (0..params.num_skeleton)
            .map(|i| {
                let coordinate_system = WrappedCoordinateSystem::new(
                    mosaic
                        .get_center_of_mass()
                        .convert_to(global_coordinate_system.clone())
                        .get_local_point(),
                    Vec3d::new(1.0, 0.0, 0.0),
                    Vec3d::new(0.0, 1.0, 0.0),
                );
                let coordinated_regioned_angle = CoordinatedRegionedAngle::new(
                    coordinate_system,
                    RegionedAngle::new(
                        (i as f64) * (360.0 / params.num_skeleton as f64),
                        0.0,
                        360.0,
                    ),
                );
                RatioLine {
                    slices: deduce_slices_from_mosaic(
                        vec![mosaic.clone()],
                        coordinated_regioned_angle.clone(),
                        mosaic.get_bounding_circle().get_radius(),
                        &params,
                    ),
                }
            })
            .collect();
        Trace { ratio_lines }
    }

    pub fn new_from_mosaics(mosaics: Vec<WrappedMosaic>, params: TraceParams) -> Self {
        let global_coordinate_system = WrappedCoordinateSystem::new(
            Vec3d::new(0.0, 0.0, 0.0),
            Vec3d::new(1.0, 0.0, 0.0),
            Vec3d::new(0.0, 1.0, 0.0),
        );
        let center_of_mass =
            calculate_center_of_mass(&mosaics).convert_to(global_coordinate_system.clone());
        let ratio_lines = (0..params.num_skeleton)
            .map(|i| {
                let coordinate_system = WrappedCoordinateSystem::new(
                    center_of_mass.clone().get_local_point(),
                    Vec3d::new(1.0, 0.0, 0.0),
                    Vec3d::new(0.0, 1.0, 0.0),
                );
                let coordinated_regioned_angle = CoordinatedRegionedAngle::new(
                    coordinate_system,
                    RegionedAngle::new(
                        (i as f64) * (360.0 / params.num_skeleton as f64),
                        0.0,
                        360.0,
                    ),
                );
                RatioLine {
                    slices: deduce_slices_from_mosaic(
                        mosaics.clone(),
                        coordinated_regioned_angle.clone(),
                        deduce_longest_radius(&mosaics, center_of_mass.clone()),
                        &params,
                    ),
                }
            })
            .collect();
        Trace { ratio_lines }
    }

    pub fn compare_with(&self, target_similarity: f64, other: &Trace) -> f64 {
        for i in 0..self.ratio_lines.len() {
            let mut second_ratio_lines = other.ratio_lines.clone();
            second_ratio_lines.rotate_right(i);
            let similarity = compare_with(&self.ratio_lines, &second_ratio_lines);
            if similarity >= target_similarity {
                return similarity;
            }
        }
        0.0
    }

    pub fn dump_details(&self) -> String {
        let mut output = String::new();
        output.push_str(&format!(
            "Trace {{ ratio_line_count: {} }}\n",
            self.ratio_lines.len()
        ));

        for (line_index, ratio_line) in self.ratio_lines.iter().enumerate() {
            output.push_str(&format!(
                "  ratio_line[{line_index}] {{ slice_count: {} }}\n",
                ratio_line.slices.len()
            ));

            for (slice_index, slice) in ratio_line.slices.iter().enumerate() {
                let start = slice.get_start();
                let end = slice.get_end();
                let start_cartesian = start.to_cartesian();
                let end_cartesian = end.to_cartesian();

                output.push_str(&format!(
                    "    slice[{slice_index}] {{ start_radius: {:.8}, start_angle_degrees: {:.8}, end_radius: {:.8}, end_angle_degrees: {:.8}, start_cartesian: ({:.8}, {:.8}, {:.8}), end_cartesian: ({:.8}, {:.8}, {:.8}) }}\n",
                    start.get_radius(),
                    start.get_angle().get_angle_degrees(),
                    end.get_radius(),
                    end.get_angle().get_angle_degrees(),
                    start_cartesian.get_x(),
                    start_cartesian.get_y(),
                    start_cartesian.get_z(),
                    end_cartesian.get_x(),
                    end_cartesian.get_y(),
                    end_cartesian.get_z(),
                ));
            }
        }

        output
    }
}

fn compare_with(first_ratio_lines: &[RatioLine], second_ratio_lines: &[RatioLine]) -> f64 {
    let mut total_similarity = 0.0;
    for (line1, line2) in first_ratio_lines.iter().zip(second_ratio_lines.iter()) {
        let similarity = compare_lines(line1, line2);
        total_similarity += similarity;
    }
    total_similarity / first_ratio_lines.len() as f64
}

fn compare_lines(line1: &RatioLine, line2: &RatioLine) -> f64 {
    if line1.slices.is_empty() && line2.slices.is_empty() {
        return 1.0;
    }
    if line1.slices.is_empty() || line2.slices.is_empty() {
        return 0.0;
    }

    let overlaps = get_overlaps(line1, line2);
    // convert the following code to rust
    let mut filtered_overlaps: Vec<TaggedRatio> = overlaps
        .into_iter()
        .filter(|tr| (tr.left_tag + tr.right_tag) != 1)
        .collect();
    filtered_overlaps.sort_by(|lhs, rhs| rhs.ratio.from.partial_cmp(&lhs.ratio.from).unwrap());
    let left_quantile_index = 2 * line1.slices.len() + 1;
    let right_quantile_index = 2 * line2.slices.len() + 1;
    let quantile_index = std::cmp::max(left_quantile_index, right_quantile_index) + 1;
    let n = std::cmp::min(filtered_overlaps.len(), quantile_index);
    let mut similarity = 0.0;
    for item in filtered_overlaps.iter().take(n) {
        similarity += item.ratio.to - item.ratio.from;
    }
    similarity
}

#[derive(Clone)]
struct Ratio {
    from: f64,
    to: f64,
}

#[derive(Clone)]
struct TaggedRatio {
    ratio: Ratio,
    left_tag: usize,
    right_tag: usize,
}

fn get_overlaps(line1: &RatioLine, line2: &RatioLine) -> Vec<TaggedRatio> {
    // convert the following code to rust
    let mut overlaps: Vec<TaggedRatio> = Vec::new();
    let mut interesting_points: Vec<f64> = Vec::new();
    interesting_points.push(0.0);
    interesting_points.push(1.0);
    for polar_slice in &line1.slices {
        interesting_points.push(polar_slice.get_start().get_radius());
        interesting_points.push(polar_slice.get_end().get_radius());
    }
    for polar_slice in &line2.slices {
        interesting_points.push(polar_slice.get_start().get_radius());
        interesting_points.push(polar_slice.get_end().get_radius());
    }
    interesting_points.sort_by(|a, b| a.partial_cmp(b).unwrap());
    for i in 0..interesting_points.len() - 1 {
        let from = interesting_points[i];
        let to = interesting_points[i + 1];
        if from == to {
            continue; // skip zero-length intervals
        }
        let current_midpoint = (from + to) / 2.0;
        let pred = |polar_ratio: &PolarSlice| {
            polar_ratio.get_start().get_radius() <= current_midpoint
                && polar_ratio.get_end().get_radius() >= current_midpoint
        };
        let lit = line1.slices.iter().find(|&ratio| pred(ratio));
        let rit = line2.slices.iter().find(|&ratio| pred(ratio));
        let mut left_tag = 1;
        let mut right_tag = 1;
        if lit.is_some() {
            left_tag = 0;
        }
        if rit.is_some() {
            right_tag = 0;
        }
        overlaps.push(TaggedRatio {
            ratio: Ratio { from, to },
            left_tag,
            right_tag,
        });
    }
    overlaps
}

fn deduce_slices_from_mosaic(
    mosaics: Vec<WrappedMosaic>,
    coordinated_regioned_angle: CoordinatedRegionedAngle,
    radius: f64,
    params: &TraceParams,
) -> Vec<PolarSlice> {
    let mut slices = Vec::new();
    // for every x in the range of -radius to radius with a step of 0.5, find the intersections with the mosaic and create slices
    let step = 0.5;
    let mut x = 0.0;
    while x <= radius {
        let global_coordinate_system = WrappedCoordinateSystem::new(
            Vec3d::new(0.0, 0.0, 0.0),
            Vec3d::new(1.0, 0.0, 0.0),
            Vec3d::new(0.0, 1.0, 0.0),
        );
        let current_polar_coordinates =
            PolarCoordinates::new(x, coordinated_regioned_angle.clone());
        let point = current_polar_coordinates.to_cartesian();
        let contains_point = mosaics
            .iter()
            .any(|mosaic| mosaic.contains_point(point.clone()));
        if contains_point {
            let global_point = point.convert_to(global_coordinate_system.clone());
            let tl = Vec3d::new(
                global_point.get_x(),
                global_point.get_y(),
                0.0,
            );
            let br = Vec3d::new(
                global_point.get_x() + 1.0,
                global_point.get_y() + 1.0,
                0.0,
            );
            let rectangle = Rectangle::new(tl, br);
            let coordinated_rectangle =
                CoordinatedRectangle::new_from_rectangle(rectangle, global_coordinate_system);
            let line_coordinate_system = coordinated_regioned_angle.get_coordinate_system().duplicate();
            line_coordinate_system.rotate(coordinated_regioned_angle.get_regioned_angle());
            let x_line_start = CoordinatedPoint::new(
                line_coordinate_system.clone(),
                Vec3d::new(0.0, 0.0, 0.0),
            );
            let x_line_end =
                CoordinatedPoint::new(line_coordinate_system.clone(), Vec3d::new(1.5*radius, 0.0, 0.0));
            let x_axis_line = CoordinatedLine::new(x_line_start, x_line_end);
            let clipped_line = coordinated_rectangle.get_intersection_line(x_axis_line);
            if let Some(clipped_line) = clipped_line {
                let polar_start = PolarCoordinates::new(
                    clipped_line.get_start().get_x() / radius,
                    coordinated_regioned_angle.clone(),
                );
                let polar_end = PolarCoordinates::new(
                    clipped_line.get_end().get_x() / radius,
                    coordinated_regioned_angle.clone(),
                );
                // check that the absoulte of the y coordinates is below 1e-4
                assert!(clipped_line.get_start().get_y().abs() < 1e-4);
                assert!(clipped_line.get_end().get_y().abs() < 1e-4);
                
                let slice = PolarSlice::new(polar_start, polar_end);
                slices.push(slice);
            }
        }
        x += step;
    }
    combine_close_slices(slices, params.close_slice_threshold)
}

fn combine_close_slices(slices: Vec<PolarSlice>, threshold: f64) -> Vec<PolarSlice> {
    if slices.is_empty() {
        return slices;
    }
    let mut combined_slices = Vec::new();
    let mut current_slice = slices[0].clone();
    for slice in slices.iter().skip(1) {
        if slice.get_start().get_radius() - current_slice.get_end().get_radius() <= threshold {
            current_slice =
                PolarSlice::new(current_slice.get_start().clone(), slice.get_end().clone());
        } else {
            combined_slices.push(current_slice);
            current_slice = slice.clone();
        }
    }
    combined_slices.push(current_slice);
    combined_slices
}

fn calculate_center_of_mass(mosaics: &[WrappedMosaic]) -> CoordinatedPoint {
    let mut total_mass = 0.0;
    let mut center_of_mass = Vec3d::new(0.0, 0.0, 0.0);
    for mosaic in mosaics {
        let mass = mosaic.get_area();
        let global_coordinate_system = WrappedCoordinateSystem::new(
            Vec3d::new(0.0, 0.0, 0.0),
            Vec3d::new(1.0, 0.0, 0.0),
            Vec3d::new(0.0, 1.0, 0.0),
        );
        let mosaic_center = mosaic
            .get_center_of_mass()
            .convert_to(global_coordinate_system.clone());
        center_of_mass.x += mosaic_center.get_x() * mass;
        center_of_mass.y += mosaic_center.get_y() * mass;
        center_of_mass.z += mosaic_center.get_z() * mass;
        total_mass += mass;
    }
    if total_mass > 0.0 {
        center_of_mass.x /= total_mass;
        center_of_mass.y /= total_mass;
        center_of_mass.z /= total_mass;
    }
    let global_coordinate_system = WrappedCoordinateSystem::new(
        Vec3d::new(0.0, 0.0, 0.0),
        Vec3d::new(1.0, 0.0, 0.0),
        Vec3d::new(0.0, 1.0, 0.0),
    );
    CoordinatedPoint::new(global_coordinate_system, center_of_mass)
}

fn deduce_longest_radius(mosaics: &[WrappedMosaic], center_of_mass: CoordinatedPoint) -> f64 {
    let mut longest_radius = 0.0;
    for mosaic in mosaics {
        let mosaic_longest_distance = mosaic.deduce_longest_distance_point(center_of_mass.clone());
        if let Some(mosaic_longest_distance) = mosaic_longest_distance {
            let distance = mosaic_longest_distance.distance_to(center_of_mass.clone());
            if distance > longest_radius {
                longest_radius = distance;
            }
        }
    }
    longest_radius
}

#[cfg(test)]
mod tests {
    use super::*;
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
        WrappedRgbImage::new(ImageBuffer::from_pixel(64, 64, Rgb([255, 255, 255])))
    }

    fn annotated_slice(x1: f64, y: usize, x2: f64) -> AnnotatedSlice {
        AnnotatedSlice::new(Slice::new(point(x1, y as f64), point(x2, y as f64)), y)
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
        let ratio = Ratio { from: 0.1, to: 0.9 };
        let tagged_ratio = TaggedRatio {
            ratio: ratio.clone(),
            left_tag: 0,
            right_tag: 1,
        };
        let line = ratio_line(&[(0.1, 0.4), (0.6, 0.8)]);

        assert_float_eq(ratio.from, 0.1);
        assert_float_eq(ratio.to, 0.9);
        assert_float_eq(tagged_ratio.ratio.from, 0.1);
        assert_eq!(tagged_ratio.left_tag, 0);
        assert_eq!(tagged_ratio.right_tag, 1);
        assert_eq!(line.slices.len(), 2);
    }

    #[test]
    fn trace_params_methods_return_constructor_values() {
        let params = TraceParams::new(36, 0.2);

        assert_eq!(params.num_skeleton(), 36);
        assert_float_eq(params.close_slice_threshold(), 0.2);
    }

    #[test]
    fn dump_details_includes_ratio_line_and_slice_information() {
        let trace = Trace {
            ratio_lines: vec![ratio_line(&[(0.1, 0.4)]), ratio_line(&[(0.6, 0.8)])],
        };

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
        assert_float_eq(overlaps[0].ratio.from, 0.0);
        assert_float_eq(overlaps[0].ratio.to, 0.2);
        assert_eq!(overlaps[0].left_tag, 1);
        assert_eq!(overlaps[0].right_tag, 1);
        assert_float_eq(overlaps[2].ratio.from, 0.3);
        assert_float_eq(overlaps[2].ratio.to, 0.4);
        assert_eq!(overlaps[2].left_tag, 0);
        assert_eq!(overlaps[2].right_tag, 0);
        assert_float_eq(overlaps[4].ratio.from, 0.5);
        assert_float_eq(overlaps[4].ratio.to, 1.0);
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
        assert_float_eq(compare_lines(&identical_left, &partial), 0.8);
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

        assert_float_eq(compare_lines(&ratio_line_1, &ratio_line_2), 0.9);
    }

    #[test]
    fn ratio_line_similarity_matches_cpp_eighty_percent_case() {
        let ratio_line_1 = ratio_line(&[(0.0, 0.4), (0.6, 1.0)]);
        let ratio_line_2 = ratio_line(&[(0.05, 0.45), (0.55, 0.95)]);

        assert_float_eq(compare_lines(&ratio_line_1, &ratio_line_2), 0.8);
    }

    #[test]
    fn compare_with_function_averages_line_similarities() {
        let first = vec![ratio_line(&[(0.2, 0.4)]), ratio_line(&[(0.1, 0.3)])];
        let second = vec![ratio_line(&[(0.2, 0.4)]), ratio_line(&[(0.2, 0.4)])];

        assert_float_eq(compare_with(&first, &second), 0.9);
    }

    #[test]
    fn trace_compare_with_rotates_ratio_lines_to_find_best_alignment() {
        let trace1 = Trace {
            ratio_lines: vec![
                ratio_line(&[(0.1, 0.2)]),
                ratio_line(&[(0.3, 0.4)]),
                ratio_line(&[(0.5, 0.6)]),
            ],
        };
        let trace2 = Trace {
            ratio_lines: vec![
                ratio_line(&[(0.3, 0.4)]),
                ratio_line(&[(0.5, 0.6)]),
                ratio_line(&[(0.1, 0.2)]),
            ],
        };

        assert_float_eq(trace1.compare_with(0.99, &trace2), 1.0);
    }

    #[test]
    fn combine_close_slices_merges_only_nearby_slices() {
        let slices = vec![
            polar_slice(0.1, 0.2),
            polar_slice(0.24, 0.3),
            polar_slice(0.6, 0.7),
        ];

        let merged = combine_close_slices(slices.clone(), 0.05);
        let separate = combine_close_slices(slices, 0.01);

        assert_eq!(merged.len(), 2);
        assert_float_eq(merged[0].get_start().get_radius(), 0.1);
        assert_float_eq(merged[0].get_end().get_radius(), 0.3);
        assert_eq!(separate.len(), 3);
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

        let radius = deduce_longest_radius(&mosaics, center);

        assert_float_eq(radius, 4.038873605350878);
    }

    #[test]
    fn deduce_slices_from_mosaic_normalizes_any_slices_it_produces() {
        let mosaic = square_mosaic();
        let params = TraceParams::new(12, 0.2);
        let coordinate_system = WrappedCoordinateSystem::new(
            mosaic.get_center_of_mass().get_local_point(),
            Vec3d::new(1.0, 0.0, 0.0),
            Vec3d::new(0.0, 1.0, 0.0),
        );

        let slices = deduce_slices_from_mosaic(
            vec![mosaic.clone()],
            CoordinatedRegionedAngle::new(coordinate_system, RegionedAngle::new(45.0, 0.0, 360.0)),
            mosaic.get_bounding_circle().get_radius(),
            &params,
        );

        for slice in slices {
            assert!(slice.get_start().get_radius() >= 0.0);
            assert!(slice.get_end().get_radius() >= slice.get_start().get_radius());
            assert!(slice.get_end().get_radius() <= 1.0);
        }
    }

    #[test]
    fn trace_new_from_mosaic_builds_requested_number_of_ratio_lines() {
        let trace = Trace::new_from_mosaic(square_mosaic(), TraceParams::new(18, 0.2));

        assert_eq!(trace.ratio_lines.len(), 18);
        assert!(trace.ratio_lines.iter().any(|line| !line.slices.is_empty()));
        assert_float_eq(trace.compare_with(0.99, &trace.clone()), 1.0);
    }

    #[test]
    fn trace_compare_with_returns_zero_when_target_similarity_is_unreachable() {
        let trace = Trace::new_from_mosaic(square_mosaic(), TraceParams::new(18, 0.2));

        assert_float_eq(trace.compare_with(1.01, &trace.clone()), 0.0);
    }

    #[test]
    fn trace_new_from_mosaics_combines_multiple_mosaics_and_self_matches() {
        let combined = Trace::new_from_mosaics(
            vec![square_mosaic(), translated_square_mosaic()],
            TraceParams::new(18, 0.2),
        );
        let same_family = Trace::new_from_mosaics(
            vec![translated_square_mosaic(), square_mosaic()],
            TraceParams::new(18, 0.2),
        );

        assert_eq!(combined.ratio_lines.len(), 18);
        assert_float_eq(combined.compare_with(0.99, &same_family), 1.0);
        assert_float_eq(combined.compare_with(1.01, &same_family), 0.0);
    }
}

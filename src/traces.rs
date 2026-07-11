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
        PolarSlice { start, end }
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
    if line1.slices.is_empty() || line2.slices.is_empty() {
        panic!("Both lines must have at least one slice to compare.");
    }
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
                global_point.get_x().floor(),
                global_point.get_y().ceil(),
                0.0,
            );
            let br = Vec3d::new(
                global_point.get_x().ceil(),
                global_point.get_y().floor(),
                0.0,
            );
            let rectangle = Rectangle::new(tl, br);
            let coordinated_rectangle =
                CoordinatedRectangle::new_from_rectangle(rectangle, global_coordinate_system)
                    .convert_to(coordinated_regioned_angle.get_coordinate_system());
            let line_coordinate_system = coordinated_regioned_angle.get_coordinate_system();
            line_coordinate_system.rotate(coordinated_regioned_angle.get_regioned_angle());
            let x_line_start = CoordinatedPoint::new(
                line_coordinate_system.clone(),
                Vec3d::new(-radius, 0.0, 0.0),
            );
            let x_line_end =
                CoordinatedPoint::new(line_coordinate_system.clone(), Vec3d::new(radius, 0.0, 0.0));
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

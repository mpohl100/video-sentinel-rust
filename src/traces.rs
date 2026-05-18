use crate::math::CoordinatedLine;
use crate::math::CoordinatedPoint;
use crate::math::CoordinatedRectangle;
use crate::math::Rectangle;
use crate::math::RegionedAngle;
use crate::math::WrappedCoordinateSystem;
use crate::mosaics::WrappedMosaic;
use crate::slices::Slice;

use rs_math3d::Vec3d;

#[derive(Clone)]
struct RatioLine {
    coordinate_system: WrappedCoordinateSystem,
    slices: Vec<Slice>,
}

impl RatioLine {
    fn duplicate(&self) -> Self {
        RatioLine {
            coordinate_system: self.coordinate_system.duplicate(),
            slices: self.slices.clone(),
        }
    }
}

#[derive(Clone, PartialEq, PartialOrd)]
pub struct TraceParams {
    num_skeleton: usize,
    close_slice_threshold: f64,
}

#[derive(Clone)]
pub struct Trace {
    ratio_lines: Vec<RatioLine>,
}

impl Trace {
    pub fn new_from_mosaic(mosaic: WrappedMosaic, params: TraceParams) -> Self {
        let ratio_lines = (0..params.num_skeleton)
            .map(|i| {
                let coordinate_system = WrappedCoordinateSystem::new(
                    mosaic.get_center_of_mass(),
                    Vec3d::new(1.0, 0.0, 0.0),
                    Vec3d::new(0.0, 1.0, 0.0),
                );
                coordinate_system.rotate(RegionedAngle::new(
                    (i as f64) * (360.0 / params.num_skeleton as f64),
                    -180.0,
                    180.0,
                ));
                RatioLine {
                    coordinate_system: coordinate_system.clone(),
                    slices: deduce_slices_from_mosaic(
                        vec![mosaic.clone()],
                        coordinate_system.clone(),
                        mosaic.get_bounding_circle().get_radius(),
                        &params,
                    ),
                }
            })
            .collect();
        Trace { ratio_lines }
    }

    pub fn compare_with(&self, target_similarity: f64, other: &Trace) -> f64 {
        for i in 0..self.ratio_lines.len() {
            let mut duplicated_ratio_lines = self
                .ratio_lines
                .iter()
                .map(|line| line.duplicate())
                .collect::<Vec<_>>();
            for line in duplicated_ratio_lines.iter_mut() {
                line.coordinate_system.rotate(RegionedAngle::new(
                    (i as f64) * (360.0 / self.ratio_lines.len() as f64),
                    -180.0,
                    180.0,
                ));
            }
            let similarity = compare_with(&mut duplicated_ratio_lines, &other.ratio_lines);
            if similarity >= target_similarity {
                return similarity;
            }
        }
        0.0
    }
}

fn compare_with(first_ratio_lines: &mut [RatioLine], second_ratio_lines: &[RatioLine]) -> f64 {
    let mut total_similarity = 0.0;
    for (line1, line2) in first_ratio_lines.iter_mut().zip(second_ratio_lines.iter()) {
        line1
            .coordinate_system
            .align_x_axis_with(&line2.coordinate_system);
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
    filtered_overlaps.sort_by(|lhs, rhs| {
        rhs.slice
            .get_end()
            .get_x()
            .partial_cmp(&lhs.slice.get_end().get_x())
            .unwrap()
    });
    let left_quantile_index = 2 * line1.slices.len() + 1;
    let right_quantile_index = 2 * line2.slices.len() + 1;
    let quantile_index = std::cmp::max(left_quantile_index, right_quantile_index) + 1;
    let n = std::cmp::min(filtered_overlaps.len(), quantile_index);
    let mut similarity = 0.0;
    for item in filtered_overlaps.iter().take(n) {
        similarity += item.slice.get_end().get_x() - item.slice.get_start().get_x();
    }
    similarity
}

#[derive(Clone)]
struct TaggedRatio {
    slice: Slice,
    left_tag: usize,
    right_tag: usize,
}

fn get_overlaps(line1: &RatioLine, line2: &RatioLine) -> Vec<TaggedRatio> {
    let coordinate_system = line1.coordinate_system.clone();
    let converted_slices_1: Vec<Slice> = line1
        .slices
        .iter()
        .map(|slice| slice.convert_to(coordinate_system.clone()))
        .collect();
    // convert the following code to rust
    let mut overlaps: Vec<TaggedRatio> = Vec::new();
    let mut interesting_points: Vec<f64> = Vec::new();
    interesting_points.push(0.0);
    interesting_points.push(1.0);
    for ratio in &converted_slices_1 {
        interesting_points.push(ratio.get_start().get_x());
        interesting_points.push(ratio.get_end().get_x());
    }
    for ratio in &line2.slices {
        interesting_points.push(ratio.get_start().get_x());
        interesting_points.push(ratio.get_end().get_x());
    }
    interesting_points.sort_by(|a, b| a.partial_cmp(b).unwrap());
    for i in 0..interesting_points.len() - 1 {
        let from = interesting_points[i];
        let to = interesting_points[i + 1];
        if from == to {
            continue; // skip zero-length intervals
        }
        let current_midpoint = (from + to) / 2.0;
        let pred = |ratio: &Slice| {
            ratio.get_start().get_x() <= current_midpoint
                && ratio.get_end().get_x() >= current_midpoint
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
            slice: Slice::new(
                CoordinatedPoint::new(coordinate_system.clone(), Vec3d::new(from, 0.0, 0.0)),
                CoordinatedPoint::new(coordinate_system.clone(), Vec3d::new(to, 0.0, 0.0)),
            ),
            left_tag,
            right_tag,
        });
    }
    overlaps
}

fn deduce_slices_from_mosaic(
    mosaics: Vec<WrappedMosaic>,
    coordinate_system: WrappedCoordinateSystem,
    radius: f64,
    params: &TraceParams,
) -> Vec<Slice> {
    let mut slices = Vec::new();
    // for every x in the range of -radius to radius with a step of 0.5, find the intersections with the mosaic and create slices
    let step = 0.5;
    let mut x = -radius;
    while x <= radius {
        let global_coordinate_system = WrappedCoordinateSystem::new(
            Vec3d::new(0.0, 0.0, 0.0),
            Vec3d::new(1.0, 0.0, 0.0),
            Vec3d::new(0.0, 1.0, 0.0),
        );
        let point = CoordinatedPoint::new(coordinate_system.clone(), Vec3d::new(x, 0.0, 0.0));
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
                    .convert_to(coordinate_system.clone());
            let x_line_start =
                CoordinatedPoint::new(coordinate_system.clone(), Vec3d::new(-radius, 0.0, 0.0));
            let x_line_end =
                CoordinatedPoint::new(coordinate_system.clone(), Vec3d::new(radius, 0.0, 0.0));
            let x_axis_line = CoordinatedLine::new(x_line_start, x_line_end);
            let clipped_line = coordinated_rectangle.get_intersection_line(x_axis_line);
            if let Some(clipped_line) = clipped_line {
                let slice = Slice::new(clipped_line.get_start(), clipped_line.get_end());
                slices.push(slice);
            }
        }
        x += step;
    }
    combine_close_slices(slices, params.close_slice_threshold)
}

fn combine_close_slices(slices: Vec<Slice>, threshold: f64) -> Vec<Slice> {
    if slices.is_empty() {
        return slices;
    }
    let mut combined_slices = Vec::new();
    let mut current_slice = slices[0].clone();
    for slice in slices.iter().skip(1) {
        if slice.get_start().get_x() - current_slice.get_end().get_x() <= threshold {
            current_slice = Slice::new(current_slice.get_start(), slice.get_end());
        } else {
            combined_slices.push(current_slice);
            current_slice = slice.clone();
        }
    }
    combined_slices.push(current_slice);
    combined_slices
}

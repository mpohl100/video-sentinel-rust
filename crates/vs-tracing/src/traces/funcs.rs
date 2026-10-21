#![warn(clippy::pedantic)]
#![allow(
    clippy::missing_errors_doc,
    clippy::missing_panics_doc,
    clippy::too_many_lines,
    clippy::unused_async_trait_impl
)]

use crate::math::AnonymizedCoordinateSystem;
use crate::math::CoordinateSystem;
use crate::math::CoordinatedLine;
use crate::math::CoordinatedPoint;
use crate::math::CoordinatedRegionedAngle;
use crate::math::PolarCoordinates;
use crate::math::Rectangle;
use crate::mosaics::WrappedMosaic;
use num_traits::ToPrimitive;
use rs_math3d::Vec3d;
use smallvec::SmallVec;
use std::sync::atomic::{AtomicBool, Ordering};

use super::{PolarSlice, Ratio, RatioLine, Tag, TaggedRatio};

pub(super) static TRACE_DEBUG: AtomicBool = AtomicBool::new(false);

pub(super) fn f64_from_usize(value: usize) -> f64 {
    value
        .to_f64()
        .expect("usize value should convert to f64 for trace calculations")
}

#[cfg(test)]
pub(super) fn usize_from_f64(value: f64) -> usize {
    value
        .to_usize()
        .expect("trace coordinate should fit within usize")
}

pub fn set_trace_debug(enabled: bool) {
    TRACE_DEBUG.store(enabled, Ordering::Relaxed);
}

pub(super) fn trace_debug_enabled() -> bool {
    TRACE_DEBUG.load(Ordering::Relaxed)
}

pub(super) fn calculate_total_mass(mosaics: &[WrappedMosaic]) -> f64 {
    mosaics
        .iter()
        .map(crate::mosaics::WrappedMosaic::get_area)
        .sum()
}

pub(super) fn calculate_total_surrounding_circle_area(mosaics: &[WrappedMosaic]) -> f64 {
    if mosaics.is_empty() {
        return 0.0;
    }

    let center_of_mass = calculate_center_of_mass(mosaics);
    let radius = deduce_longest_radius(mosaics, &center_of_mass);

    std::f64::consts::PI * radius * radius
}

pub(super) fn compare_with_rotation(
    first_ratio_lines: &[RatioLine],
    second_ratio_lines: &[RatioLine],
    rotation: usize,
) -> f64 {
    assert_eq!(
        first_ratio_lines.len(),
        second_ratio_lines.len(),
        "ratio line counts must match"
    );

    if first_ratio_lines.is_empty() {
        return 0.0;
    }

    let line_count = first_ratio_lines.len();
    let rotation = rotation % line_count;
    let mut total_similarity = 0.0;

    for (line_index, line1) in first_ratio_lines.iter().enumerate() {
        let rotated_index = (line_index + line_count - rotation) % line_count;
        let similarity = compare_lines(line1, &second_ratio_lines[rotated_index]);
        if trace_debug_enabled() {
            println!(
                "trace.compare_with line_index={line_index} rotated_index={rotated_index} line_similarity={similarity:.8}",
            );
        }
        total_similarity += similarity;
    }

    let similarity = total_similarity / f64_from_usize(line_count);
    if trace_debug_enabled() {
        println!("trace.compare_with average_similarity={similarity:.8} line_count={line_count}");
    }
    similarity
}

pub(super) fn compare_lines(line1: &RatioLine, line2: &RatioLine) -> f64 {
    if line1.slices().is_empty() && line2.slices().is_empty() {
        if trace_debug_enabled() {
            println!("trace.compare_lines both_empty similarity=1.00000000");
        }
        return 1.0;
    }
    if line1.slices().is_empty() || line2.slices().is_empty() {
        if trace_debug_enabled() {
            println!(
                "trace.compare_lines one_empty left_slice_count={} right_slice_count={} similarity=0.00000000",
                line1.slices().len(),
                line2.slices().len(),
            );
        }
        return 0.0;
    }

    let overlaps = get_overlaps(line1, line2);
    if trace_debug_enabled() {
        println!(
            "trace.compare_lines overlap_count={} left_slice_count={} right_slice_count={}",
            overlaps.len(),
            line1.slices().len(),
            line2.slices().len(),
        );
        for (overlap_index, overlap) in overlaps.iter().enumerate() {
            println!(
                "trace.compare_lines overlap[{}] from={:.8} to={:.8} left_tag={:?} right_tag={:?}",
                overlap_index,
                overlap.ratio().from(),
                overlap.ratio().to(),
                overlap.left_tag(),
                overlap.right_tag(),
            );
        }
    }
    let similar_overlap: f64 = overlaps
        .iter()
        .filter(|tr| tr.left_tag() == Tag::Filled && tr.right_tag() == Tag::Filled)
        .map(|tagged_ratio| tagged_ratio.ratio().to() - tagged_ratio.ratio().from())
        .sum();
    let different_overlap: f64 = overlaps
        .iter()
        .filter(|tr| tr.left_tag() != tr.right_tag())
        .map(|tagged_ratio| tagged_ratio.ratio().to() - tagged_ratio.ratio().from())
        .sum();
    if similar_overlap.abs() < 1e-6 {
        if trace_debug_enabled() {
            println!(
                "trace.compare_lines similar_overlap={similar_overlap:.8} different_overlap={different_overlap:.8} similarity=0.00000000",
            );
        }
        return 0.0;
    }
    let similarity = (similar_overlap - different_overlap) / similar_overlap;
    if trace_debug_enabled() {
        println!(
            "trace.compare_lines similar_overlap={similar_overlap:.8} different_overlap={different_overlap:.8} similarity={similarity:.8}",
        );
    }
    similarity
}

pub(super) fn get_overlaps(line1: &RatioLine, line2: &RatioLine) -> SmallVec<[TaggedRatio; 16]> {
    // convert the following code to rust
    let mut overlaps: SmallVec<[TaggedRatio; 16]> = SmallVec::with_capacity(16);
    let mut interesting_points: SmallVec<[f64; 16]> = SmallVec::with_capacity(16);
    interesting_points.push(0.0);
    interesting_points.push(1.0);
    for polar_slice in line1.slices() {
        interesting_points.push(polar_slice.get_start().get_radius());
        interesting_points.push(polar_slice.get_end().get_radius());
    }
    for polar_slice in line2.slices() {
        interesting_points.push(polar_slice.get_start().get_radius());
        interesting_points.push(polar_slice.get_end().get_radius());
    }
    interesting_points.sort_by(|a, b| a.partial_cmp(b).unwrap());
    for i in 0..interesting_points.len() - 1 {
        let from = interesting_points[i];
        let to = interesting_points[i + 1];
        if (from - to).abs() < f64::EPSILON {
            continue; // skip zero-length intervals
        }
        let current_midpoint = f64::midpoint(from, to);
        let pred = |polar_ratio: &PolarSlice| {
            polar_ratio.get_start().get_radius() <= current_midpoint
                && polar_ratio.get_end().get_radius() >= current_midpoint
        };
        let lit = line1.slices().iter().find(|&ratio| pred(ratio));
        let rit = line2.slices().iter().find(|&ratio| pred(ratio));
        let mut left_tag = Tag::Empty;
        let mut right_tag = Tag::Empty;
        if lit.is_some() {
            left_tag = Tag::Filled;
        }
        if rit.is_some() {
            right_tag = Tag::Filled;
        }
        overlaps.push(TaggedRatio::new(Ratio::new(from, to), left_tag, right_tag));
    }
    overlaps
}

pub(super) fn deduce_slices_from_mosaic(
    mosaics: &[WrappedMosaic],
    coordinated_regioned_angle: &CoordinatedRegionedAngle,
    radius: f64,
) -> Vec<PolarSlice> {
    let mut slices = Vec::with_capacity(20);
    let step = 0.5;
    let mut x = -0.1 * radius;
    while x <= 1.15 * radius {
        let global_coordinate_system = AnonymizedCoordinateSystem::Direct(CoordinateSystem::new(
            Vec3d::new(0.0, 0.0, 0.0),
            Vec3d::new(1.0, 0.0, 0.0),
            Vec3d::new(0.0, 1.0, 0.0),
        ));
        let current_polar_coordinates =
            PolarCoordinates::new(x, coordinated_regioned_angle.clone());
        let point = current_polar_coordinates.to_cartesian();
        let contains_point = mosaics.iter().any(|mosaic| mosaic.contains_point(&point));
        if contains_point {
            let global_point = point.convert_to(global_coordinate_system.clone());
            let tl = Vec3d::new(
                (global_point.get_x()).floor(),
                (global_point.get_y()).floor(),
                0.0,
            );
            let br = Vec3d::new(
                (global_point.get_x() + 1.0).floor(),
                (global_point.get_y() + 1.0).floor(),
                0.0,
            );
            let rectangle = Rectangle::new(tl, br);
            let mut line_coordinate_system = coordinated_regioned_angle
                .get_coordinate_system()
                .duplicate();
            line_coordinate_system
                .rotate(coordinated_regioned_angle.get_regioned_angle().inverted());
            let x_line_start =
                CoordinatedPoint::new(line_coordinate_system.clone(), Vec3d::new(0.0, 0.0, 0.0));
            let x_line_end = CoordinatedPoint::new(
                line_coordinate_system.clone(),
                Vec3d::new(1.1 * radius, 0.0, 0.0),
            );
            let x_axis_line = CoordinatedLine::new(x_line_start, x_line_end);
            let x_axis_global_line = x_axis_line.to_global_line();
            let clipped_line = rectangle.get_intersection_line(x_axis_global_line);
            if let Some(clipped_line) = clipped_line {
                let global_clipped_line = CoordinatedLine::new(
                    CoordinatedPoint::new(
                        global_coordinate_system.clone(),
                        clipped_line.get_start().point,
                    ),
                    CoordinatedPoint::new(
                        global_coordinate_system.clone(),
                        clipped_line.get_end().point,
                    ),
                );
                let converted_clipped_line =
                    global_clipped_line.convert_to(line_coordinate_system.clone());

                let polar_start = PolarCoordinates::new(
                    converted_clipped_line.get_start().get_x() / radius,
                    coordinated_regioned_angle.clone(),
                );
                let polar_end = PolarCoordinates::new(
                    converted_clipped_line.get_end().get_x() / radius,
                    coordinated_regioned_angle.clone(),
                );
                assert!(converted_clipped_line.get_start().get_y().abs() < 1e-4);
                assert!(converted_clipped_line.get_end().get_y().abs() < 1e-4);

                let slice = PolarSlice::new(polar_start, polar_end);
                slices.push(Some(slice));
            } else {
                slices.push(None);
            }
        }
        x += step;
    }
    combine_close_slices(&slices)
}

pub(super) fn combine_close_slices(slices: &[Option<PolarSlice>]) -> Vec<PolarSlice> {
    if slices.is_empty() {
        return Vec::new();
    }
    let mut combined_slices = Vec::new();
    let mut current_slice = slices[0].clone();
    for slice in slices.iter().skip(1) {
        match (current_slice.clone(), slice.clone()) {
            (Some(mut current), Some(next)) => {
                current = PolarSlice::new(current.get_start().clone(), next.get_end().clone());
                current_slice = Some(current);
            }
            (Some(_), None) => {
                combined_slices.push(current_slice.unwrap());
                current_slice = None;
            }
            (None, Some(_)) => {
                current_slice.clone_from(slice);
            }
            (None, None) => {}
        }
    }
    if let Some(item) = current_slice {
        combined_slices.push(item);
    }

    combined_slices
}

pub(super) fn calculate_center_of_mass(mosaics: &[WrappedMosaic]) -> CoordinatedPoint {
    let mut total_mass = 0.0;
    let mut center_of_mass = Vec3d::new(0.0, 0.0, 0.0);
    let global_coordinate_system = AnonymizedCoordinateSystem::Direct(CoordinateSystem::new(
        Vec3d::new(0.0, 0.0, 0.0),
        Vec3d::new(1.0, 0.0, 0.0),
        Vec3d::new(0.0, 1.0, 0.0),
    ));
    for mosaic in mosaics {
        let mass = mosaic.get_area();
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
    CoordinatedPoint::new(global_coordinate_system, center_of_mass)
}

pub(super) fn deduce_longest_radius(
    mosaics: &[WrappedMosaic],
    center_of_mass: &CoordinatedPoint,
) -> f64 {
    let mut longest_radius = 0.0;
    for mosaic in mosaics {
        let mosaic_longest_distance = mosaic.deduce_longest_distance_point(center_of_mass);
        if let Some(mosaic_longest_distance) = mosaic_longest_distance {
            let distance = mosaic_longest_distance.distance_to(center_of_mass);
            if distance > longest_radius {
                longest_radius = distance;
            }
        }
    }
    longest_radius
}

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
use std::sync::atomic::{AtomicBool, Ordering};

static TRACE_DEBUG: AtomicBool = AtomicBool::new(false);

pub fn set_trace_debug(enabled: bool) {
    TRACE_DEBUG.store(enabled, Ordering::Relaxed);
}

fn trace_debug_enabled() -> bool {
    TRACE_DEBUG.load(Ordering::Relaxed)
}

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
            PolarSlice {
                start: end,
                end: start,
            }
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
                    ),
                }
            })
            .collect();
        Trace { ratio_lines }
    }

    pub fn compare_with(&self, target_similarity: f64, other: &Trace) -> f64 {
        let mut highest_similarity = 0.0;
        for i in 0..self.ratio_lines.len() {
            let mut second_ratio_lines = other.ratio_lines.clone();
            second_ratio_lines.rotate_right(i);
            let similarity = compare_with(&self.ratio_lines, &second_ratio_lines);
            if trace_debug_enabled() {
                println!(
                    "trace.compare_with rotation={} similarity={:.8} highest_before={:.8} target={:.8}",
                    i, similarity, highest_similarity, target_similarity,
                );
            }
            if similarity > highest_similarity {
                highest_similarity = similarity;
            }
            if highest_similarity >= target_similarity {
                if trace_debug_enabled() {
                    println!(
                        "trace.compare_with early_exit rotation={} highest_similarity={:.8}",
                        i, highest_similarity,
                    );
                }
                break;
            }
        }
        if trace_debug_enabled() {
            println!(
                "trace.compare_with final highest_similarity={:.8}",
                highest_similarity,
            );
        }
        highest_similarity
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
    let mut similarities = Vec::new();
    for (line_index, (line1, line2)) in first_ratio_lines
        .iter()
        .zip(second_ratio_lines.iter())
        .enumerate()
    {
        let similarity = compare_lines(line1, line2);
        if trace_debug_enabled() {
            println!(
                "trace.compare_with line_index={} line_similarity={:.8}",
                line_index, similarity,
            );
        }
        similarities.push(similarity);
    }
    // calculate the average similarity
    let similarity = similarities.iter().sum::<f64>() / similarities.len() as f64;
    if trace_debug_enabled() {
        println!(
            "trace.compare_with average_similarity={:.8} line_count={}",
            similarity,
            first_ratio_lines.len(),
        );
    }
    similarity
}

fn compare_lines(line1: &RatioLine, line2: &RatioLine) -> f64 {
    if line1.slices.is_empty() && line2.slices.is_empty() {
        if trace_debug_enabled() {
            println!("trace.compare_lines both_empty similarity=1.00000000");
        }
        return 1.0;
    }
    if line1.slices.is_empty() || line2.slices.is_empty() {
        if trace_debug_enabled() {
            println!(
                "trace.compare_lines one_empty left_slice_count={} right_slice_count={} similarity=0.00000000",
                line1.slices.len(),
                line2.slices.len(),
            );
        }
        return 0.0;
    }

    let overlaps = get_overlaps(line1, line2);
    if trace_debug_enabled() {
        println!(
            "trace.compare_lines overlap_count={} left_slice_count={} right_slice_count={}",
            overlaps.len(),
            line1.slices.len(),
            line2.slices.len(),
        );
        for (overlap_index, overlap) in overlaps.iter().enumerate() {
            println!(
                "trace.compare_lines overlap[{}] from={:.8} to={:.8} left_tag={:?} right_tag={:?}",
                overlap_index,
                overlap.ratio.from,
                overlap.ratio.to,
                overlap.left_tag,
                overlap.right_tag,
            );
        }
    }
    let similar_overlaps: Vec<TaggedRatio> = overlaps.clone()
        .into_iter()
        .filter(|tr| tr.left_tag == Tag::Filled && tr.right_tag == Tag::Filled)
        .collect();
    let mut similar_overlap = 0.0;
    for item in similar_overlaps.iter() {
        similar_overlap += item.ratio.to - item.ratio.from;
    }
    let different_overlaps: Vec<TaggedRatio> = overlaps.into_iter().filter(|tr| tr.left_tag != tr.right_tag).collect();
    let mut different_overlap = 0.0;
    for item in different_overlaps {
        different_overlap += item.ratio.to - item.ratio.from;
    }
    if similar_overlap.abs() < 1e-6 {
        if trace_debug_enabled() {
            println!(
                "trace.compare_lines similar_overlap={:.8} different_overlap={:.8} similarity=0.00000000",
                similar_overlap,
                different_overlap,
            );
        }
        return 0.0;
    }
    let similarity = (similar_overlap - different_overlap) / similar_overlap;
    if trace_debug_enabled() {
        println!(
            "trace.compare_lines similar_overlap={:.8} different_overlap={:.8} similarity={:.8}",
            similar_overlap,
            different_overlap,
            similarity,
        );
    }
    similarity
}

#[derive(Clone)]
struct Ratio {
    from: f64,
    to: f64,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Tag {
    Empty = 0,
    Filled = 1,
}

#[derive(Clone)]
struct TaggedRatio {
    ratio: Ratio,
    left_tag: Tag,
    right_tag: Tag,
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
        let mut left_tag = Tag::Empty;
        let mut right_tag = Tag::Empty;
        if lit.is_some() {
            left_tag = Tag::Filled;
        }
        if rit.is_some() {
            right_tag = Tag::Filled;
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
) -> Vec<PolarSlice> {
    // let input_coordinate_system = coordinated_regioned_angle.get_coordinate_system();
    // let input_origin = input_coordinate_system.to_global(CoordinatedPoint::new(
    //     input_coordinate_system.clone(),
    //     Vec3d::new(0.0, 0.0, 0.0),
    // ));
    // let input_x_axis_point = input_coordinate_system.to_global(CoordinatedPoint::new(
    //     input_coordinate_system.clone(),
    //     Vec3d::new(1.0, 0.0, 0.0),
    // ));
    // let input_y_axis_point = input_coordinate_system.to_global(CoordinatedPoint::new(
    //     input_coordinate_system.clone(),
    //     Vec3d::new(0.0, 1.0, 0.0),
    // ));
    // let input_x_axis = input_x_axis_point - input_origin;
    // let input_y_axis = input_y_axis_point - input_origin;
    // println!("deduce_slices_from_mosaic: begin");
    // println!("  input mosaics.len = {}", mosaics.len());
    // for (mosaic_index, mosaic) in mosaics.iter().enumerate() {
    //     let bounding_box = mosaic.get_bounding_box().to_global_rectangle();
    //     let center = mosaic
    //         .get_center_of_mass()
    //         .convert_to(input_coordinate_system.clone())
    //         .convert_to(WrappedCoordinateSystem::new(
    //             Vec3d::new(0.0, 0.0, 0.0),
    //             Vec3d::new(1.0, 0.0, 0.0),
    //             Vec3d::new(0.0, 1.0, 0.0),
    //         ));
    //     println!(
    //         "  input mosaic[{mosaic_index}] area={:.8} bbox=(({:.8}, {:.8}), ({:.8}, {:.8})) center=({:.8}, {:.8}, {:.8})",
    //         mosaic.get_area(),
    //         bounding_box.get_top_left().x,
    //         bounding_box.get_top_left().y,
    //         bounding_box.get_bottom_right().x,
    //         bounding_box.get_bottom_right().y,
    //         center.get_x(),
    //         center.get_y(),
    //         center.get_z(),
    //     );
    // }
    // println!(
    //     "  input coordinated_regioned_angle.degrees = {:.8}",
    //     coordinated_regioned_angle.get_angle_degrees()
    // );
    // println!("  input coordinated_regioned_angle.coordinate_system:");
    // println!(
    //     "    origin=({:.8}, {:.8}, {:.8})",
    //     input_origin.x, input_origin.y, input_origin.z,
    // );
    // println!(
    //     "    x_axis=({:.8}, {:.8}, {:.8})",
    //     input_x_axis.x, input_x_axis.y, input_x_axis.z,
    // );
    // println!(
    //     "    y_axis=({:.8}, {:.8}, {:.8})",
    //     input_y_axis.x, input_y_axis.y, input_y_axis.z,
    // );
    // println!("  input radius = {:.8}", radius);
    // println!("  input params.num_skeleton = {}", params.num_skeleton());
    // println!(
    //     "  input params.close_slice_threshold = {:.8}",
    //     params.close_slice_threshold()
    // );

    let mut slices = Vec::new();
    // for every x in the range of -radius to radius with a step of 0.5, find the intersections with the mosaic and create slices
    let step = 0.5;
    // println!("  local step = {:.8}", step);
    let mut x = -0.1 * radius;
    // let mut iteration = 0usize;
    // println!("  local initial x = {:.8}", x);
    while x <= 1.15 * radius {
        // println!("loop iteration {iteration}: begin");
        // println!("  local x = {:.8}", x);
        let global_coordinate_system = WrappedCoordinateSystem::new(
            Vec3d::new(0.0, 0.0, 0.0),
            Vec3d::new(1.0, 0.0, 0.0),
            Vec3d::new(0.0, 1.0, 0.0),
        );
        // let global_origin = global_coordinate_system.to_global(CoordinatedPoint::new(
        //     global_coordinate_system.clone(),
        //     Vec3d::new(0.0, 0.0, 0.0),
        // ));
        // let global_x_axis_point = global_coordinate_system.to_global(CoordinatedPoint::new(
        //     global_coordinate_system.clone(),
        //     Vec3d::new(1.0, 0.0, 0.0),
        // ));
        // let global_y_axis_point = global_coordinate_system.to_global(CoordinatedPoint::new(
        //     global_coordinate_system.clone(),
        //     Vec3d::new(0.0, 1.0, 0.0),
        // ));
        // let global_x_axis = global_x_axis_point - global_origin;
        // let global_y_axis = global_y_axis_point - global_origin;
        // println!("  local global_coordinate_system:");
        // println!(
        //     "    origin=({:.8}, {:.8}, {:.8})",
        //     global_origin.x, global_origin.y, global_origin.z,
        // );
        // println!(
        //     "    x_axis=({:.8}, {:.8}, {:.8})",
        //     global_x_axis.x, global_x_axis.y, global_x_axis.z,
        // );
        // println!(
        //     "    y_axis=({:.8}, {:.8}, {:.8})",
        //     global_y_axis.x, global_y_axis.y, global_y_axis.z,
        // );
        let current_polar_coordinates =
            PolarCoordinates::new(x, coordinated_regioned_angle.clone());
        // println!(
        //     "  local current_polar_coordinates radius={:.8} angle_degrees={:.8}",
        //     current_polar_coordinates.get_radius(),
        //     current_polar_coordinates.get_angle().get_angle_degrees(),
        // );
        let point = current_polar_coordinates.to_cartesian();
        // let point_global = point.convert_to(global_coordinate_system.clone());
        // println!(
        //     "  local point=({:.8}, {:.8}, {:.8})",
        //     point_global.get_x(),
        //     point_global.get_y(),
        //     point_global.get_z(),
        // );
        let contains_point = mosaics
            .iter()
            .any(|mosaic| mosaic.contains_point(point.clone()));
        // println!("  local contains_point = {}", contains_point);
        if contains_point {
            let global_point = point.convert_to(global_coordinate_system.clone());
            // println!(
            //     "  local global_point=({:.8}, {:.8}, {:.8})",
            //     global_point.get_x(),
            //     global_point.get_y(),
            //     global_point.get_z(),
            // );
            let tl = Vec3d::new(
                (global_point.get_x()).floor(),
                (global_point.get_y()).floor(),
                0.0,
            );
            // println!("  local tl=({:.8}, {:.8}, {:.8})", tl.x, tl.y, tl.z);
            let br = Vec3d::new(
                (global_point.get_x() + 1.0).floor(),
                (global_point.get_y() + 1.0).floor(),
                0.0,
            );
            // println!("  local br=({:.8}, {:.8}, {:.8})", br.x, br.y, br.z);
            let rectangle = Rectangle::new(tl, br);
            // println!(
            //     "  local rectangle top_left=({:.8}, {:.8}, {:.8}) bottom_right=({:.8}, {:.8}, {:.8})",
            //     tl.x,
            //     tl.y,
            //     tl.z,
            //     br.x,
            //     br.y,
            //     br.z,
            // );
            let coordinated_rectangle = CoordinatedRectangle::new_from_rectangle(
                rectangle,
                global_coordinate_system.clone(),
            );
            // let coordinated_rectangle_global = coordinated_rectangle.to_global_rectangle();
            // println!(
            //     "  local coordinated_rectangle global_top_left=({:.8}, {:.8}, {:.8}) global_bottom_right=({:.8}, {:.8}, {:.8})",
            //     coordinated_rectangle_global.get_top_left().x,
            //     coordinated_rectangle_global.get_top_left().y,
            //     coordinated_rectangle_global.get_top_left().z,
            //     coordinated_rectangle_global.get_bottom_right().x,
            //     coordinated_rectangle_global.get_bottom_right().y,
            //     coordinated_rectangle_global.get_bottom_right().z,
            // );
            let line_coordinate_system = coordinated_regioned_angle
                .get_coordinate_system()
                .duplicate();
            // let line_coordinate_system_origin = line_coordinate_system.to_global(CoordinatedPoint::new(
            //     line_coordinate_system.clone(),
            //     Vec3d::new(0.0, 0.0, 0.0),
            // ));
            // let line_coordinate_system_x_axis_point = line_coordinate_system.to_global(CoordinatedPoint::new(
            //     line_coordinate_system.clone(),
            //     Vec3d::new(1.0, 0.0, 0.0),
            // ));
            // let line_coordinate_system_y_axis_point = line_coordinate_system.to_global(CoordinatedPoint::new(
            //     line_coordinate_system.clone(),
            //     Vec3d::new(0.0, 1.0, 0.0),
            // ));
            // let line_coordinate_system_x_axis =
            //     line_coordinate_system_x_axis_point - line_coordinate_system_origin;
            // let line_coordinate_system_y_axis =
            //     line_coordinate_system_y_axis_point - line_coordinate_system_origin;
            // println!("  local line_coordinate_system before_rotate:");
            // println!(
            //     "    origin=({:.8}, {:.8}, {:.8})",
            //     line_coordinate_system_origin.x,
            //     line_coordinate_system_origin.y,
            //     line_coordinate_system_origin.z,
            // );
            // println!(
            //     "    x_axis=({:.8}, {:.8}, {:.8})",
            //     line_coordinate_system_x_axis.x,
            //     line_coordinate_system_x_axis.y,
            //     line_coordinate_system_x_axis.z,
            // );
            // println!(
            //     "    y_axis=({:.8}, {:.8}, {:.8})",
            //     line_coordinate_system_y_axis.x,
            //     line_coordinate_system_y_axis.y,
            //     line_coordinate_system_y_axis.z,
            // );
            // println!(
            //     "    target_angle_degrees={:.8}",
            //     coordinated_regioned_angle.get_angle_degrees(),
            // );
            line_coordinate_system
                .rotate(coordinated_regioned_angle.get_regioned_angle().inverted());
            // let rotated_origin = line_coordinate_system.to_global(CoordinatedPoint::new(
            //     line_coordinate_system.clone(),
            //     Vec3d::new(0.0, 0.0, 0.0),
            // ));
            // let rotated_x_axis_point = line_coordinate_system.to_global(CoordinatedPoint::new(
            //     line_coordinate_system.clone(),
            //     Vec3d::new(1.0, 0.0, 0.0),
            // ));
            // let rotated_y_axis_point = line_coordinate_system.to_global(CoordinatedPoint::new(
            //     line_coordinate_system.clone(),
            //     Vec3d::new(0.0, 1.0, 0.0),
            // ));
            // let rotated_x_axis = rotated_x_axis_point - rotated_origin;
            // let rotated_y_axis = rotated_y_axis_point - rotated_origin;
            // println!("  local line_coordinate_system after_rotate:");
            // println!(
            //     "    origin=({:.8}, {:.8}, {:.8})",
            //     rotated_origin.x,
            //     rotated_origin.y,
            //     rotated_origin.z,
            // );
            // println!(
            //     "    x_axis=({:.8}, {:.8}, {:.8})",
            //     rotated_x_axis.x,
            //     rotated_x_axis.y,
            //     rotated_x_axis.z,
            // );
            // println!(
            //     "    y_axis=({:.8}, {:.8}, {:.8})",
            //     rotated_y_axis.x,
            //     rotated_y_axis.y,
            //     rotated_y_axis.z,
            // );
            let x_line_start =
                CoordinatedPoint::new(line_coordinate_system.clone(), Vec3d::new(0.0, 0.0, 0.0));
            // let x_line_start_global = x_line_start.convert_to(global_coordinate_system.clone());
            // println!(
            //     "  local x_line_start=({:.8}, {:.8}, {:.8})",
            //     x_line_start_global.get_x(),
            //     x_line_start_global.get_y(),
            //     x_line_start_global.get_z(),
            // );
            let x_line_end = CoordinatedPoint::new(
                line_coordinate_system.clone(),
                Vec3d::new(1.1 * radius, 0.0, 0.0),
            );
            // let x_line_end_global = x_line_end.convert_to(global_coordinate_system.clone());
            // println!(
            //     "  local x_line_end=({:.8}, {:.8}, {:.8})",
            //     x_line_end_global.get_x(),
            //     x_line_end_global.get_y(),
            //     x_line_end_global.get_z(),
            // );
            let x_axis_line = CoordinatedLine::new(x_line_start, x_line_end);
            // println!("  local x_axis_line created");
            let clipped_line = coordinated_rectangle.get_intersection_line(x_axis_line);
            // println!("  local clipped_line.is_some = {}", clipped_line.is_some());
            if let Some(clipped_line) = clipped_line {
                // let clipped_line_start_global =
                //     clipped_line.get_start().convert_to(global_coordinate_system.clone());
                // let clipped_line_end_global =
                //     clipped_line.get_end().convert_to(global_coordinate_system.clone());
                // println!("  local clipped_line:");
                // println!(
                //     "    start=({:.8}, {:.8}, {:.8})",
                //     clipped_line_start_global.get_x(),
                //     clipped_line_start_global.get_y(),
                //     clipped_line_start_global.get_z(),
                // );
                // println!(
                //     "    end=({:.8}, {:.8}, {:.8})",
                //     clipped_line_end_global.get_x(),
                //     clipped_line_end_global.get_y(),
                //     clipped_line_end_global.get_z(),
                // );
                let polar_start = PolarCoordinates::new(
                    clipped_line.get_start().get_x() / radius,
                    coordinated_regioned_angle.clone(),
                );
                // println!(
                //     "  local polar_start radius={:.8} angle_degrees={:.8}",
                //     polar_start.get_radius(),
                //     polar_start.get_angle().get_angle_degrees(),
                // );
                let polar_end = PolarCoordinates::new(
                    clipped_line.get_end().get_x() / radius,
                    coordinated_regioned_angle.clone(),
                );
                // println!(
                //     "  local polar_end radius={:.8} angle_degrees={:.8}",
                //     polar_end.get_radius(),
                //     polar_end.get_angle().get_angle_degrees(),
                // );
                // check that the absoulte of the y coordinates is below 1e-4
                // println!(
                //     "  local clipped_line.start.y.abs = {:.12}",
                //     clipped_line.get_start().get_y().abs(),
                // );
                // println!(
                //     "  local clipped_line.end.y.abs = {:.12}",
                //     clipped_line.get_end().get_y().abs(),
                // );
                assert!(clipped_line.get_start().get_y().abs() < 1e-4);
                assert!(clipped_line.get_end().get_y().abs() < 1e-4);

                let slice = PolarSlice::new(polar_start, polar_end);
                // let created_slice_start_cartesian = slice.get_start().to_cartesian();
                // let created_slice_start_cartesian_global =
                //     created_slice_start_cartesian.convert_to(global_coordinate_system.clone());
                // let created_slice_end_cartesian = slice.get_end().to_cartesian();
                // let created_slice_end_cartesian_global =
                //     created_slice_end_cartesian.convert_to(global_coordinate_system.clone());
                // println!("  local created_polar_slice:");
                // println!(
                //     "    start_radius={:.8}",
                //     slice.get_start().get_radius(),
                // );
                // println!(
                //     "    start_angle_degrees={:.8}",
                //     slice.get_start().get_angle().get_angle_degrees(),
                // );
                // println!(
                //     "    start_cartesian=({:.8}, {:.8}, {:.8})",
                //     created_slice_start_cartesian_global.get_x(),
                //     created_slice_start_cartesian_global.get_y(),
                //     created_slice_start_cartesian_global.get_z(),
                // );
                // println!(
                //     "    end_radius={:.8}",
                //     slice.get_end().get_radius(),
                // );
                // println!(
                //     "    end_angle_degrees={:.8}",
                //     slice.get_end().get_angle().get_angle_degrees(),
                // );
                // println!(
                //     "    end_cartesian=({:.8}, {:.8}, {:.8})",
                //     created_slice_end_cartesian_global.get_x(),
                //     created_slice_end_cartesian_global.get_y(),
                //     created_slice_end_cartesian_global.get_z(),
                // );
                slices.push(Some(slice));
                // println!("  local slices.len after push = {}", slices.len());
            } else {
                slices.push(None);
            }
        }
        x += step;
        // println!("  local x after increment = {:.8}", x);
        // println!("loop iteration {iteration}: end");
        // iteration += 1;
    }
    // println!(
    //     "deduce_slices_from_mosaic: pre-combine slices.len = {}",
    //     slices.len()
    // );
    // for (slice_index, slice) in slices.iter().enumerate() {
    //     println!(
    //         "  pre-combine slice[{slice_index}] start_radius={:.8} end_radius={:.8} angle_degrees={:.8}",
    //         slice.get_start().get_radius(),
    //         slice.get_end().get_radius(),
    //         slice.get_start().get_angle().get_angle_degrees(),
    //     );
    // }
    let combined = combine_close_slices(slices);
    // println!(
    //     "deduce_slices_from_mosaic: post-combine combined.len = {}",
    //     combined.len()
    // );
    // for (slice_index, slice) in combined.iter().enumerate() {
    //     println!(
    //         "  combined slice[{slice_index}] start_radius={:.8} end_radius={:.8} angle_degrees={:.8}",
    //         slice.get_start().get_radius(),
    //         slice.get_end().get_radius(),
    //         slice.get_start().get_angle().get_angle_degrees(),
    //     );
    // }
    // println!("deduce_slices_from_mosaic: end");
    combined
}

fn combine_close_slices(slices: Vec<Option<PolarSlice>>) -> Vec<PolarSlice> {
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
                current_slice = slice.clone();
            }
            (None, None) => {}
        }
    }
    if let Some(item) = current_slice {
        combined_slices.push(item);
    }

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

    fn blank_image(width: u32, height: u32) -> WrappedRgbImage {
        WrappedRgbImage::new(ImageBuffer::from_pixel(width, height, Rgb([0, 0, 0])))
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

    fn add_horizontal_slice(
        slice_matrix: &mut SliceMatrix,
        line_number: usize,
        start_x: f64,
        end_x: f64,
    ) {
        let slice = Slice::new(
            point(start_x, line_number as f64),
            point(end_x, line_number as f64),
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

        for y in top_left.y as usize..bottom_right.y as usize {
            add_horizontal_slice(&mut slice_matrix, y, start_x, end_x);
        }

        slice_matrix
    }

    fn circle_slice_matrix(width: u32, height: u32, center: Vec3d, radius: f64) -> SliceMatrix {
        let mut slice_matrix = SliceMatrix::new(blank_image(width, height));
        let start_y = (center.y - radius).floor().max(0.0) as usize;
        let end_y = (center.y + radius).ceil().min(height as f64) as usize;
        let max_x = width.saturating_sub(1) as f64;

        for y in start_y..end_y {
            let dy = y as f64 - center.y;
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
        let mosaics = slice_matrices.into_iter().map(WrappedMosaic::new).collect();
        Trace::new_from_mosaics(mosaics, params)
    }

    fn assert_trace_matches_expected_lines(trace: &Trace, expected: &[Option<(f64, f64)>]) {
        assert_eq!(trace.ratio_lines.len(), expected.len());

        for (index, expected_slice) in expected.iter().enumerate() {
            let ratio_line = &trace.ratio_lines[index];
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
        let ratio = Ratio { from: 0.1, to: 0.9 };
        let tagged_ratio = TaggedRatio {
            ratio: ratio.clone(),
            left_tag: Tag::Empty,
            right_tag: Tag::Filled,
        };
        let line = ratio_line(&[(0.1, 0.4), (0.6, 0.8)]);

        assert_float_eq(ratio.from, 0.1);
        assert_float_eq(ratio.to, 0.9);
        assert_float_eq(tagged_ratio.ratio.from, 0.1);
        assert_eq!(tagged_ratio.left_tag, Tag::Empty);
        assert_eq!(tagged_ratio.right_tag, Tag::Filled);
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
        assert_eq!(overlaps[0].left_tag, Tag::Filled);
        assert_eq!(overlaps[0].right_tag, Tag::Filled);
        assert_float_eq(overlaps[2].ratio.from, 0.3);
        assert_float_eq(overlaps[2].ratio.to, 0.4);
        assert_eq!(overlaps[2].left_tag, Tag::Empty);
        assert_eq!(overlaps[2].right_tag, Tag::Empty);
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
    fn combine_close_slices_merges_adjacent_entries_and_splits_at_none() {
        let slices = vec![
            Some(polar_slice(0.1, 0.2)),
            Some(polar_slice(0.24, 0.3)),
            None,
            Some(polar_slice(0.6, 0.7)),
        ];

        let combined = combine_close_slices(slices);

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

        let radius = deduce_longest_radius(&mosaics, center);

        assert_float_eq(radius, 4.038873605350878);
    }

    #[test]
    fn deduce_slices_from_mosaic_produces_ordered_finite_slices() {
        let mosaic = square_mosaic();
        let coordinate_system = WrappedCoordinateSystem::new(
            mosaic.get_center_of_mass().get_local_point(),
            Vec3d::new(1.0, 0.0, 0.0),
            Vec3d::new(0.0, 1.0, 0.0),
        );

        let slices = deduce_slices_from_mosaic(
            vec![mosaic.clone()],
            CoordinatedRegionedAngle::new(coordinate_system, RegionedAngle::new(45.0, 0.0, 360.0)),
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
            TraceParams::new(36, 1e-4),
        );

        let expected = [
            Some((0.0, 0.78153907)),
            Some((0.0, 0.79359557)),
            Some((0.0, 0.83169651)),
            Some((0.0, 0.90244359)),
            Some((0.0, 1.02022680)),
            Some((0.0, 1.10000000)),
            Some((0.0, 0.98839060)),
            Some((0.0, 0.91090570)),
            Some((0.0, 0.86917610)),
            Some((0.0, 0.85597137)),
            Some((0.0, 0.86917610)),
            Some((0.0, 0.91090570)),
            Some((0.0, 0.98839060)),
            Some((0.0, 1.10000000)),
            Some((0.0, 1.02022680)),
            Some((0.0, 0.90244359)),
            Some((0.0, 0.83169651)),
            Some((0.0, 0.79359557)),
            Some((0.0, 0.78153907)),
            Some((0.0, 0.79359557)),
            Some((0.0, 0.83169651)),
            Some((0.0, 0.90244359)),
            Some((0.0, 1.02022680)),
            Some((0.0, 1.02022680)),
            Some((0.0, 0.90244359)),
            Some((0.0, 0.83169651)),
            Some((0.0, 0.79359557)),
            Some((0.0, 0.78153907)),
            Some((0.0, 0.79359557)),
            Some((0.0, 0.83169651)),
            Some((0.0, 0.90244359)),
            Some((0.0, 1.02022680)),
            Some((0.0, 1.02022680)),
            Some((0.0, 0.90244359)),
            Some((0.0, 0.83169651)),
            Some((0.0, 0.79359557)),
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
            TraceParams::new(36, 1e-4),
        );

        let expected = [
            Some((0.0, 0.99979595)),
            Some((0.0, 1.01521942)),
            Some((0.0, 1.05330933)),
            Some((0.0, 1.04027749)),
            Some((0.0, 1.04424592)),
            Some((0.0, 1.04424592)),
            Some((0.0, 1.06215485)),
            Some((0.0, 1.06396063)),
            Some((0.0, 1.05580748)),
            Some((0.0, 1.03976739)),
            Some((0.0, 1.05580748)),
            Some((0.0, 1.06396063)),
            Some((0.0, 1.06215485)),
            Some((0.0, 1.04424592)),
            Some((0.0, 1.04291414)),
            Some((0.0, 1.04027749)),
            Some((0.0, 1.05330933)),
            Some((0.0, 1.01418348)),
            Some((0.0, 1.03874719)),
            Some((0.0, 1.01418348)),
            Some((0.0, 1.02033825)),
            Some((0.0, 1.01482179)),
            Some((0.0, 1.04291414)),
            Some((0.0, 1.04291414)),
            Some((0.0, 1.01482179)),
            Some((0.0, 1.02033825)),
            Some((0.0, 1.01418348)),
            Some((0.0, 1.03874719)),
            Some((0.0, 1.01418348)),
            Some((0.0, 1.02033825)),
            Some((0.0, 1.01482179)),
            Some((0.0, 1.04291414)),
            Some((0.0, 1.04424592)),
            Some((0.0, 1.01599982)),
            Some((0.0, 1.02142392)),
            Some((0.0, 1.01521942)),
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
            TraceParams::new(36, 1e-4),
        );

        let expected = [
            Some((0.0, 0.52321664)),
            Some((0.0, 0.53128810)),
            Some((0.0, 0.55679552)),
            Some((0.0, 0.60415854)),
            Some((0.0, 0.66598413)),
            Some((0.0, 0.80719461)),
            Some((0.0, 1.04354657)),
            Some((0.0, 1.10000000)),
            Some((0.0, 1.10000000)),
            Some((0.0, 1.09399844)),
            Some((0.0, 1.10000000)),
            Some((0.0, 1.10000000)),
            Some((0.0, 1.04354657)),
            Some((0.0, 0.80719461)),
            Some((0.0, 0.66598413)),
            Some((0.0, 0.60415854)),
            Some((0.0, 0.55679552)),
            Some((0.0, 0.53128810)),
            Some((0.0, 0.52321664)),
            Some((0.0, 0.53128810)),
            Some((0.0, 0.55679552)),
            Some((0.0, 0.60415854)),
            Some((0.0, 0.66598413)),
            Some((0.0, 0.80719461)),
            Some((0.0, 1.04354657)),
            Some((0.0, 1.06297327)),
            Some((0.0, 1.01427729)),
            Some((0.0, 0.99886814)),
            Some((0.0, 1.01427729)),
            Some((0.0, 1.06297327)),
            Some((0.0, 1.04354657)),
            Some((0.0, 0.80719461)),
            Some((0.0, 0.66598413)),
            Some((0.0, 0.60415854)),
            Some((0.0, 0.55679552)),
            Some((0.0, 0.53128810)),
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
            TraceParams::new(24, 1e-4),
        );

        let expected = [
            Some((0.30653137, 0.98412702)),
            Some((0.31734462, 1.01884326)),
            Some((0.35493106, 0.74212857)),
            Some((0.43350082, 0.52476414)),
            None,
            None,
            None,
            None,
            None,
            Some((0.43350082, 0.52476414)),
            Some((0.35493106, 0.74212857)),
            Some((0.31734462, 1.01884326)),
            Some((0.30653137, 0.98412702)),
            Some((0.31734462, 1.01884326)),
            Some((0.35493106, 0.67759565)),
            Some((0.43350082, 0.47913248)),
            None,
            None,
            None,
            None,
            None,
            Some((0.43350082, 0.47913248)),
            Some((0.35493106, 0.67759565)),
            Some((0.31734462, 1.01884326)),
        ];

        assert_trace_matches_expected_lines(&trace, &expected);
    }

    #[test]
    fn trace_new_from_mosaic_builds_requested_number_of_ratio_lines() {
        let trace = Trace::new_from_mosaic(square_mosaic(), TraceParams::new(18, 0.2));
        let self_similarity = trace.compare_with(0.0, &trace.clone());

        assert_eq!(trace.ratio_lines.len(), 18);
        assert!(trace.ratio_lines.iter().any(|line| !line.slices.is_empty()));
        assert!(self_similarity >= 1.0);
    }

    #[test]
    fn trace_compare_with_returns_zero_when_target_similarity_is_unreachable() {
        let trace = Trace::new_from_mosaic(square_mosaic(), TraceParams::new(18, 0.2));
        let self_similarity = trace.compare_with(0.0, &trace.clone());

        assert_float_eq(
            trace.compare_with(self_similarity + EPSILON, &trace.clone()),
            0.0,
        );
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
        let similarity = combined.compare_with(0.0, &same_family);

        assert_eq!(combined.ratio_lines.len(), 18);
        assert!(similarity >= 1.0);
        assert_float_eq(
            combined.compare_with(similarity + EPSILON, &same_family),
            0.0,
        );
    }
}

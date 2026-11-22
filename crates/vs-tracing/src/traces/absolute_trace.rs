#![warn(clippy::pedantic)]
#![allow(
    clippy::missing_errors_doc,
    clippy::missing_panics_doc,
    clippy::too_many_lines,
    clippy::unused_async_trait_impl
)]

use crate::math::CoordinatedRegionedAngle;
use crate::math::RegionedAngle;
use crate::ring::Pizza;
use crate::traces::PolarSlice;
use crate::traces::Skeleton;
use crate::traces::Tag;
use crate::traces::trace::TraceSource;
use std::fmt::Write as _;
use vs_math::math::PolarCoordinates;

use super::{
    RatioLine, TraceParams, deduce_slices_from_pizza, f64_from_usize, get_overlaps,
};

#[derive(Clone)]
enum CombineType {
    Add,
    Subtract,
}

#[derive(Clone)]
pub struct AbsoluteTrace {
    source: Option<TraceSource>,
    params: TraceParams,
    skeleton: Skeleton,
    ratio_lines: Vec<RatioLine>,
    total_mass: f64,
    total_surrounding_circle_area: f64,
}

impl AbsoluteTrace {
    #[cfg(test)]
    pub(super) fn new(
        ratio_lines: Vec<RatioLine>,
        total_mass: f64,
        total_surrounding_circle_area: f64,
    ) -> Self {
        Self {
            source: None,
            params: TraceParams::new(ratio_lines.len()),
            skeleton: Skeleton::default(),
            ratio_lines,
            total_mass,
            total_surrounding_circle_area,
        }
    }
}

impl AbsoluteTrace {
    pub fn new_from_pizza(skeleton: Skeleton, pizza: &Pizza) -> Self {
        let ratio_lines = Self::ratio_lines_from_pizza(&skeleton, pizza);
        Self {
            source: Some(TraceSource::Pizza),
            params: TraceParams::new(skeleton.num_skeleton()),
            skeleton: skeleton.clone(),
            ratio_lines,
            total_mass: pizza.get_area(),
            total_surrounding_circle_area: skeleton.get_area(),
        }
    }

    pub fn params(&self) -> &TraceParams {
        &self.params
    }

    pub fn skeleton(&self) -> &Skeleton {
        &self.skeleton
    }
    
    pub fn total_mass(&self) -> f64 {
        self.total_mass
    }

    pub fn total_surrounding_circle_area(&self) -> f64 {
        self.total_surrounding_circle_area
    }

    pub fn ratio_lines(&self) -> &Vec<RatioLine> {
        &self.ratio_lines
    }

    pub fn add_trace(&mut self, other_trace: AbsoluteTrace) {
        // the caller needs to ensure that both skeletons match if they want that
        self.ratio_lines = Self::combine_ratio_lines(&self.ratio_lines, &other_trace.ratio_lines, &self.skeleton, CombineType::Add);
    }

    pub fn subtract_trace(&mut self, other_trace: AbsoluteTrace) {
        // the caller needs to ensure that both skeletons match if they want that
        self.ratio_lines = Self::combine_ratio_lines(&self.ratio_lines, &other_trace.ratio_lines, &self.skeleton, CombineType::Subtract);
    }

    fn combine_ratio_lines(this_ratio_lines: &[RatioLine], other_ratio_lines: &[RatioLine], skeleton: &Skeleton, combine_type: CombineType) -> Vec<RatioLine> {
        let mut combined_ratio_lines = Vec::with_capacity(this_ratio_lines.len());
        let coordinate_system = skeleton.get_coordinate_system();
        for (i, (this_ratio_line, other_ratio_line)) in this_ratio_lines
            .iter()
            .zip(other_ratio_lines.iter())
            .enumerate()
        {
            let angle_degrees = skeleton.get_delta_angle_degrees() * i as f64;
            let regioned_angle = RegionedAngle::new(angle_degrees, 0.0, 360.0);
            let coordinated_regioned_angle =
                CoordinatedRegionedAngle::new(coordinate_system.clone(), regioned_angle);
            combined_ratio_lines.push(Self::combine_ratio_line(
                this_ratio_line,
                other_ratio_line,
                &coordinated_regioned_angle,
                combine_type.clone(),
            ));
        }
        combined_ratio_lines
    }

    fn combine_ratio_line(
        ratio_line1: &RatioLine,
        ratio_line2: &RatioLine,
        coordinated_regioned_angle: &CoordinatedRegionedAngle,
        combine_type: CombineType,
    ) -> RatioLine {
        let overlaps = get_overlaps(ratio_line1, ratio_line2);
        let filled_overlaps = overlaps.iter().filter(|overlap| {
            match combine_type {
                CombineType::Add => {
                    overlap.left_tag() == Tag::Filled || overlap.right_tag() == Tag::Filled
                }
                CombineType::Subtract => {
                    overlap.left_tag() == Tag::Filled && overlap.right_tag() == Tag::Empty
                }
            }
        });
        let polar_slices: Vec<_> = filled_overlaps
            .map(|overlap| {
                let polar_coordinates_start = PolarCoordinates::new(
                    overlap.ratio().from(),
                    coordinated_regioned_angle.clone(),
                );
                let polar_coordinates_end =
                    PolarCoordinates::new(overlap.ratio().to(), coordinated_regioned_angle.clone());
                PolarSlice::new(polar_coordinates_start, polar_coordinates_end)
            })
            .collect();
        RatioLine::new(polar_slices)
    }

    fn ratio_lines_from_pizza(skeleton: &Skeleton, pizza: &Pizza) -> Vec<RatioLine> {
        let base_coordinate_system = skeleton.get_coordinate_system();
        (0..skeleton.num_skeleton())
            .map(|i| {
                let coordinated_regioned_angle = CoordinatedRegionedAngle::new(
                    base_coordinate_system.duplicate(),
                    RegionedAngle::new(
                        f64_from_usize(i) * (360.0 / f64_from_usize(skeleton.num_skeleton())),
                        0.0,
                        360.0,
                    ),
                );
                RatioLine::new(deduce_slices_from_pizza(
                    pizza,
                    &coordinated_regioned_angle,
                    skeleton.get_radius(),
                ))
            })
            .collect()
    }

    pub fn to_pizza(&self) -> Pizza {
        let mut pizza = Pizza::new(Vec::new());
        let delta_angle_degrees = 360.0 / f64_from_usize(self.ratio_lines.len());
        for ratio_line in &self.ratio_lines {
            for slice in ratio_line.slices() {
                let coordinated_regioned_delta_angle = CoordinatedRegionedAngle::new(
                    slice
                        .get_start()
                        .get_angle()
                        .get_coordinate_system()
                        .duplicate(),
                    RegionedAngle::new(delta_angle_degrees, -180.0, 180.0),
                );
                pizza.add_ring_sector(
                    slice
                        .get_ring_sector(coordinated_regioned_delta_angle)
                        .clone(),
                );
            }
        }
        pizza
    }

    #[must_use]
    pub fn dump_details(&mut self) -> String {
        let mut output = String::new();
        let _ = writeln!(
            output,
            "Trace {{ ratio_line_count: {} }}",
            self.ratio_lines.len()
        );

        for (line_index, ratio_line) in self.ratio_lines.iter().enumerate() {
            let _ = writeln!(
                output,
                "  ratio_line[{line_index}] {{ slice_count: {} }}",
                ratio_line.slices().len()
            );

            for (slice_index, slice) in ratio_line.slices().iter().enumerate() {
                let start = slice.get_start();
                let end = slice.get_end();
                let start_cartesian = start.to_cartesian();
                let end_cartesian = end.to_cartesian();

                let _ = writeln!(
                    output,
                    "    slice[{slice_index}] {{ start_radius: {:.8}, start_angle_degrees: {:.8}, end_radius: {:.8}, end_angle_degrees: {:.8}, start_cartesian: ({:.8}, {:.8}, {:.8}), end_cartesian: ({:.8}, {:.8}, {:.8}) }}",
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
                );
            }
        }

        output
    }
}

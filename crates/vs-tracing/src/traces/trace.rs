#![warn(clippy::pedantic)]
#![allow(
    clippy::missing_errors_doc,
    clippy::missing_panics_doc,
    clippy::too_many_lines,
    clippy::unused_async_trait_impl
)]

use crate::math::AnonymizedCoordinateSystem;
use crate::math::CoordinateSystem;
use crate::math::CoordinatedRegionedAngle;
use crate::math::RegionedAngle;
use crate::mosaics::WrappedMosaic;
use crate::ring::Pizza;
use crate::traces::PolarSlice;
use crate::traces::Skeleton;
use crate::traces::Tag;
use rs_math3d::Vec3d;
use std::fmt::Write as _;
use vs_math::math::PolarCoordinates;

use super::{
    RatioLine, TraceParams, calculate_center_of_mass, calculate_total_mass,
    calculate_total_surrounding_circle_area, compare_with_rotation, deduce_longest_radius,
    deduce_slices_from_mosaic, deduce_slices_from_pizza, f64_from_usize, get_overlaps,
    trace_debug_enabled,
};

#[derive(Clone)]
enum TraceSource {
    Single(WrappedMosaic),
    Multiple(Vec<WrappedMosaic>),
    Pizza,
}

#[derive(Clone)]
enum CombineType {
    Add,
    Subtract,
}

#[derive(Clone)]
pub struct Trace {
    source: Option<TraceSource>,
    params: TraceParams,
    skeleton: Skeleton,
    ratio_lines: Vec<RatioLine>,
    ratio_lines_populated: bool,
    total_mass: f64,
    total_surrounding_circle_area: f64,
}

impl Trace {
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
            ratio_lines_populated: true,
            total_mass,
            total_surrounding_circle_area,
        }
    }

    #[cfg(test)]
    pub(super) fn ratio_lines(&mut self) -> &Vec<RatioLine> {
        self.populate_ratio_lines();
        &self.ratio_lines
    }
}

impl Trace {
    /// Creates a trace whose ratio lines are not computed yet; call
    #[must_use]
    pub fn new_from_mosaic(mosaic: &WrappedMosaic, params: TraceParams) -> Self {
        Trace {
            source: Some(TraceSource::Single(mosaic.clone())),
            params,
            skeleton: Skeleton::default(),
            ratio_lines: Vec::new(),
            ratio_lines_populated: false,
            total_mass: calculate_total_mass(std::slice::from_ref(mosaic)),
            total_surrounding_circle_area: calculate_total_surrounding_circle_area(
                std::slice::from_ref(mosaic),
            ),
        }
    }

    /// Creates a trace whose ratio lines are not computed yet; call
    #[must_use]
    pub fn new_from_mosaics(mosaics: &[WrappedMosaic], params: TraceParams) -> Self {
        Trace {
            source: Some(TraceSource::Multiple(mosaics.to_vec())),
            params,
            skeleton: Skeleton::default(),
            ratio_lines: Vec::new(),
            ratio_lines_populated: false,
            total_mass: calculate_total_mass(mosaics),
            total_surrounding_circle_area: calculate_total_surrounding_circle_area(mosaics),
        }
    }

    pub fn new_from_pizza(skeleton: Skeleton, pizza: &Pizza) -> Self {
        let ratio_lines = Self::ratio_lines_from_pizza(&skeleton, pizza);
        Self {
            source: Some(TraceSource::Pizza),
            params: TraceParams::new(skeleton.num_skeleton()),
            skeleton: skeleton.clone(),
            ratio_lines,
            ratio_lines_populated: true,
            total_mass: pizza.get_area(),
            total_surrounding_circle_area: skeleton.get_area(),
        }
    }

    pub fn add_trace(&mut self, other_trace: Trace) {
        // the caller needs to ensure that both skeletons match if they want that
        self.ratio_lines = Self::combine_ratio_lines(&self.ratio_lines, &other_trace.ratio_lines, &self.skeleton, CombineType::Add);
    }

    pub fn subtract_trace(&mut self, other_trace: Trace) {
        // the caller needs to ensure that both skeletons match if they want that
        self.ratio_lines = Self::combine_ratio_lines(&self.ratio_lines, &other_trace.ratio_lines, &self.skeleton, CombineType::Subtract);
    }

    #[cfg(test)]
    pub(super) fn ratio_lines_populated(&self) -> bool {
        self.ratio_lines_populated
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

    /// Computes and caches the ratio lines. Does nothing if already populated.
    fn populate_ratio_lines(&mut self) {
        if self.ratio_lines_populated {
            return;
        }
        let (skeleton, ratio_lines) = match &self.source {
            Some(TraceSource::Single(mosaic)) => Self::ratio_lines_from_mosaic(mosaic, self.params),
            Some(TraceSource::Multiple(mosaics)) => {
                Self::ratio_lines_from_mosaics(mosaics, self.params)
            }
            Some(TraceSource::Pizza) => {
                let skeleton = self.skeleton.clone();
                let ratio_lines = self.ratio_lines.clone();
                (skeleton, ratio_lines)
            }
            None => (Skeleton::default(), Vec::new()),
        };
        self.skeleton = skeleton;
        self.ratio_lines = ratio_lines;
        self.ratio_lines_populated = true;
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

    fn ratio_lines_from_mosaic(
        mosaic: &WrappedMosaic,
        params: TraceParams,
    ) -> (Skeleton, Vec<RatioLine>) {
        let global_coordinate_system = AnonymizedCoordinateSystem::Direct(CoordinateSystem::new(
            Vec3d::new(0.0, 0.0, 0.0),
            Vec3d::new(1.0, 0.0, 0.0),
            Vec3d::new(0.0, 1.0, 0.0),
        ));

        let coordinate_system = AnonymizedCoordinateSystem::Direct(CoordinateSystem::new(
            mosaic
                .get_center_of_mass()
                .convert_to(global_coordinate_system.clone())
                .get_local_point(),
            Vec3d::new(1.0, 0.0, 0.0),
            Vec3d::new(0.0, 1.0, 0.0),
        ));
        let skeleton = Skeleton::new_from_trace_params(
            coordinate_system.duplicate(),
            params,
            mosaic.get_bounding_circle().get_radius(),
        );
        let ratio_lines = (0..params.num_skeleton())
            .map(|i| {
                let coordinated_regioned_angle = CoordinatedRegionedAngle::new(
                    coordinate_system.duplicate(),
                    RegionedAngle::new(
                        f64_from_usize(i) * (360.0 / f64_from_usize(params.num_skeleton())),
                        0.0,
                        360.0,
                    ),
                );
                RatioLine::new(deduce_slices_from_mosaic(
                    std::slice::from_ref(mosaic),
                    &coordinated_regioned_angle,
                    mosaic.get_bounding_circle().get_radius(),
                ))
            })
            .collect();
        (skeleton, ratio_lines)
    }

    fn ratio_lines_from_mosaics(
        mosaics: &[WrappedMosaic],
        params: TraceParams,
    ) -> (Skeleton, Vec<RatioLine>) {
        let global_coordinate_system = AnonymizedCoordinateSystem::Direct(CoordinateSystem::new(
            Vec3d::new(0.0, 0.0, 0.0),
            Vec3d::new(1.0, 0.0, 0.0),
            Vec3d::new(0.0, 1.0, 0.0),
        ));
        let center_of_mass =
            calculate_center_of_mass(mosaics).convert_to(global_coordinate_system.clone());
        let coordinate_system = AnonymizedCoordinateSystem::Direct(CoordinateSystem::new(
            center_of_mass.clone().get_local_point(),
            Vec3d::new(1.0, 0.0, 0.0),
            Vec3d::new(0.0, 1.0, 0.0),
        ));
        let longest_radius = deduce_longest_radius(mosaics, &center_of_mass);
        let skeleton =
            Skeleton::new_from_trace_params(coordinate_system.duplicate(), params, longest_radius);
        let ratio_lines = (0..params.num_skeleton())
            .map(|i| {
                let coordinated_regioned_angle = CoordinatedRegionedAngle::new(
                    coordinate_system.duplicate(),
                    RegionedAngle::new(
                        f64_from_usize(i) * (360.0 / f64_from_usize(params.num_skeleton())),
                        0.0,
                        360.0,
                    ),
                );
                RatioLine::new(deduce_slices_from_mosaic(
                    mosaics,
                    &coordinated_regioned_angle,
                    longest_radius,
                ))
            })
            .collect();
        (skeleton, ratio_lines)
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
    pub fn compare_with(&mut self, target_similarity: f64, other: &mut Trace) -> f64 {
        self.populate_ratio_lines();
        other.populate_ratio_lines();
        let mut highest_similarity = 0.0;
        let first_factor = self.total_mass / self.total_surrounding_circle_area;
        let second_factor = other.total_mass / other.total_surrounding_circle_area;
        let closeness =
            (first_factor.min(second_factor) / first_factor.max(second_factor)).max(0.0);

        for i in 0..self.ratio_lines.len() {
            let similarity = compare_with_rotation(&self.ratio_lines, &other.ratio_lines, i);
            let adjusted_similarity = similarity * closeness;
            if trace_debug_enabled() {
                println!(
                    "trace.compare_with rotation={i} similarity={similarity:.8} highest_before={highest_similarity:.8} target={target_similarity:.8}",
                );
            }
            if adjusted_similarity > highest_similarity {
                highest_similarity = adjusted_similarity;
            }
            if highest_similarity >= target_similarity {
                break;
            }
        }
        if trace_debug_enabled() {
            println!("trace.compare_with final highest_similarity={highest_similarity:.8}");
        }
        highest_similarity
    }

    #[must_use]
    pub fn dump_details(&mut self) -> String {
        self.populate_ratio_lines();
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

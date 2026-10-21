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
use rs_math3d::Vec3d;
use std::fmt::Write as _;

use super::{
    RatioLine, TraceParams, calculate_center_of_mass, calculate_total_mass,
    calculate_total_surrounding_circle_area, compare_with_rotation, deduce_longest_radius,
    deduce_slices_from_mosaic, f64_from_usize, trace_debug_enabled,
};

#[derive(Clone)]
pub struct Trace {
    ratio_lines: Vec<RatioLine>,
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
            ratio_lines,
            total_mass,
            total_surrounding_circle_area,
        }
    }

    #[cfg(test)]
    pub(super) fn ratio_lines(&self) -> &Vec<RatioLine> {
        &self.ratio_lines
    }
}

impl Trace {
    #[must_use]
    pub fn new_from_mosaic(mosaic: &WrappedMosaic, params: TraceParams) -> Self {
        let global_coordinate_system = AnonymizedCoordinateSystem::Direct(CoordinateSystem::new(
            Vec3d::new(0.0, 0.0, 0.0),
            Vec3d::new(1.0, 0.0, 0.0),
            Vec3d::new(0.0, 1.0, 0.0),
        ));
        let ratio_lines = (0..params.num_skeleton())
            .map(|i| {
                let coordinate_system = AnonymizedCoordinateSystem::Direct(CoordinateSystem::new(
                    mosaic
                        .get_center_of_mass()
                        .convert_to(global_coordinate_system.clone())
                        .get_local_point(),
                    Vec3d::new(1.0, 0.0, 0.0),
                    Vec3d::new(0.0, 1.0, 0.0),
                ));
                let coordinated_regioned_angle = CoordinatedRegionedAngle::new(
                    coordinate_system,
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
        Trace {
            ratio_lines,
            total_mass: calculate_total_mass(std::slice::from_ref(mosaic)),
            total_surrounding_circle_area: calculate_total_surrounding_circle_area(
                std::slice::from_ref(mosaic),
            ),
        }
    }

    #[must_use]
    pub fn new_from_mosaics(mosaics: &[WrappedMosaic], params: TraceParams) -> Self {
        let global_coordinate_system = AnonymizedCoordinateSystem::Direct(CoordinateSystem::new(
            Vec3d::new(0.0, 0.0, 0.0),
            Vec3d::new(1.0, 0.0, 0.0),
            Vec3d::new(0.0, 1.0, 0.0),
        ));
        let center_of_mass =
            calculate_center_of_mass(mosaics).convert_to(global_coordinate_system.clone());
        let ratio_lines = (0..params.num_skeleton())
            .map(|i| {
                let coordinate_system = AnonymizedCoordinateSystem::Direct(CoordinateSystem::new(
                    center_of_mass.clone().get_local_point(),
                    Vec3d::new(1.0, 0.0, 0.0),
                    Vec3d::new(0.0, 1.0, 0.0),
                ));
                let coordinated_regioned_angle = CoordinatedRegionedAngle::new(
                    coordinate_system,
                    RegionedAngle::new(
                        f64_from_usize(i) * (360.0 / f64_from_usize(params.num_skeleton())),
                        0.0,
                        360.0,
                    ),
                );
                RatioLine::new(deduce_slices_from_mosaic(
                    mosaics,
                    &coordinated_regioned_angle,
                    deduce_longest_radius(mosaics, &center_of_mass),
                ))
            })
            .collect();
        Trace {
            ratio_lines,
            total_mass: calculate_total_mass(mosaics),
            total_surrounding_circle_area: calculate_total_surrounding_circle_area(mosaics),
        }
    }

    #[must_use]
    pub fn compare_with(&self, target_similarity: f64, other: &Trace) -> f64 {
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
    pub fn dump_details(&self) -> String {
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

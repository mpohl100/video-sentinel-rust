#![warn(clippy::pedantic)]
#![allow(
    clippy::missing_errors_doc,
    clippy::missing_panics_doc,
    clippy::too_many_lines,
    clippy::unused_async_trait_impl
)]

use crate::eye::TileParams;
use crate::slices::RelativeRectangle;
use crate::slices::{Rectangle, WrappedRelativeRectangle};
use crate::traced_mosaics::TracedRelativeMosaic;
use rs_math3d::Vector3;
use std::collections::HashMap;

use super::{BucketedMosaicsPerSection, f64_from_usize, i64_from_f64, usize_from_f64};

#[derive(Clone)]
pub struct BucketedMosaics {
    sections: Vec<BucketedMosaicsPerSection>,
}

impl BucketedMosaics {
    #[cfg(test)]
    pub(super) fn sections(&self) -> &Vec<BucketedMosaicsPerSection> {
        &self.sections
    }

    #[must_use]
    pub fn new(tile_params: TileParams, delta: f64) -> Self {
        let regions = Self::calculate_regions(tile_params);
        let sections = regions
            .into_iter()
            .map(|region| BucketedMosaicsPerSection::new(region, delta))
            .collect();
        BucketedMosaics { sections }
    }

    pub fn add_mosaic(&mut self, mosaic: &TracedRelativeMosaic) {
        for section in &mut self.sections {
            section.add_mosaic(mosaic.clone());
        }
    }

    #[must_use]
    pub fn get_potentially_similar_mosaics(
        &self,
        mosaic: &TracedRelativeMosaic,
    ) -> Vec<TracedRelativeMosaic> {
        let mut mosaics_by_center_of_mass: HashMap<(i64, i64), TracedRelativeMosaic> =
            HashMap::new();
        for section in self.get_overlapping_sections(Rectangle::new_from_math_rectangle(
            &mosaic
                .get_relative_mosaic()
                .get_bounding_box()
                .to_global_rectangle(),
        )) {
            for candidate in section.get_potentially_similar_mosaics(mosaic) {
                let center_of_mass = candidate.get_relative_mosaic().get_center_of_mass();
                let key_tuple = (
                    i64_from_f64(center_of_mass.get_x() * 1_000_000.0),
                    i64_from_f64(center_of_mass.get_y() * 1_000_000.0),
                );
                mosaics_by_center_of_mass.insert(key_tuple, candidate);
            }
        }
        mosaics_by_center_of_mass.into_values().collect()
    }

    #[must_use]
    pub fn get_all_similar_mosaics(
        &self,
        mosaic: &TracedRelativeMosaic,
    ) -> Vec<TracedRelativeMosaic> {
        let mut mosaics_by_center_of_mass: HashMap<(i64, i64), TracedRelativeMosaic> =
            HashMap::new();
        for section in &self.sections {
            for candidate in section.get_potentially_similar_mosaics(mosaic) {
                let center_of_mass = candidate.get_relative_mosaic().get_center_of_mass();
                let key_tuple = (
                    i64_from_f64(center_of_mass.get_x() * 1_000_000.0),
                    i64_from_f64(center_of_mass.get_y() * 1_000_000.0),
                );
                mosaics_by_center_of_mass.insert(key_tuple, candidate);
            }
        }
        mosaics_by_center_of_mass.into_values().collect()
    }

    #[must_use]
    pub fn get_similar_mosaics_from_rectangle(
        &self,
        mosaic: &TracedRelativeMosaic,
        region: &WrappedRelativeRectangle,
    ) -> Vec<TracedRelativeMosaic> {
        let mut mosaics_by_center_of_mass: HashMap<(i64, i64), TracedRelativeMosaic> =
            HashMap::new();
        for section in self.get_overlapping_sections(region.to_rectangle()) {
            for candidate in section.get_potentially_similar_mosaics(mosaic) {
                let center_of_mass = candidate.get_relative_mosaic().get_center_of_mass();
                let key_tuple = (
                    i64_from_f64(center_of_mass.get_x() * 1_000_000.0),
                    i64_from_f64(center_of_mass.get_y() * 1_000_000.0),
                );
                mosaics_by_center_of_mass.insert(key_tuple, candidate);
            }
        }
        mosaics_by_center_of_mass.into_values().collect()
    }

    pub(super) fn get_overlapping_sections(
        &self,
        bounding_box: Rectangle,
    ) -> Vec<&BucketedMosaicsPerSection> {
        self.sections
            .iter()
            .filter(|section| section.region().overlaps(&bounding_box))
            .collect()
    }

    fn calculate_regions(tile_params: TileParams) -> Vec<WrappedRelativeRectangle> {
        let num_steps_x = usize_from_f64((1.0 / tile_params.relative_tile_x()).ceil());
        let num_steps_y = usize_from_f64((1.0 / tile_params.relative_tile_y()).ceil());
        let mut regions = Vec::new();
        for i in 0..num_steps_x {
            for j in 0..num_steps_y {
                let top_left_x = tile_params.relative_tile_x() * f64_from_usize(i);
                let top_left_y = tile_params.relative_tile_y() * f64_from_usize(j);
                let width = (1.0 - top_left_x).min(tile_params.relative_tile_x());
                let height = (1.0 - top_left_y).min(tile_params.relative_tile_y());
                let region = WrappedRelativeRectangle::new(RelativeRectangle::new(
                    Vector3::new(top_left_x, top_left_y, 0.0),
                    Vector3::new(width, height, 0.0),
                ));
                regions.push(region);
            }
        }
        regions
    }
}

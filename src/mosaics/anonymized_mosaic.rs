#![warn(clippy::pedantic)]
#![allow(
    clippy::missing_errors_doc,
    clippy::missing_panics_doc,
    clippy::too_many_lines,
    clippy::unused_async_trait_impl
)]

use crate::math::{CoordinatedCircle, CoordinatedPoint, CoordinatedRectangle};
use rs_math3d::Vec3d;

use super::{Results, WrappedMosaic, WrappedRelativeMosaic};

#[derive(Clone)]
pub enum AnonymizedMosaic {
    Absolute(WrappedMosaic),
    Relative(WrappedRelativeMosaic),
}

impl AnonymizedMosaic {
    #[must_use]
    pub fn new(
        results: Results,
        mosaic: WrappedMosaic,
        relative_mosaic: Option<WrappedRelativeMosaic>,
    ) -> Self {
        match results {
            Results::Absolute => AnonymizedMosaic::Absolute(mosaic),
            Results::Relative => AnonymizedMosaic::Relative(
                relative_mosaic.expect("Relative mosaic must be provided for Results::Relative"),
            ),
        }
    }

    #[must_use]
    pub fn get_mosaic(&self) -> WrappedMosaic {
        match self {
            AnonymizedMosaic::Absolute(mosaic) => mosaic.clone(),
            AnonymizedMosaic::Relative(relative_mosaic) => relative_mosaic.get_mosaic(),
        }
    }

    #[must_use]
    pub fn get_bounding_box(&self) -> CoordinatedRectangle {
        match self {
            AnonymizedMosaic::Absolute(mosaic) => mosaic.get_bounding_box(),
            AnonymizedMosaic::Relative(relative_mosaic) => relative_mosaic.get_bounding_box(),
        }
    }

    #[must_use]
    pub fn get_bounding_circle(&self) -> CoordinatedCircle {
        match self {
            AnonymizedMosaic::Absolute(mosaic) => mosaic.get_bounding_circle(),
            AnonymizedMosaic::Relative(relative_mosaic) => relative_mosaic.get_bounding_circle(),
        }
    }

    #[must_use]
    pub fn get_center_of_mass(&self) -> CoordinatedPoint {
        match self {
            AnonymizedMosaic::Absolute(mosaic) => mosaic.get_center_of_mass(),
            AnonymizedMosaic::Relative(relative_mosaic) => relative_mosaic.get_center_of_mass(),
        }
    }

    #[must_use]
    pub fn get_area(&self) -> f64 {
        match self {
            AnonymizedMosaic::Absolute(mosaic) => mosaic.get_area(),
            AnonymizedMosaic::Relative(relative_mosaic) => relative_mosaic.get_area(),
        }
    }

    #[must_use]
    pub fn get_average_color(&self) -> Vec3d {
        match self {
            AnonymizedMosaic::Absolute(mosaic) => mosaic.get_average_color(),
            AnonymizedMosaic::Relative(relative_mosaic) => relative_mosaic.get_average_color(),
        }
    }
}

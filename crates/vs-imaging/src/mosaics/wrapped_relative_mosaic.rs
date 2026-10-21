#![warn(clippy::pedantic)]
#![allow(
    clippy::missing_errors_doc,
    clippy::missing_panics_doc,
    clippy::too_many_lines,
    clippy::unused_async_trait_impl
)]

use crate::math::{CoordinatedCircle, CoordinatedPoint, CoordinatedRectangle, Rectangle};
use rs_math3d::Vec3d;
use std::sync::{Arc, Mutex};

use super::{RelativeMosaic, WrappedMosaic};

#[derive(Clone)]
pub struct WrappedRelativeMosaic {
    relative_mosaic: Arc<Mutex<RelativeMosaic>>,
}

impl WrappedRelativeMosaic {
    #[must_use]
    pub fn new(mosaic: WrappedMosaic, absolute_rectangle: Rectangle) -> Self {
        Self {
            relative_mosaic: Arc::new(Mutex::new(RelativeMosaic::new(mosaic, absolute_rectangle))),
        }
    }

    #[must_use]
    pub fn get_bounding_box(&self) -> CoordinatedRectangle {
        let mut relative_mosaic = self.relative_mosaic.lock().unwrap();
        relative_mosaic.get_bounding_box()
    }

    #[must_use]
    pub fn get_bounding_circle(&self) -> CoordinatedCircle {
        let mut relative_mosaic = self.relative_mosaic.lock().unwrap();
        relative_mosaic.get_bounding_circle()
    }

    #[must_use]
    pub fn get_center_of_mass(&self) -> CoordinatedPoint {
        let mut relative_mosaic = self.relative_mosaic.lock().unwrap();
        relative_mosaic.get_center_of_mass()
    }

    #[must_use]
    pub fn get_area(&self) -> f64 {
        let mut relative_mosaic = self.relative_mosaic.lock().unwrap();
        relative_mosaic.get_area()
    }

    #[must_use]
    pub fn get_average_color(&self) -> Vec3d {
        let mut relative_mosaic = self.relative_mosaic.lock().unwrap();
        relative_mosaic.get_average_color()
    }

    #[must_use]
    pub fn get_absolute_rectangle(&self) -> Rectangle {
        let relative_mosaic = self.relative_mosaic.lock().unwrap();
        relative_mosaic.get_absolute_rectangle()
    }

    #[must_use]
    pub fn get_mosaic(&self) -> WrappedMosaic {
        let relative_mosaic = self.relative_mosaic.lock().unwrap();
        relative_mosaic.get_mosaic()
    }

    #[must_use]
    pub fn shares_identity_with(&self, other: &WrappedRelativeMosaic) -> bool {
        Arc::ptr_eq(&self.relative_mosaic, &other.relative_mosaic)
    }
}

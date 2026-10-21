#![warn(clippy::pedantic)]
#![allow(
    clippy::missing_errors_doc,
    clippy::missing_panics_doc,
    clippy::too_many_lines,
    clippy::unused_async_trait_impl
)]

use crate::math::CoordinatedPoint;
use rs_math3d::Vec3d;

use super::{Slice, i32_from_usize};

#[derive(Clone, PartialEq)]
pub struct AnnotatedSlice {
    slice: Slice,
    line_number: usize,
}

impl AnnotatedSlice {
    pub(super) fn set_line_number(&mut self, line_number: usize) {
        self.line_number = line_number;
    }

    pub(super) fn set_slice(&mut self, slice: Slice) {
        self.slice = slice;
    }

    pub(super) fn slice(&self) -> &Slice {
        &self.slice
    }

    pub(super) fn line_number(&self) -> usize {
        self.line_number
    }
}

impl AnnotatedSlice {
    #[must_use]
    pub fn new(slice: Slice, line_number: usize) -> Self {
        Self { slice, line_number }
    }

    #[must_use]
    pub fn touches(&self, other: &AnnotatedSlice) -> bool {
        // Check if the slices are on adjacent lines
        if (i32_from_usize(self.line_number) - i32_from_usize(other.line_number)).abs() != 1 {
            return false;
        }
        // Check if the slices overlap in the x-axis
        let self_start_x = self.slice.get_start().get_x();
        let self_end_x = self.slice.get_end().get_x();
        let other_start_x = other.slice.get_start().get_x();
        let other_end_x = other.slice.get_end().get_x();

        !(self_end_x < other_start_x || self_start_x > other_end_x)
    }

    #[must_use]
    pub fn get_mass(&self) -> f64 {
        self.slice.get_end().get_x() - self.slice.get_start().get_x() + 1.0
    }

    #[must_use]
    pub fn get_midpoint(&self) -> Vec3d {
        Vec3d::new(
            f64::midpoint(self.slice.get_start().get_x(), self.slice.get_end().get_x()),
            self.slice.get_start().get_y(),
            0.0,
        )
    }

    #[must_use]
    pub fn get_slice(&self) -> Slice {
        self.slice.clone()
    }

    #[must_use]
    pub fn get_start(&self) -> &CoordinatedPoint {
        self.slice.get_start()
    }

    #[must_use]
    pub fn get_end(&self) -> &CoordinatedPoint {
        self.slice.get_end()
    }
}

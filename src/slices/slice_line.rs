#![warn(clippy::pedantic)]
#![allow(
    clippy::missing_errors_doc,
    clippy::missing_panics_doc,
    clippy::too_many_lines,
    clippy::unused_async_trait_impl
)]

use crate::math::CoordinatedPoint;
use rs_math3d::Vec3d;

use super::{AnnotatedSlice, Slice};

#[derive(Clone)]
pub struct SliceLine {
    line_number: usize,
    slices: Vec<AnnotatedSlice>,
}

impl SliceLine {
    pub(super) fn slices_mut(&mut self) -> &mut Vec<AnnotatedSlice> {
        &mut self.slices
    }

    pub(super) fn line_number(&self) -> usize {
        self.line_number
    }

    pub(super) fn slices(&self) -> &Vec<AnnotatedSlice> {
        &self.slices
    }
}

impl SliceLine {
    #[must_use]
    pub fn new(line_number: usize, slices: Vec<AnnotatedSlice>) -> Self {
        Self {
            line_number,
            slices,
        }
    }

    pub fn maintain_slices(&mut self) {
        // remove duplicates of the same slice
        self.slices.sort_by(|a, b| {
            let a_start_x = a.slice().get_start().get_x();
            let a_end_x = a.slice().get_end().get_x();
            let b_start_x = b.slice().get_start().get_x();
            let b_end_x = b.slice().get_end().get_x();
            a_start_x
                .partial_cmp(&b_start_x)
                .unwrap()
                .then(a_end_x.partial_cmp(&b_end_x).unwrap())
        });
        self.slices.dedup();
    }

    pub(super) fn merge_overlapping_slices(&mut self) {
        if self.slices.len() < 2 {
            return;
        }

        let mut merged = vec![self.slices[0].clone()];
        for slice in self.slices.iter().skip(1) {
            let previous = merged.last_mut().unwrap();
            let previous_end = previous.slice().get_end().get_x();
            let current_start = slice.slice().get_start().get_x();
            if previous_end >= current_start {
                let new_end = previous_end.max(slice.slice().get_end().get_x());
                *previous = AnnotatedSlice::new(
                    Slice::new(
                        previous.slice().get_start(),
                        &CoordinatedPoint::new(
                            previous.slice().get_end().get_coordinate_system().clone(),
                            Vec3d::new(new_end, previous.slice().get_end().get_y(), 0.0),
                        ),
                    ),
                    previous.line_number(),
                );
            } else {
                merged.push(slice.clone());
            }
        }
        self.slices = merged;
    }

    pub(super) fn expand_by(&mut self, pixels: f64) {
        for slice in &mut self.slices {
            let new_start = slice
                .slice()
                .get_start()
                .plus(Vec3d::new(-pixels, 0.0, 0.0));
            let new_end = slice.slice().get_end().plus(Vec3d::new(pixels, 0.0, 0.0));
            *slice = AnnotatedSlice::new(Slice::new(&new_start, &new_end), slice.line_number());
        }
    }

    pub fn add_unchecked(&mut self, slice: AnnotatedSlice) {
        assert!(
            slice.line_number() == self.line_number,
            "Slice line number does not match SliceLine's line number"
        );
        self.slices.push(slice);
    }

    pub fn add(&mut self, slice: AnnotatedSlice) {
        assert!(
            slice.line_number() == self.line_number,
            "Slice line number does not match SliceLine's line number"
        );
        self.slices.push(slice);
        self.maintain_slices();
    }

    pub fn add_slices(&mut self, slice_line: &SliceLine) {
        assert!(
            slice_line.line_number == self.line_number,
            "Slice line number does not match SliceLine's line number"
        );
        self.slices.extend(slice_line.slices.clone());
        self.maintain_slices();
    }

    #[must_use]
    pub fn get_line_number(&self) -> usize {
        self.line_number
    }

    #[must_use]
    pub fn get_slices(&self) -> &[AnnotatedSlice] {
        &self.slices
    }
}

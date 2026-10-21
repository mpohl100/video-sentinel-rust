#![warn(clippy::pedantic)]
#![allow(
    clippy::missing_errors_doc,
    clippy::missing_panics_doc,
    clippy::too_many_lines,
    clippy::unused_async_trait_impl
)]

use super::PreliminarySlice;

pub(super) struct PreliminaryAnnotatedSlice {
    slice: PreliminarySlice,
    line_number: usize,
}

impl PreliminaryAnnotatedSlice {
    pub(super) fn slice(&self) -> &PreliminarySlice {
        &self.slice
    }

    pub(super) fn line_number(&self) -> usize {
        self.line_number
    }
}

impl PreliminaryAnnotatedSlice {
    pub fn new(slice: PreliminarySlice, line_number: usize) -> Self {
        Self { slice, line_number }
    }

    pub fn get_slice(&self) -> PreliminarySlice {
        self.slice.clone()
    }
}

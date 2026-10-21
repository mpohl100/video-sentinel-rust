#![warn(clippy::pedantic)]
#![allow(
    clippy::missing_errors_doc,
    clippy::missing_panics_doc,
    clippy::too_many_lines,
    clippy::unused_async_trait_impl
)]

#[derive(Clone, Copy, PartialEq, PartialOrd)]
pub struct TraceParams {
    num_skeleton: usize,
    close_slice_threshold: f64,
}

impl TraceParams {
    #[must_use]
    pub fn new(num_skeleton: usize, close_slice_threshold: f64) -> Self {
        TraceParams {
            num_skeleton,
            close_slice_threshold,
        }
    }

    #[must_use]
    pub fn num_skeleton(&self) -> usize {
        self.num_skeleton
    }

    #[must_use]
    pub fn close_slice_threshold(&self) -> f64 {
        self.close_slice_threshold
    }
}

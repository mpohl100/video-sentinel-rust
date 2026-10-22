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
}

impl TraceParams {
    #[must_use]
    pub fn new(num_skeleton: usize) -> Self {
        TraceParams { num_skeleton }
    }

    #[must_use]
    pub fn num_skeleton(&self) -> usize {
        self.num_skeleton
    }
}

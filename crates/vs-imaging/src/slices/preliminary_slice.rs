#![warn(clippy::pedantic)]
#![allow(
    clippy::missing_errors_doc,
    clippy::missing_panics_doc,
    clippy::too_many_lines,
    clippy::unused_async_trait_impl
)]

use crate::math::CoordinatedPoint;

#[derive(Clone)]
pub(super) struct PreliminarySlice {
    start: CoordinatedPoint,
    end: CoordinatedPoint,
}

impl PreliminarySlice {
    pub fn new(start: CoordinatedPoint, end: CoordinatedPoint) -> Self {
        Self { start, end }
    }

    pub fn get_start(&self) -> CoordinatedPoint {
        self.start.clone()
    }

    pub fn get_end(&self) -> CoordinatedPoint {
        self.end.clone()
    }
}

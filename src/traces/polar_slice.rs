#![warn(clippy::pedantic)]
#![allow(
    clippy::missing_errors_doc,
    clippy::missing_panics_doc,
    clippy::too_many_lines,
    clippy::unused_async_trait_impl
)]

use crate::math::PolarCoordinates;

#[derive(Clone)]
pub(super) struct PolarSlice {
    start: PolarCoordinates,
    end: PolarCoordinates,
}

impl PolarSlice {
    pub(super) fn new(start: PolarCoordinates, end: PolarCoordinates) -> Self {
        if start.get_radius() < end.get_radius() {
            PolarSlice { start, end }
        } else {
            PolarSlice {
                start: end,
                end: start,
            }
        }
    }

    pub(super) fn get_start(&self) -> &PolarCoordinates {
        &self.start
    }

    pub(super) fn get_end(&self) -> &PolarCoordinates {
        &self.end
    }
}

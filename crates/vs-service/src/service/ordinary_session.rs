#![warn(clippy::pedantic)]
#![allow(
    clippy::missing_errors_doc,
    clippy::missing_panics_doc,
    clippy::too_many_lines,
    clippy::unused_async_trait_impl
)]

use super::get_rectangles_request::SessionCalculation;
use crate::mosaics::Results;
use crate::slices::BasicParams;
use std::sync::Arc;

use super::RgbColor;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ColorFilter {
    pub color: RgbColor,
    pub threshold_for_color: RgbColor,
}

impl ColorFilter {
    pub(super) fn matches(self, average: RgbColor) -> bool {
        average.red.abs_diff(self.color.red) <= self.threshold_for_color.red
            && average.green.abs_diff(self.color.green) <= self.threshold_for_color.green
            && average.blue.abs_diff(self.color.blue) <= self.threshold_for_color.blue
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum AddColorFilterResult {
    Success,
    SessionNotFound,
    SessionTypeDoesNotSupportColorFilters,
}

#[derive(Clone)]
pub struct OrdinarySession {
    pub basic_params: BasicParams,
    pub results: Results,
    pub color_filters: Vec<ColorFilter>,
    pub(super) calculation: Arc<SessionCalculation>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn color_filter_checks_each_channel_inclusively_without_overflow() {
        let filter = ColorFilter {
            color: RgbColor {
                red: 5,
                green: 100,
                blue: 250,
            },
            threshold_for_color: RgbColor {
                red: 5,
                green: 10,
                blue: 5,
            },
        };
        assert!(filter.matches(RgbColor {
            red: 0,
            green: 110,
            blue: 255
        }));
        assert!(filter.matches(RgbColor {
            red: 10,
            green: 90,
            blue: 245
        }));
        assert!(!filter.matches(RgbColor {
            red: 11,
            green: 100,
            blue: 250
        }));
        assert!(!filter.matches(RgbColor {
            red: 5,
            green: 111,
            blue: 250
        }));
        assert!(!filter.matches(RgbColor {
            red: 5,
            green: 100,
            blue: 244
        }));
    }
}

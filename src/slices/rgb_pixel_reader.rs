#![warn(clippy::pedantic)]
#![allow(
    clippy::missing_errors_doc,
    clippy::missing_panics_doc,
    clippy::too_many_lines,
    clippy::unused_async_trait_impl
)]

use opencv::core::Mat;
use opencv::prelude::*;
use std::sync::MutexGuard;

use super::WrappedRgbImage;

pub(super) struct RgbPixelReader<'a> {
    _guard: MutexGuard<'a, Mat>,
    data: *const u8,
    row_stride: usize,
}

impl<'a> RgbPixelReader<'a> {
    pub(super) fn from_guard(guard: MutexGuard<'a, Mat>) -> Result<Self, opencv::Error> {
        let data = guard.data();
        let row_stride = guard.step1(0)?;
        Ok(Self {
            _guard: guard,
            data,
            row_stride,
        })
    }

    pub(super) fn new(image: &'a WrappedRgbImage) -> Result<Self, opencv::Error> {
        Self::from_guard(image.get_mat())
    }

    #[inline]
    pub(super) unsafe fn channel_unchecked(&self, x: usize, y: usize, channel: usize) -> f64 {
        unsafe { f64::from(*self.data.add(y * self.row_stride + x * 3 + channel)) }
    }

    #[inline]
    pub(super) unsafe fn pixel_unchecked(&self, x: usize, y: usize) -> [u8; 3] {
        let offset = y * self.row_stride + x * 3;
        unsafe {
            [
                *self.data.add(offset),
                *self.data.add(offset + 1),
                *self.data.add(offset + 2),
            ]
        }
    }
}

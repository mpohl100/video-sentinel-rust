#![warn(clippy::pedantic)]
#![allow(
    clippy::missing_errors_doc,
    clippy::missing_panics_doc,
    clippy::too_many_lines,
    clippy::unused_async_trait_impl
)]

use opencv::core::Mat;
use opencv::prelude::*;
use std::marker::PhantomData;

pub(super) struct GrayPixelReader<'a> {
    data: *const u8,
    row_stride: usize,
    _mat: PhantomData<&'a Mat>,
}

impl<'a> GrayPixelReader<'a> {
    pub(super) fn new(image: &'a Mat) -> Result<Self, opencv::Error> {
        Ok(Self {
            data: image.data(),
            row_stride: image.step1(0)?,
            _mat: PhantomData,
        })
    }

    #[inline]
    pub(super) unsafe fn value_unchecked(&self, x: usize, y: usize) -> f64 {
        unsafe { f64::from(*self.data.add(y * self.row_stride + x)) }
    }
}

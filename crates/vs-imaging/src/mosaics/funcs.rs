#![warn(clippy::pedantic)]
#![allow(
    clippy::missing_errors_doc,
    clippy::missing_panics_doc,
    clippy::too_many_lines,
    clippy::unused_async_trait_impl
)]

use crate::slices::SliceMatrix;

use super::{WrappedMosaic, WrappedRelativeMosaic};

pub fn deduce_mosaics(
    slice_matrices: Vec<SliceMatrix>,
    surrounding_rectangle: &crate::math::Rectangle,
) -> Vec<WrappedRelativeMosaic> {
    slice_matrices
        .into_iter()
        .map(WrappedMosaic::new)
        .map(|mosaic| WrappedRelativeMosaic::new(mosaic, surrounding_rectangle.clone()))
        .collect()
}

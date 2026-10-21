#![warn(clippy::pedantic)]
#![allow(
    clippy::missing_errors_doc,
    clippy::missing_panics_doc,
    clippy::too_many_lines,
    clippy::unused_async_trait_impl
)]

use crate::traces::TraceParams;

use super::TileParams;

#[derive(Clone, Copy, PartialEq)]
pub struct EyeParams {
    pub tile_params: TileParams,
    pub bucket_delta: f64,
    pub trace_params: TraceParams,
    pub target_similarity: f64,
}

impl EyeParams {
    #[must_use]
    pub fn new(
        tile_params: TileParams,
        bucket_delta: f64,
        trace_params: TraceParams,
        target_similarity: f64,
    ) -> Self {
        EyeParams {
            tile_params,
            bucket_delta,
            trace_params,
            target_similarity,
        }
    }
}

#![warn(clippy::pedantic)]
#![allow(
    clippy::missing_errors_doc,
    clippy::missing_panics_doc,
    clippy::too_many_lines,
    clippy::unused_async_trait_impl
)]

use crate::eye::TileParams;
use crate::traces::TraceParams;

#[derive(Clone, Copy)]
pub struct ObjectDetectionParams {
    pub tile_params: TileParams,
    pub bucket_delta: f64,
    pub trace_params: TraceParams,
    pub target_similarity: f64,
}

impl ObjectDetectionParams {
    #[must_use]
    pub fn new(
        tile_params: TileParams,
        bucket_delta: f64,
        trace_params: TraceParams,
        target_similarity: f64,
    ) -> Self {
        ObjectDetectionParams {
            tile_params,
            bucket_delta,
            trace_params,
            target_similarity,
        }
    }
}

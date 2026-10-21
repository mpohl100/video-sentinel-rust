#![warn(clippy::pedantic)]
#![allow(
    clippy::missing_errors_doc,
    clippy::missing_panics_doc,
    clippy::too_many_lines,
    clippy::unused_async_trait_impl
)]

use super::{TileParamsInput, TraceParamsInput};

#[derive(Clone, Copy)]
pub struct EyeParamsInput {
    pub tile_params: TileParamsInput,
    pub bucket_delta: f64,
    pub trace_params: TraceParamsInput,
    pub target_similarity: f64,
}

#![warn(clippy::pedantic)]
#![allow(
    clippy::missing_errors_doc,
    clippy::missing_panics_doc,
    clippy::too_many_lines,
    clippy::unused_async_trait_impl
)]

use super::{TileParamsGql, TraceParamsGql};

#[derive(async_graphql::SimpleObject, Clone)]
pub struct EyeParamsGql {
    tile_params: TileParamsGql,
    bucket_delta: f64,
    trace_params: TraceParamsGql,
    target_similarity: f64,
}

impl From<crate::eye::EyeParams> for EyeParamsGql {
    fn from(value: crate::eye::EyeParams) -> Self {
        Self {
            tile_params: value.tile_params.into(),
            bucket_delta: value.bucket_delta,
            trace_params: value.trace_params.into(),
            target_similarity: value.target_similarity,
        }
    }
}

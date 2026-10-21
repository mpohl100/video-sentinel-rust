#![warn(clippy::pedantic)]
#![allow(
    clippy::missing_errors_doc,
    clippy::missing_panics_doc,
    clippy::too_many_lines,
    clippy::unused_async_trait_impl
)]

use crate::service::EyeParamsInput;
use async_graphql::InputObject;

use super::{TileParamsInputGql, TraceParamsInputGql};

#[derive(InputObject, Clone)]
pub struct EyeParamsInputGql {
    tile_params: TileParamsInputGql,
    bucket_delta: f64,
    trace_params: TraceParamsInputGql,
    target_similarity: f64,
}

impl From<EyeParamsInputGql> for EyeParamsInput {
    fn from(value: EyeParamsInputGql) -> Self {
        Self {
            tile_params: value.tile_params.into(),
            bucket_delta: value.bucket_delta,
            trace_params: value.trace_params.into(),
            target_similarity: value.target_similarity,
        }
    }
}

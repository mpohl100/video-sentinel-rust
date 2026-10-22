#![warn(clippy::pedantic)]
#![allow(
    clippy::missing_errors_doc,
    clippy::missing_panics_doc,
    clippy::too_many_lines,
    clippy::unused_async_trait_impl
)]

use crate::service::TraceParamsInput;
use async_graphql::InputObject;

#[derive(InputObject, Clone)]
pub struct TraceParamsInputGql {
    num_skeleton: usize,
}

impl From<TraceParamsInputGql> for TraceParamsInput {
    fn from(value: TraceParamsInputGql) -> Self {
        Self {
            num_skeleton: value.num_skeleton,
        }
    }
}

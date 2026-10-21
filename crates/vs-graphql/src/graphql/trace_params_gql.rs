#![warn(clippy::pedantic)]
#![allow(
    clippy::missing_errors_doc,
    clippy::missing_panics_doc,
    clippy::too_many_lines,
    clippy::unused_async_trait_impl
)]

#[derive(async_graphql::SimpleObject, Clone)]
pub struct TraceParamsGql {
    num_skeleton: usize,
    close_slice_threshold: f64,
}

impl From<crate::traces::TraceParams> for TraceParamsGql {
    fn from(value: crate::traces::TraceParams) -> Self {
        Self {
            num_skeleton: value.num_skeleton(),
            close_slice_threshold: value.close_slice_threshold(),
        }
    }
}

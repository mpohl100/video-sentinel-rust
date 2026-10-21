#![warn(clippy::pedantic)]
#![allow(
    clippy::missing_errors_doc,
    clippy::missing_panics_doc,
    clippy::too_many_lines,
    clippy::unused_async_trait_impl
)]

use crate::service::SliceLine;

use super::SliceGql;

#[derive(async_graphql::SimpleObject, Clone)]
pub struct SliceLineGql {
    slices: Vec<SliceGql>,
    line_number: usize,
}

impl From<SliceLine> for SliceLineGql {
    fn from(value: SliceLine) -> Self {
        Self {
            slices: value.slices.into_iter().map(Into::into).collect(),
            line_number: value.line_number,
        }
    }
}

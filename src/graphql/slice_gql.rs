#![warn(clippy::pedantic)]
#![allow(
    clippy::missing_errors_doc,
    clippy::missing_panics_doc,
    clippy::too_many_lines,
    clippy::unused_async_trait_impl
)]

use super::PointGql;

#[derive(async_graphql::SimpleObject, Clone)]
pub struct SliceGql {
    start: PointGql,
    end: PointGql,
}

impl From<crate::service::Slice> for SliceGql {
    fn from(value: crate::service::Slice) -> Self {
        Self {
            start: value.start.into(),
            end: value.end.into(),
        }
    }
}

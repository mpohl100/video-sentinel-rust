#![warn(clippy::pedantic)]
#![allow(
    clippy::missing_errors_doc,
    clippy::missing_panics_doc,
    clippy::too_many_lines,
    clippy::unused_async_trait_impl
)]

use super::{BasicParamsGql, ResultsGql};

#[derive(async_graphql::SimpleObject, Clone)]
pub struct OrdinarySessionGql {
    basic_params: BasicParamsGql,
    results: ResultsGql,
}

impl From<crate::service::OrdinarySession> for OrdinarySessionGql {
    fn from(value: crate::service::OrdinarySession) -> Self {
        Self {
            basic_params: value.basic_params.into(),
            results: value.results.into(),
        }
    }
}

#![warn(clippy::pedantic)]
#![allow(
    clippy::missing_errors_doc,
    clippy::missing_panics_doc,
    clippy::too_many_lines,
    clippy::unused_async_trait_impl
)]

use super::{BasicParamsGql, EyeParamsGql, ResultsGql};

#[derive(async_graphql::SimpleObject, Clone)]
pub struct EyeSessionGql {
    basic_params: BasicParamsGql,
    eye_params: EyeParamsGql,
    results: ResultsGql,
}

impl From<crate::service::EyeSession> for EyeSessionGql {
    fn from(value: crate::service::EyeSession) -> Self {
        Self {
            basic_params: value.basic_params.into(),
            eye_params: value.eye_params.into(),
            results: value.results.into(),
        }
    }
}

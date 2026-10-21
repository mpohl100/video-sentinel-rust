#![warn(clippy::pedantic)]
#![allow(
    clippy::missing_errors_doc,
    clippy::missing_panics_doc,
    clippy::too_many_lines,
    clippy::unused_async_trait_impl
)]

use crate::mosaics::Results;
use async_graphql::Enum;

#[derive(Enum, Copy, Clone, Eq, PartialEq)]
pub enum ResultsGql {
    Absolute,
    Relative,
}

impl From<Results> for ResultsGql {
    fn from(value: Results) -> Self {
        match value {
            Results::Absolute => Self::Absolute,
            Results::Relative => Self::Relative,
        }
    }
}

impl From<ResultsGql> for Results {
    fn from(value: ResultsGql) -> Self {
        match value {
            ResultsGql::Absolute => Self::Absolute,
            ResultsGql::Relative => Self::Relative,
        }
    }
}

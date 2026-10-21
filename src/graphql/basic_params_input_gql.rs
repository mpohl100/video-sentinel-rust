#![warn(clippy::pedantic)]
#![allow(
    clippy::missing_errors_doc,
    clippy::missing_panics_doc,
    clippy::too_many_lines,
    clippy::unused_async_trait_impl
)]

use crate::service::BasicParamsInput;
use async_graphql::InputObject;

#[derive(InputObject, Clone)]
pub struct BasicParamsInputGql {
    do_grayscale: bool,
    gradient_threshold: u8,
}

impl From<BasicParamsInputGql> for BasicParamsInput {
    fn from(value: BasicParamsInputGql) -> Self {
        Self {
            do_grayscale: value.do_grayscale,
            gradient_threshold: value.gradient_threshold,
        }
    }
}

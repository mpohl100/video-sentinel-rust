#![warn(clippy::pedantic)]
#![allow(
    clippy::missing_errors_doc,
    clippy::missing_panics_doc,
    clippy::too_many_lines,
    clippy::unused_async_trait_impl
)]

#[derive(async_graphql::SimpleObject, Clone)]
pub struct BasicParamsGql {
    do_grayscale: bool,
    gradient_threshold: u8,
}

impl From<crate::slices::BasicParams> for BasicParamsGql {
    fn from(value: crate::slices::BasicParams) -> Self {
        Self {
            do_grayscale: value.do_grayscale(),
            gradient_threshold: value.gradient_threshold(),
        }
    }
}

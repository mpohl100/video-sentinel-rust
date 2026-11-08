#![warn(clippy::pedantic)]
#![allow(
    clippy::missing_errors_doc,
    clippy::missing_panics_doc,
    clippy::too_many_lines,
    clippy::unused_async_trait_impl
)]

#[derive(Clone, Debug, PartialEq)]
pub enum CreateThreeDTriangleSessionResult {
    Success,
    SessionAlreadyExists,
    InvalidInput(String),
}

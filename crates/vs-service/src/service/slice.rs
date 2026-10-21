#![warn(clippy::pedantic)]
#![allow(
    clippy::missing_errors_doc,
    clippy::missing_panics_doc,
    clippy::too_many_lines,
    clippy::unused_async_trait_impl
)]

use super::Point;

#[derive(Clone, Copy)]
pub struct Slice {
    pub start: Point,
    pub end: Point,
}

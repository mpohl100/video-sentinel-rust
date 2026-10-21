#![warn(clippy::pedantic)]
#![allow(
    clippy::missing_errors_doc,
    clippy::missing_panics_doc,
    clippy::too_many_lines,
    clippy::unused_async_trait_impl
)]

use super::Slice;

#[derive(Clone)]
pub struct SliceLine {
    pub slices: Vec<Slice>,
    pub line_number: usize,
}

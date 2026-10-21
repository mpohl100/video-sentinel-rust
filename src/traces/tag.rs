#![warn(clippy::pedantic)]
#![allow(
    clippy::missing_errors_doc,
    clippy::missing_panics_doc,
    clippy::too_many_lines,
    clippy::unused_async_trait_impl
)]

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(super) enum Tag {
    Empty = 0,
    Filled = 1,
}

#![warn(clippy::pedantic)]
#![allow(
    clippy::missing_errors_doc,
    clippy::missing_panics_doc,
    clippy::too_many_lines,
    clippy::unused_async_trait_impl
)]

use num_traits::ToPrimitive;

pub(super) fn i64_from_f64(value: f64) -> i64 {
    value.to_i64().expect("bucket key should fit within i64")
}

pub(super) fn usize_from_f64(value: f64) -> usize {
    value
        .to_usize()
        .expect("tile step count should fit within usize")
}

pub(super) fn f64_from_usize(value: usize) -> f64 {
    value.to_f64().expect("tile index should convert to f64")
}

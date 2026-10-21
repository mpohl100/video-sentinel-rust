#![warn(clippy::pedantic)]
#![allow(
    clippy::missing_errors_doc,
    clippy::missing_panics_doc,
    clippy::too_many_lines,
    clippy::unused_async_trait_impl
)]

#[derive(Clone)]
pub(super) struct Ratio {
    from: f64,
    to: f64,
}

impl Ratio {
    pub(super) fn new(from: f64, to: f64) -> Self {
        Self { from, to }
    }

    pub(super) fn from(&self) -> f64 {
        self.from
    }

    pub(super) fn to(&self) -> f64 {
        self.to
    }
}

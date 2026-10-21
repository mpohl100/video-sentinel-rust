#![warn(clippy::pedantic)]
#![allow(
    clippy::missing_errors_doc,
    clippy::missing_panics_doc,
    clippy::too_many_lines,
    clippy::unused_async_trait_impl
)]

#[derive(Clone, Copy)]
pub struct BasicParams {
    do_grayscale: bool,
    gradient_threshold: u8,
}

impl BasicParams {
    #[must_use]
    pub fn new(do_grayscale: bool, gradient_threshold: u8) -> Self {
        Self {
            do_grayscale,
            gradient_threshold,
        }
    }

    #[must_use]
    pub fn do_grayscale(&self) -> bool {
        self.do_grayscale
    }

    #[must_use]
    pub fn gradient_threshold(&self) -> u8 {
        self.gradient_threshold
    }
}

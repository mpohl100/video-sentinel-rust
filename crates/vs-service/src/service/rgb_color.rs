#![warn(clippy::pedantic)]
#![allow(
    clippy::missing_errors_doc,
    clippy::missing_panics_doc,
    clippy::too_many_lines,
    clippy::unused_async_trait_impl
)]

#[derive(Clone, Copy)]
pub struct RgbColor {
    pub red: u8,
    pub green: u8,
    pub blue: u8,
}

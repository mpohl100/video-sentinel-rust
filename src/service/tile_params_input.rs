#![warn(clippy::pedantic)]
#![allow(
    clippy::missing_errors_doc,
    clippy::missing_panics_doc,
    clippy::too_many_lines,
    clippy::unused_async_trait_impl
)]

#[derive(Clone, Copy)]
pub struct TileParamsInput {
    pub tile_x: f64,
    pub tile_y: f64,
}

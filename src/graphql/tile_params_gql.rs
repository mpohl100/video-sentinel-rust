#![warn(clippy::pedantic)]
#![allow(
    clippy::missing_errors_doc,
    clippy::missing_panics_doc,
    clippy::too_many_lines,
    clippy::unused_async_trait_impl
)]

#[derive(async_graphql::SimpleObject, Clone)]
pub struct TileParamsGql {
    tile_x: f64,
    tile_y: f64,
}

impl From<crate::eye::TileParams> for TileParamsGql {
    fn from(value: crate::eye::TileParams) -> Self {
        Self {
            tile_x: value.relative_tile_x(),
            tile_y: value.relative_tile_y(),
        }
    }
}

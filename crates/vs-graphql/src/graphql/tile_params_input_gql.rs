#![warn(clippy::pedantic)]
#![allow(
    clippy::missing_errors_doc,
    clippy::missing_panics_doc,
    clippy::too_many_lines,
    clippy::unused_async_trait_impl
)]

use crate::service::TileParamsInput;
use async_graphql::InputObject;

#[derive(InputObject, Clone)]
pub struct TileParamsInputGql {
    tile_x: f64,
    tile_y: f64,
}

impl From<TileParamsInputGql> for TileParamsInput {
    fn from(value: TileParamsInputGql) -> Self {
        Self {
            tile_x: value.tile_x,
            tile_y: value.tile_y,
        }
    }
}

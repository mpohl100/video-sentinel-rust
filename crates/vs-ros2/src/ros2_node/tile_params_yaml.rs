#![warn(clippy::pedantic)]
#![allow(
    clippy::missing_errors_doc,
    clippy::missing_panics_doc,
    clippy::too_many_lines,
    clippy::unused_async_trait_impl
)]

use crate::service::TileParamsInput;
use serde::Deserialize;

use super::default_tile_fraction;

#[derive(Debug, Clone, Copy, Deserialize)]
pub(super) struct TileParamsYaml {
    #[serde(default = "default_tile_fraction")]
    tile_x: f64,
    #[serde(default = "default_tile_fraction")]
    tile_y: f64,
}

impl Default for TileParamsYaml {
    fn default() -> Self {
        Self {
            tile_x: default_tile_fraction(),
            tile_y: default_tile_fraction(),
        }
    }
}

impl TileParamsYaml {
    pub(super) fn into_input(self) -> TileParamsInput {
        TileParamsInput {
            tile_x: self.tile_x,
            tile_y: self.tile_y,
        }
    }
}

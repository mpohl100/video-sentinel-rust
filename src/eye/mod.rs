mod eye_params;
mod funcs;
mod tile_params;

pub use eye_params::EyeParams;
#[cfg(test)]
use funcs::f64_from_usize;
pub use funcs::{deduce_bucketed_mosaics, deduce_rectangles};
pub use tile_params::TileParams;

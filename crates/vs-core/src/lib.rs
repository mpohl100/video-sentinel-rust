//! Core facade: re-exports layers 0-3 under their original module names.

pub use vs_detection::{bucketed_mosaics, eye, object_detection};
pub use vs_imaging::{mosaics, slices};
pub use vs_math::math;
pub use vs_tracing::{traced_mosaics, traces};

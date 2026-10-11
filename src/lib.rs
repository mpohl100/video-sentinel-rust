#![warn(clippy::pedantic)]
#![allow(
    clippy::missing_errors_doc,
    clippy::missing_panics_doc,
    clippy::too_many_lines,
    clippy::unused_async_trait_impl
)]

pub mod bucketed_mosaics;
pub mod eye;
pub mod graphql;
pub mod math;
pub mod mosaics;
pub mod object_detection;
pub mod service;
pub mod slices;
pub mod traced_mosaics;
pub mod traces;

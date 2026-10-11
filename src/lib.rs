#![warn(clippy::pedantic)]
#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    clippy::cast_precision_loss,
    clippy::cast_sign_loss,
    clippy::float_cmp,
    clippy::format_push_string,
    clippy::many_single_char_names,
    clippy::match_wildcard_for_single_variants,
    clippy::missing_errors_doc,
    clippy::missing_panics_doc,
    clippy::must_use_candidate,
    clippy::needless_pass_by_value,
    clippy::similar_names,
    clippy::too_many_lines,
    clippy::unreadable_literal,
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

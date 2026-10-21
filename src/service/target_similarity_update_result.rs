#![warn(clippy::pedantic)]
#![allow(
    clippy::missing_errors_doc,
    clippy::missing_panics_doc,
    clippy::too_many_lines,
    clippy::unused_async_trait_impl
)]

pub enum TargetSimilarityUpdateResult {
    Success,
    SessionNotFound,
    SessionTypeDoesNotSupportTargetSimilarity,
}

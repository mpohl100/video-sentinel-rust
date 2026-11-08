#![warn(clippy::pedantic)]
#![allow(
    clippy::missing_errors_doc,
    clippy::missing_panics_doc,
    clippy::too_many_lines,
    clippy::unused_async_trait_impl
)]

#[derive(async_graphql::Enum, Clone, Copy, Eq, PartialEq)]
pub enum DepthEncodingGql {
    #[graphql(name = "UINT16_MM")]
    Uint16Mm,
    #[graphql(name = "FLOAT32_M")]
    Float32M,
}

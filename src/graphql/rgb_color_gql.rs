#![warn(clippy::pedantic)]
#![allow(
    clippy::missing_errors_doc,
    clippy::missing_panics_doc,
    clippy::too_many_lines,
    clippy::unused_async_trait_impl
)]

#[derive(async_graphql::SimpleObject, Clone)]
pub struct RgbColorGql {
    red: u8,
    green: u8,
    blue: u8,
}

impl From<crate::service::RgbColor> for RgbColorGql {
    fn from(value: crate::service::RgbColor) -> Self {
        Self {
            red: value.red,
            green: value.green,
            blue: value.blue,
        }
    }
}

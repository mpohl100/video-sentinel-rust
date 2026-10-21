#![warn(clippy::pedantic)]
#![allow(
    clippy::missing_errors_doc,
    clippy::missing_panics_doc,
    clippy::too_many_lines,
    clippy::unused_async_trait_impl
)]

use crate::slices::Color;
use async_graphql::Enum;

#[derive(Enum, Copy, Clone, Eq, PartialEq)]
pub enum ColorGql {
    Red,
    Green,
    Blue,
}

impl From<Color> for ColorGql {
    fn from(value: Color) -> Self {
        match value {
            Color::Red => Self::Red,
            Color::Green => Self::Green,
            Color::Blue => Self::Blue,
        }
    }
}

#![warn(clippy::pedantic)]
#![allow(
    clippy::missing_errors_doc,
    clippy::missing_panics_doc,
    clippy::too_many_lines,
    clippy::unused_async_trait_impl
)]

use crate::mosaics::AnonymizedMosaic;

use super::{Color, Rectangle};

#[derive(Clone)]
pub struct ColoredRectangle {
    rectangle: Rectangle,
    color: Color,
    mosaics: Vec<AnonymizedMosaic>,
}

impl ColoredRectangle {
    #[must_use]
    pub fn new(rectangle: Rectangle, color: Color, mosaics: Vec<AnonymizedMosaic>) -> Self {
        Self {
            rectangle,
            color,
            mosaics,
        }
    }

    #[must_use]
    pub fn get_rectangle(&self) -> Rectangle {
        self.rectangle
    }

    #[must_use]
    pub fn get_color(&self) -> Color {
        self.color.clone()
    }

    #[must_use]
    pub fn get_mosaics(&self) -> Vec<AnonymizedMosaic> {
        self.mosaics.clone()
    }
}

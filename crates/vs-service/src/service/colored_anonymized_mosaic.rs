#![warn(clippy::pedantic)]
#![allow(
    clippy::missing_errors_doc,
    clippy::missing_panics_doc,
    clippy::too_many_lines,
    clippy::unused_async_trait_impl
)]

use crate::mosaics::AnonymizedMosaic;
use crate::slices::Color;

/// Internal pairing of a color and a wrapped relative mosaic before enrichment.
pub(super) struct ColoredAnonymizedMosaic {
    color: Color,
    label: Option<String>,
    mosaic: AnonymizedMosaic,
}

impl ColoredAnonymizedMosaic {
    pub(super) fn new(color: Color, label: Option<String>, mosaic: AnonymizedMosaic) -> Self {
        Self {
            color,
            label,
            mosaic,
        }
    }

    pub(super) fn color(&self) -> &Color {
        &self.color
    }

    pub(super) fn label(&self) -> Option<&String> {
        self.label.as_ref()
    }

    pub(super) fn mosaic(&self) -> &AnonymizedMosaic {
        &self.mosaic
    }
}

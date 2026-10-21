#![warn(clippy::pedantic)]
#![allow(
    clippy::missing_errors_doc,
    clippy::missing_panics_doc,
    clippy::too_many_lines,
    clippy::unused_async_trait_impl
)]

use serde::Deserialize;
use std::path::PathBuf;

use super::RectangleYaml;

#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "source", rename_all = "snake_case")]
pub(super) enum ReferenceObjectYaml {
    ImagePath {
        object_id: String,
        image_path: PathBuf,
        surrounding_rectangle: RectangleYaml,
    },
    AsciiArt {
        object_id: String,
        ascii_art: String,
    },
}

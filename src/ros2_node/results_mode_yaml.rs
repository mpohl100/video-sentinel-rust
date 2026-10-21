#![warn(clippy::pedantic)]
#![allow(
    clippy::missing_errors_doc,
    clippy::missing_panics_doc,
    clippy::too_many_lines,
    clippy::unused_async_trait_impl
)]

use crate::mosaics::Results;
use serde::Deserialize;

#[derive(Debug, Clone, Copy, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub(super) enum ResultsModeYaml {
    #[default]
    Absolute,
    Relative,
}

impl ResultsModeYaml {
    pub(super) fn into_results(self) -> Results {
        match self {
            ResultsModeYaml::Absolute => Results::Absolute,
            ResultsModeYaml::Relative => Results::Relative,
        }
    }
}

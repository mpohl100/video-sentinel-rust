#![warn(clippy::pedantic)]
#![allow(
    clippy::missing_errors_doc,
    clippy::missing_panics_doc,
    clippy::too_many_lines,
    clippy::unused_async_trait_impl
)]

use crate::bucketed_mosaics::BucketedMosaics;
use crate::eye::EyeParams;
use crate::mosaics::Results;
use crate::slices::BasicParams;

#[derive(Clone)]
pub struct EyeSession {
    pub basic_params: BasicParams,
    pub eye_params: EyeParams,
    pub cached_previous_mosaics: Option<BucketedMosaics>,
    pub results: Results,
}

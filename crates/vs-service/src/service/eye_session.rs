#![warn(clippy::pedantic)]
#![allow(
    clippy::missing_errors_doc,
    clippy::missing_panics_doc,
    clippy::too_many_lines,
    clippy::unused_async_trait_impl
)]

use super::get_rectangles_request::SessionCalculation;
use crate::bucketed_mosaics::BucketedMosaics;
use crate::eye::EyeParams;
use crate::mosaics::Results;
use crate::slices::BasicParams;
use std::sync::{Arc, Mutex};

#[derive(Clone)]
pub struct EyeSession {
    pub basic_params: BasicParams,
    pub eye_params: EyeParams,
    pub cached_previous_mosaics: Option<BucketedMosaics>,
    pub results: Results,
    pub(super) cache_state: Arc<Mutex<(u64, Option<BucketedMosaics>)>>,
    pub(super) cache_generation: u64,
    pub(super) calculation: Arc<SessionCalculation>,
}

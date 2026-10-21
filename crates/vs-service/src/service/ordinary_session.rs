#![warn(clippy::pedantic)]
#![allow(
    clippy::missing_errors_doc,
    clippy::missing_panics_doc,
    clippy::too_many_lines,
    clippy::unused_async_trait_impl
)]

use crate::mosaics::Results;
use crate::slices::BasicParams;

#[derive(Clone)]
pub struct OrdinarySession {
    pub basic_params: BasicParams,
    pub results: Results,
}

#![warn(clippy::pedantic)]
#![allow(
    clippy::missing_errors_doc,
    clippy::missing_panics_doc,
    clippy::too_many_lines,
    clippy::unused_async_trait_impl
)]

use super::{EyeSession, ObjectSession, OrdinarySession};

#[derive(Clone)]
pub enum Session {
    Ordinary(OrdinarySession),
    Eye(EyeSession),
    Object(ObjectSession),
}

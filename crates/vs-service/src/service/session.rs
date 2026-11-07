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

impl Session {
    pub(super) fn calculation(
        &self,
    ) -> &std::sync::Arc<super::get_rectangles_request::SessionCalculation> {
        match self {
            Self::Ordinary(session) => &session.calculation,
            Self::Eye(session) => &session.calculation,
            Self::Object(session) => &session.calculation,
        }
    }
}

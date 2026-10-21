#![warn(clippy::pedantic)]
#![allow(
    clippy::missing_errors_doc,
    clippy::missing_panics_doc,
    clippy::too_many_lines,
    clippy::unused_async_trait_impl
)]

use crate::service::Service;
use std::sync::Arc;
use tokio::sync::Mutex;

#[derive(Clone)]
pub struct GraphqlAppState {
    pub service: Arc<Mutex<Service>>,
}

impl Default for GraphqlAppState {
    fn default() -> Self {
        Self {
            service: Arc::new(Mutex::new(Service::new())),
        }
    }
}

#![warn(clippy::pedantic)]
#![allow(
    clippy::missing_errors_doc,
    clippy::missing_panics_doc,
    clippy::too_many_lines,
    clippy::unused_async_trait_impl
)]

use std::sync::{Arc, Mutex};

use super::Trace;

#[derive(Clone)]
pub struct WrappedTrace {
    trace: Arc<Mutex<Trace>>,
}

impl WrappedTrace {
    #[must_use]
    pub fn new(trace: Trace) -> Self {
        Self {
            trace: Arc::new(Mutex::new(trace)),
        }
    }

    #[must_use]
    pub fn compare_with(&self, target_similarity: f64, other: &WrappedTrace) -> f64 {
        // If both point to the same return 1.0
        if Arc::ptr_eq(&self.trace, &other.trace) {
            return 1.0;
        }
        let self_trace = self.trace.lock().unwrap();
        let other_trace = other.trace.lock().unwrap();
        self_trace.compare_with(target_similarity, &other_trace)
    }
}

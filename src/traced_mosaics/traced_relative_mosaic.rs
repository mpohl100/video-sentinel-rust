#![warn(clippy::pedantic)]
#![allow(
    clippy::missing_errors_doc,
    clippy::missing_panics_doc,
    clippy::too_many_lines,
    clippy::unused_async_trait_impl
)]

use crate::mosaics::WrappedRelativeMosaic;
use crate::traces::{Trace, TraceParams, WrappedTrace};

#[derive(Clone)]
pub struct TracedRelativeMosaic {
    relative_mosaic: WrappedRelativeMosaic,
    trace: WrappedTrace,
}

impl TracedRelativeMosaic {
    #[must_use]
    pub fn new(relative_mosaic: WrappedRelativeMosaic, trace_params: TraceParams) -> Self {
        let trace = WrappedTrace::new(Trace::new_from_mosaic(
            &relative_mosaic.get_mosaic(),
            trace_params,
        ));
        TracedRelativeMosaic {
            relative_mosaic,
            trace,
        }
    }

    #[must_use]
    pub fn get_relative_mosaic(&self) -> WrappedRelativeMosaic {
        self.relative_mosaic.clone()
    }

    #[must_use]
    pub fn get_trace(&self) -> &WrappedTrace {
        &self.trace
    }
}

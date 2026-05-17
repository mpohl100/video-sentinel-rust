use crate::mosaics::WrappedMosaic;
use crate::traces::{Trace, TraceParams};

#[derive(Clone)]
pub struct TracedMosaic {
    mosaic: WrappedMosaic,
    trace: Trace,
}

impl TracedMosaic {
    pub fn new(mosaic: WrappedMosaic, trace_params: TraceParams) -> Self {
        let trace = Trace::new_from_mosaic(mosaic.clone(), trace_params);
        TracedMosaic { mosaic, trace }
    }

    pub fn get_mosaic(&self) -> &WrappedMosaic {
        &self.mosaic
    }

    pub fn get_trace(&self) -> &Trace {
        &self.trace
    }
}

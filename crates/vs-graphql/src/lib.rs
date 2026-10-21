//! Layer 5 adapter: GraphQL schema over `vs-service`.

#[allow(unused_imports)]
pub(crate) use vs_core::{eye, mosaics, object_detection, slices, traces};
#[allow(unused_imports)]
pub(crate) use vs_service::service;

pub mod graphql;

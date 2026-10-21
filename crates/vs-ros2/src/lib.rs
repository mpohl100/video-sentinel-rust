//! Layer 5 adapter: ROS 2 node over `vs-service`.

#[allow(unused_imports)]
pub(crate) use vs_core::{mosaics, slices};
#[allow(unused_imports)]
pub(crate) use vs_service::service;

pub mod ros2_node;

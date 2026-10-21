#![warn(clippy::pedantic)]
#![allow(
    clippy::missing_errors_doc,
    clippy::missing_panics_doc,
    clippy::too_many_lines,
    clippy::unused_async_trait_impl
)]

use rs_math3d::Vec3d;

use super::{CoordinateSystem, CoordinatedPoint, RegionedAngle, WrappedCoordinateSystem};

#[derive(Clone)]
pub enum AnonymizedCoordinateSystem {
    Direct(CoordinateSystem),
    Indirect(WrappedCoordinateSystem),
}

impl AnonymizedCoordinateSystem {
    #[must_use]
    pub fn new_direct(coordinate_system: CoordinateSystem) -> Self {
        AnonymizedCoordinateSystem::Direct(coordinate_system)
    }

    #[must_use]
    pub fn new_indirect(coordinate_system: CoordinateSystem) -> Self {
        AnonymizedCoordinateSystem::Indirect(WrappedCoordinateSystem::new(
            coordinate_system.origin(),
            coordinate_system.x_axis(),
            coordinate_system.y_axis(),
        ))
    }

    pub fn rotate(&mut self, angle: RegionedAngle) {
        match self {
            AnonymizedCoordinateSystem::Direct(cs) => {
                cs.rotate(angle);
            }
            AnonymizedCoordinateSystem::Indirect(wrapped_cs) => {
                wrapped_cs.rotate(angle);
            }
        }
    }

    #[must_use]
    pub fn duplicate(&self) -> Self {
        match self {
            AnonymizedCoordinateSystem::Direct(cs) => AnonymizedCoordinateSystem::Direct(*cs),
            AnonymizedCoordinateSystem::Indirect(wrapped_cs) => {
                AnonymizedCoordinateSystem::Indirect(wrapped_cs.duplicate())
            }
        }
    }

    #[must_use]
    pub fn to_global(&self, point: &CoordinatedPoint) -> CoordinatedPoint {
        match self {
            AnonymizedCoordinateSystem::Direct(cs) => {
                let point = cs.to_global(point);
                let global_coordinate_system =
                    AnonymizedCoordinateSystem::new_direct(CoordinateSystem::new(
                        Vec3d::new(0.0, 0.0, 0.0),
                        Vec3d::new(1.0, 0.0, 0.0),
                        Vec3d::new(0.0, 1.0, 0.0),
                    ));
                CoordinatedPoint::new(global_coordinate_system, point)
            }
            AnonymizedCoordinateSystem::Indirect(wrapped_cs) => {
                let point = wrapped_cs.to_global(point);
                let global_coordinate_system =
                    AnonymizedCoordinateSystem::new_direct(CoordinateSystem::new(
                        Vec3d::new(0.0, 0.0, 0.0),
                        Vec3d::new(1.0, 0.0, 0.0),
                        Vec3d::new(0.0, 1.0, 0.0),
                    ));
                CoordinatedPoint::new(global_coordinate_system, point)
            }
        }
    }

    #[must_use]
    pub fn to_local(&self, global_point: &CoordinatedPoint) -> CoordinatedPoint {
        match self {
            AnonymizedCoordinateSystem::Direct(cs) => cs.to_local(global_point),
            AnonymizedCoordinateSystem::Indirect(wrapped_cs) => wrapped_cs.to_local(global_point),
        }
    }

    #[must_use]
    pub fn convert_from_global(&self, global_point: &CoordinatedPoint) -> CoordinatedPoint {
        let global_coordinates = global_point.get_local_point();
        match self {
            AnonymizedCoordinateSystem::Direct(cs) => cs.convert_from_global(global_coordinates),
            AnonymizedCoordinateSystem::Indirect(wrapped_cs) => {
                wrapped_cs.convert_from_global(global_coordinates)
            }
        }
    }

    #[must_use]
    pub fn get_angle_between(&self, other: &AnonymizedCoordinateSystem) -> RegionedAngle {
        match self {
            AnonymizedCoordinateSystem::Direct(cs) => cs.get_angle_between(other),
            AnonymizedCoordinateSystem::Indirect(wrapped_cs) => wrapped_cs.get_angle_between(other),
        }
    }

    pub fn align_x_axis_with(&mut self, other: &AnonymizedCoordinateSystem) {
        match self {
            AnonymizedCoordinateSystem::Direct(cs) => cs.align_x_axis_with(other),
            AnonymizedCoordinateSystem::Indirect(wrapped_cs) => wrapped_cs.align_x_axis_with(other),
        }
    }
}

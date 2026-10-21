#![warn(clippy::pedantic)]
#![allow(
    clippy::missing_errors_doc,
    clippy::missing_panics_doc,
    clippy::too_many_lines,
    clippy::unused_async_trait_impl
)]

use rs_math3d::Vector;
use rs_math3d::{CrossProduct, Vec3d, Vector3};
use std::sync::{Arc, Mutex};

use super::{AnonymizedCoordinateSystem, CoordinateSystem, CoordinatedPoint, RegionedAngle};

#[derive(Clone)]
pub struct WrappedCoordinateSystem {
    coordinate_system: Arc<Mutex<CoordinateSystem>>,
}

impl WrappedCoordinateSystem {
    pub(super) fn coordinate_system(&self) -> &Arc<Mutex<CoordinateSystem>> {
        &self.coordinate_system
    }
}

impl WrappedCoordinateSystem {
    #[must_use]
    pub fn new(origin: Vec3d, x_axis: Vec3d, y_axis: Vec3d) -> Self {
        let coordinate_system = CoordinateSystem::new(origin, x_axis, y_axis);
        Self {
            coordinate_system: Arc::new(Mutex::new(coordinate_system)),
        }
    }

    pub fn rotate(&self, angle: RegionedAngle) {
        let mut cs = self.coordinate_system.lock().unwrap();
        cs.rotate(angle);
    }

    #[must_use]
    pub fn to_local(&self, point: &CoordinatedPoint) -> CoordinatedPoint {
        let cs = self.coordinate_system.lock().unwrap();
        cs.to_local(point)
    }

    #[must_use]
    pub fn convert_from_global(&self, point: Vec3d) -> CoordinatedPoint {
        let cs = self.coordinate_system.lock().unwrap();
        cs.convert_from_global(point)
    }

    #[must_use]
    pub fn to_global(&self, point: &CoordinatedPoint) -> Vec3d {
        let cs = self.coordinate_system.lock().unwrap();
        cs.to_global(point)
    }

    #[must_use]
    pub fn get_angle_between(&self, other: &AnonymizedCoordinateSystem) -> RegionedAngle {
        let self_x_axis = self.coordinate_system.lock().unwrap().x_axis();
        let other_x_axis = match other {
            AnonymizedCoordinateSystem::Direct(cs) => cs.x_axis(),
            AnonymizedCoordinateSystem::Indirect(wrapped_cs) => {
                wrapped_cs.coordinate_system.lock().unwrap().x_axis()
            }
        };
        let dot = Vector3::<f64>::dot(&self_x_axis, &other_x_axis);
        let cross = Vector3::<f64>::cross(&self_x_axis, &other_x_axis);

        RegionedAngle::new(cross.z.atan2(dot).to_degrees(), -180.0, 180.0)
    }

    #[must_use]
    pub fn duplicate(&self) -> Self {
        let cs = self.coordinate_system.lock().unwrap();
        WrappedCoordinateSystem::new(cs.origin(), cs.x_axis(), cs.y_axis())
    }

    pub fn align_x_axis_with(&self, other: &AnonymizedCoordinateSystem) {
        let angle_between = self.get_angle_between(other);
        let angle_to_rotate = RegionedAngle::new(
            -angle_between.angle_degrees,
            angle_between.min_degrees,
            angle_between.max_degrees,
        );
        self.rotate(angle_to_rotate);
    }
}

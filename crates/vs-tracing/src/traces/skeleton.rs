use vs_math::math::CoordinateSystem;

use crate::traces::trace_params::TraceParams;
use crate::traces::funcs::f64_from_usize;
use crate::math::AnonymizedCoordinateSystem;

use rs_math3d::Vec3d;


#[derive(Clone)]
pub struct Skeleton {
    coordinate_system: AnonymizedCoordinateSystem,
    delta_angle: f64,
    radius: f64,
}

impl Default for Skeleton {
    fn default() -> Self {
        Self {
            coordinate_system: AnonymizedCoordinateSystem::Direct(CoordinateSystem::new(
                Vec3d::new(0.0, 0.0, 0.0),
                Vec3d::new(1.0, 0.0, 0.0),
                Vec3d::new(0.0, 1.0, 0.0),
            )),
            delta_angle: 0.0,
            radius: 0.0,
        }
    }
}

impl Skeleton {
    pub fn new_from_trace_params(coordinate_system: AnonymizedCoordinateSystem, trace_params: TraceParams, radius: f64) -> Self {
        let delta_angle = 360.0 / f64_from_usize(trace_params.num_skeleton());
        Self {
            coordinate_system,
            delta_angle,
            radius,
        }
    }

    pub fn num_skeleton(&self) -> usize {
        (360.0 / self.delta_angle) as usize
    }

    pub fn get_area(&self) -> f64 {
        std::f64::consts::PI * self.radius.powi(2)
    }

    pub fn get_coordinate_system(&self) -> AnonymizedCoordinateSystem {
        self.coordinate_system.duplicate()
    }

    pub fn get_radius(&self) -> f64 {
        self.radius
    }

    pub fn get_delta_angle_degrees(&self) -> f64 {
        self.delta_angle
    }
}
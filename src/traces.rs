use crate::math::CoordinatedLine;
use crate::math::CoordinatedPoint;
use crate::math::CoordinatedRectangle;
use crate::math::Rectangle;
use crate::math::RegionedAngle;
use crate::math::WrappedCoordinateSystem;
use crate::shapes::WrappedShape;
use crate::slices::Slice;

use rs_math3d::Vec3d;

#[derive(Clone)]
struct RatioLine {
    coordinate_system: WrappedCoordinateSystem,
    slices: Vec<Slice>,
}

pub struct TraceParams {
    angle_skeleton: usize,
    close_slice_threshold: f64,
}

#[derive(Clone)]
pub struct Trace {
    ratio_lines: Vec<RatioLine>,
}

impl Trace {
    pub fn new_from_shape(shape: WrappedShape, params: TraceParams) -> Self {
        let ratio_lines = (0..params.angle_skeleton)
            .map(|i| {
                let coordinate_system = WrappedCoordinateSystem::new(
                    shape.get_center_of_mass(),
                    Vec3d::new(1.0, 0.0, 0.0),
                    Vec3d::new(0.0, 1.0, 0.0),
                );
                coordinate_system.rotate(RegionedAngle::new(
                    (i as f64) * (180.0 / params.angle_skeleton as f64),
                    -180.0,
                    180.0,
                ));
                RatioLine {
                    coordinate_system: coordinate_system.clone(),
                    slices: deduce_slices_from_shape(
                        vec![shape.clone()],
                        coordinate_system.clone(),
                        shape.get_bounding_circle().get_radius(),
                        &params,
                    ),
                }
            })
            .collect();
        Trace { ratio_lines }
    }
}

fn deduce_slices_from_shape(
    shapes: Vec<WrappedShape>,
    coordinate_system: WrappedCoordinateSystem,
    radius: f64,
    params: &TraceParams,
) -> Vec<Slice> {
    let mut slices = Vec::new();
    // for every x in the range of -radius to radius with a step of 0.5, find the intersections with the shape and create slices
    let step = 0.5;
    let mut x = -radius;
    while x <= radius {
        let global_coordinate_system = WrappedCoordinateSystem::new(
            Vec3d::new(0.0, 0.0, 0.0),
            Vec3d::new(1.0, 0.0, 0.0),
            Vec3d::new(0.0, 1.0, 0.0),
        );
        let point = CoordinatedPoint::new(coordinate_system.clone(), Vec3d::new(x, 0.0, 0.0));
        let contains_point = shapes
            .iter()
            .any(|shape| shape.contains_point(point.clone()));
        if contains_point {
            let global_point = point.convert_to(global_coordinate_system.clone());
            let tl = Vec3d::new(
                global_point.get_x().floor(),
                global_point.get_y().ceil(),
                0.0,
            );
            let br = Vec3d::new(
                global_point.get_x().ceil(),
                global_point.get_y().floor(),
                0.0,
            );
            let rectangle = Rectangle::new(tl, br);
            let coordinated_rectangle =
                CoordinatedRectangle::new_from_rectangle(rectangle, global_coordinate_system)
                    .convert_to(coordinate_system.clone());
            let x_line_start =
                CoordinatedPoint::new(coordinate_system.clone(), Vec3d::new(-radius, 0.0, 0.0));
            let x_line_end =
                CoordinatedPoint::new(coordinate_system.clone(), Vec3d::new(radius, 0.0, 0.0));
            let x_axis_line = CoordinatedLine::new(x_line_start, x_line_end);
            let clipped_line = coordinated_rectangle.get_intersection_line(x_axis_line);
            if let Some(clipped_line) = clipped_line {
                let slice = Slice::new(
                    clipped_line.get_start().get_local_point(),
                    clipped_line.get_end().get_local_point(),
                );
                slices.push(slice);
            }
        }
        x += step;
    }
    combine_close_slices(slices, params.close_slice_threshold)
}

fn combine_close_slices(slices: Vec<Slice>, threshold: f64) -> Vec<Slice> {
    if slices.is_empty() {
        return slices;
    }
    let mut combined_slices = Vec::new();
    let mut current_slice = slices[0].clone();
    for slice in slices.iter().skip(1) {
        if slice.get_start().x - current_slice.get_end().x <= threshold {
            current_slice = Slice::new(current_slice.get_start(), slice.get_end());
        } else {
            combined_slices.push(current_slice);
            current_slice = slice.clone();
        }
    }
    combined_slices.push(current_slice);
    combined_slices
}

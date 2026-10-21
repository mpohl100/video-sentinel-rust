#![warn(clippy::pedantic)]
#![allow(
    clippy::missing_errors_doc,
    clippy::missing_panics_doc,
    clippy::too_many_lines,
    clippy::unused_async_trait_impl
)]

use rs_math3d::Vector;
use rs_math3d::{Vec3d, Vector3};

use super::{Circle, Line, Rectangle};

#[must_use]
pub fn expand_rectangle(rectangle: &Rectangle, expansion: f64) -> Rectangle {
    let top_left = rectangle.get_top_left();
    let bottom_right = rectangle.get_bottom_right();
    let expanded_top_left = Vec3d::new(top_left.x - expansion, top_left.y - expansion, 0.0);
    let expanded_bottom_right =
        Vec3d::new(bottom_right.x + expansion, bottom_right.y + expansion, 0.0);
    Rectangle::new(expanded_top_left, expanded_bottom_right)
}

pub(super) fn get_circle_line_intersection_points(circle: &Circle, line: &Line) -> Vec<Vec3d> {
    let line_delta = line.end() - line.start();
    let center_offset = line.start() - circle.center();

    let quadratic_a = Vector3::<f64>::dot(&line_delta, &line_delta);
    let quadratic_b = 2.0 * Vector3::<f64>::dot(&center_offset, &line_delta);
    let quadratic_c =
        Vector3::<f64>::dot(&center_offset, &center_offset) - circle.radius() * circle.radius();

    let discriminant = quadratic_b * quadratic_b - 4.0 * quadratic_a * quadratic_c;
    if discriminant < 0.0 {
        vec![] // No intersection
    } else {
        let sqrt_discriminant = discriminant.sqrt();
        let t1 = (-quadratic_b - sqrt_discriminant) / (2.0 * quadratic_a);
        let t2 = (-quadratic_b + sqrt_discriminant) / (2.0 * quadratic_a);
        let mut points = Vec::new();
        if (0.0..=1.0).contains(&t1) {
            points.push(line.start() + line_delta * t1);
        }
        if (0.0..=1.0).contains(&t2) {
            points.push(line.start() + line_delta * t2);
        }
        points
    }
}

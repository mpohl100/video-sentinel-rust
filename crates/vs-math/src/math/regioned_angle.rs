#![warn(clippy::pedantic)]
#![allow(
    clippy::missing_errors_doc,
    clippy::missing_panics_doc,
    clippy::too_many_lines,
    clippy::unused_async_trait_impl
)]

use rs_math3d::Vec3d;

use super::Line;

#[derive(Clone, Copy)]
pub struct RegionedAngle {
    pub angle_degrees: f64,
    pub min_degrees: f64,
    pub max_degrees: f64,
}

impl RegionedAngle {
    #[must_use]
    pub fn new(degrees: f64, min_degrees: f64, max_degrees: f64) -> Self {
        let mut regioned_angle = Self {
            angle_degrees: degrees,
            min_degrees,
            max_degrees,
        };
        regioned_angle.adjust();
        regioned_angle
    }

    #[must_use]
    pub fn new_from_points(
        start: Vec3d,
        end: Vec3d,
        mid: Vec3d,
        min_degrees: f64,
        max_degrees: f64,
    ) -> Self {
        let start_to_mid_line = Line::new(start, mid);
        let start_to_end_line = Line::new(start, end);
        let angle_radians = start_to_mid_line
            .angle_between(&start_to_end_line)
            .radians();
        let angle_degrees = angle_radians.to_degrees();
        let mut regioned_angle = Self::new(angle_degrees, min_degrees, max_degrees);
        regioned_angle.adjust();
        regioned_angle
    }

    #[must_use]
    pub fn new_from_lines(line1: Line, line2: Line, min_degrees: f64, max_degrees: f64) -> Self {
        let angle_radians = line1.angle_between(&line2).radians();
        let angle_degrees = angle_radians.to_degrees();
        let mut regioned_angle = Self::new(angle_degrees, min_degrees, max_degrees);
        regioned_angle.adjust();
        regioned_angle
    }

    #[must_use]
    pub fn radians(&self) -> f64 {
        self.angle_degrees.to_radians()
    }

    #[must_use]
    pub fn get_min_degrees(&self) -> f64 {
        self.min_degrees
    }

    #[must_use]
    pub fn get_max_degrees(&self) -> f64 {
        self.max_degrees
    }

    #[must_use]
    pub fn add_angle(&self, other: RegionedAngle) -> RegionedAngle {
        let new_angle_degrees = self.angle_degrees + other.angle_degrees;
        let mut result = Self::new(new_angle_degrees, self.min_degrees, self.max_degrees);
        result.adjust();
        result
    }

    fn adjust(&mut self) {
        while self.angle_degrees < self.min_degrees {
            self.angle_degrees += 360.0;
        }
        while self.angle_degrees > self.max_degrees {
            self.angle_degrees -= 360.0;
        }
    }

    #[must_use]
    pub fn inverted(&self) -> Self {
        Self::new(-self.angle_degrees, self.min_degrees, self.max_degrees)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::math::*;

    use rs_math3d::Vec3d;

    const EPSILON: f64 = 1e-8;

    fn assert_float_eq(actual: f64, expected: f64) {
        assert!(
            (actual - expected).abs() <= EPSILON,
            "expected {expected}, got {actual}"
        );
    }

    fn assert_vec_eq(actual: Vec3d, expected: Vec3d) {
        assert_float_eq(actual.x, expected.x);
        assert_float_eq(actual.y, expected.y);
        assert_float_eq(actual.z, expected.z);
    }

    fn global_coordinate_system() -> AnonymizedCoordinateSystem {
        AnonymizedCoordinateSystem::Direct(CoordinateSystem::new(
            Vec3d::new(0.0, 0.0, 0.0),
            Vec3d::new(1.0, 0.0, 0.0),
            Vec3d::new(0.0, 1.0, 0.0),
        ))
    }

    fn translated_coordinate_system() -> AnonymizedCoordinateSystem {
        AnonymizedCoordinateSystem::Direct(CoordinateSystem::new(
            Vec3d::new(10.0, -5.0, 0.0),
            Vec3d::new(1.0, 0.0, 0.0),
            Vec3d::new(0.0, 1.0, 0.0),
        ))
    }

    fn rotated_coordinate_system() -> AnonymizedCoordinateSystem {
        AnonymizedCoordinateSystem::Direct(CoordinateSystem::new(
            Vec3d::new(1.0, 2.0, 0.0),
            Vec3d::new(0.0, 1.0, 0.0),
            Vec3d::new(-1.0, 0.0, 0.0),
        ))
    }

    fn assert_global_point_eq(point: &CoordinatedPoint, expected: Vec3d) {
        let global_point = point.convert_to(global_coordinate_system());
        assert_vec_eq(global_point.get_local_point(), expected);
    }

    #[test]
    fn line_methods_cover_equality_intersections_and_angles() {
        let line = Line::new(Vec3d::new(0.0, 0.0, 0.0), Vec3d::new(2.0, 2.0, 0.0));
        let almost_equal = Line::new(Vec3d::new(0.0, 0.0, 0.0), Vec3d::new(2.0, 2.000_000_5, 0.0));
        let different = Line::new(Vec3d::new(0.0, 0.0, 0.0), Vec3d::new(2.0, 2.01, 0.0));
        let crossing = Line::new(Vec3d::new(0.0, 2.0, 0.0), Vec3d::new(2.0, 0.0, 0.0));
        let parallel = Line::new(Vec3d::new(0.0, 1.0, 0.0), Vec3d::new(2.0, 1.0, 0.0));
        let horizontal = Line::new(Vec3d::new(0.0, 0.0, 0.0), Vec3d::new(2.0, 0.0, 0.0));
        let vertical = Line::new(Vec3d::new(1.0, -1.0, 0.0), Vec3d::new(1.0, 1.0, 0.0));
        let disjoint = Line::new(Vec3d::new(3.0, 0.0, 0.0), Vec3d::new(4.0, 0.0, 0.0));
        let zero_length = Line::new(Vec3d::new(1.0, 1.0, 0.0), Vec3d::new(1.0, 1.0, 0.0));

        assert_vec_eq(line.start(), Vec3d::new(0.0, 0.0, 0.0));
        assert_vec_eq(line.end(), Vec3d::new(2.0, 2.0, 0.0));
        assert!(line == almost_equal);
        assert!(line != different);
        assert!(line.intersects(&crossing));
        assert!(horizontal.intersects(&vertical));
        assert!(!horizontal.intersects(&parallel));
        assert!(!horizontal.intersects(&disjoint));
        assert_vec_eq(
            line.get_intersection_point(&crossing).point,
            Vec3d::new(1.0, 1.0, 0.0),
        );
        assert_vec_eq(
            horizontal.get_intersection_point(&parallel).point,
            Vec3d::new(1.0, 0.0, 0.0),
        );
        assert_float_eq(horizontal.angle_between(&vertical).angle_degrees, 90.0);
        assert_float_eq(zero_length.angle_between(&horizontal).angle_degrees, 0.0);
    }

    #[test]
    fn regioned_angle_methods_cover_adjustment_and_construction() {
        let adjusted_positive = RegionedAngle::new(450.0, -180.0, 180.0);
        let adjusted_negative = RegionedAngle::new(-270.0, -180.0, 180.0);
        let from_points = RegionedAngle::new_from_points(
            Vec3d::new(0.0, 0.0, 0.0),
            Vec3d::new(0.0, 2.0, 0.0),
            Vec3d::new(2.0, 0.0, 0.0),
            -180.0,
            180.0,
        );
        let from_lines = RegionedAngle::new_from_lines(
            Line::new(Vec3d::new(0.0, 0.0, 0.0), Vec3d::new(2.0, 0.0, 0.0)),
            Line::new(Vec3d::new(0.0, 0.0, 0.0), Vec3d::new(0.0, 2.0, 0.0)),
            -180.0,
            180.0,
        );
        let pi_angle = RegionedAngle::new(180.0, -180.0, 180.0);
        let sum = RegionedAngle::new(150.0, -180.0, 180.0)
            .add_angle(RegionedAngle::new(60.0, -180.0, 180.0));

        assert_float_eq(adjusted_positive.angle_degrees, 90.0);
        assert_float_eq(adjusted_negative.angle_degrees, 90.0);
        assert_float_eq(from_points.angle_degrees, 90.0);
        assert_float_eq(from_lines.angle_degrees, 90.0);
        assert_float_eq(pi_angle.radians(), std::f64::consts::PI);
        assert_float_eq(pi_angle.get_min_degrees(), -180.0);
        assert_float_eq(pi_angle.get_max_degrees(), 180.0);
        assert_float_eq(sum.angle_degrees, -150.0);
    }

    #[test]
    fn rectangle_and_expansion_methods_cover_dimensions_and_intersections() {
        let rectangle = Rectangle::new(Vec3d::new(1.0, 2.0, 0.0), Vec3d::new(5.0, 8.0, 0.0));
        let from_lines = Rectangle::new(rectangle.points()[0].point, rectangle.points()[1].point);
        let overlapping = Rectangle::new(Vec3d::new(4.0, 4.0, 0.0), Vec3d::new(7.0, 10.0, 0.0));
        let disjoint = Rectangle::new(Vec3d::new(6.0, 9.0, 0.0), Vec3d::new(8.0, 12.0, 0.0));
        let expanded = expand_rectangle(&rectangle, 1.5);

        assert_eq!(rectangle.points().len(), 2);
        assert!(from_lines == rectangle);
        assert_float_eq(rectangle.get_area(), 24.0);
        assert_vec_eq(rectangle.get_center(), Vec3d::new(3.0, 5.0, 0.0));
        assert_vec_eq(rectangle.get_top_left(), Vec3d::new(1.0, 2.0, 0.0));
        assert_vec_eq(rectangle.get_bottom_right(), Vec3d::new(5.0, 8.0, 0.0));
        assert_float_eq(rectangle.get_width(), 4.0);
        assert_float_eq(rectangle.get_height(), 6.0);
        assert!(rectangle.intersects(&overlapping));
        assert!(!rectangle.intersects(&disjoint));
        assert_vec_eq(expanded.get_top_left(), Vec3d::new(-0.5, 0.5, 0.0));
        assert_vec_eq(expanded.get_bottom_right(), Vec3d::new(6.5, 9.5, 0.0));
    }

    #[test]
    fn circle_methods_cover_geometry_and_intersections() {
        let circle = Circle::new(Vec3d::new(2.0, -1.0, 0.0), 2.0);
        let almost_equal = Circle::new(Vec3d::new(2.0, -1.0, 0.0), 2.000_000_5);
        let overlapping = Circle::new(Vec3d::new(5.0, -1.0, 0.0), 2.0);
        let tangent = Circle::new(Vec3d::new(6.0, -1.0, 0.0), 2.0);
        let separate = Circle::new(Vec3d::new(7.0, -1.0, 0.0), 2.0);
        let bounding_box = circle.get_bounding_box();

        assert_vec_eq(circle.get_center(), Vec3d::new(2.0, -1.0, 0.0));
        assert_float_eq(circle.get_radius(), 2.0);
        assert!(circle == almost_equal);
        assert!(circle.intersects(&overlapping));
        assert!(!circle.intersects(&tangent));
        assert!(!circle.intersects(&separate));
        assert_vec_eq(bounding_box.get_top_left(), Vec3d::new(0.0, -3.0, 0.0));
        assert_vec_eq(bounding_box.get_bottom_right(), Vec3d::new(4.0, 1.0, 0.0));
        assert_float_eq(circle.get_area(), 12.566_370_614_359_172);
    }

    #[test]
    fn coordinate_system_methods_cover_rotation_and_coordinate_conversion() {
        let translated = CoordinateSystem::new(
            Vec3d::new(10.0, -5.0, 0.0),
            Vec3d::new(1.0, 0.0, 0.0),
            Vec3d::new(0.0, 1.0, 0.0),
        );
        let rotated = CoordinateSystem::new(
            Vec3d::new(1.0, 2.0, 0.0),
            Vec3d::new(0.0, 1.0, 0.0),
            Vec3d::new(-1.0, 0.0, 0.0),
        );
        let local_point = translated.convert_from_global(Vec3d::new(12.0, -2.0, 0.0));
        let point_in_global =
            CoordinatedPoint::new(global_coordinate_system(), Vec3d::new(12.0, -2.0, 0.0));
        let mut standard = CoordinateSystem::new(
            Vec3d::new(0.0, 0.0, 0.0),
            Vec3d::new(1.0, 0.0, 0.0),
            Vec3d::new(0.0, 1.0, 0.0),
        );

        assert_vec_eq(translated.origin(), Vec3d::new(10.0, -5.0, 0.0));
        assert_vec_eq(translated.x_axis(), Vec3d::new(1.0, 0.0, 0.0));
        assert_vec_eq(translated.y_axis(), Vec3d::new(0.0, 1.0, 0.0));
        assert_vec_eq(local_point.get_local_point(), Vec3d::new(2.0, 3.0, 0.0));
        assert_vec_eq(
            translated.to_global(&CoordinatedPoint::new(
                translated_coordinate_system(),
                Vec3d::new(2.0, 3.0, 0.0),
            )),
            Vec3d::new(12.0, -2.0, 0.0),
        );
        assert_vec_eq(
            rotated.to_local_coordinates(Vec3d::new(4.0, 5.0, 0.0)),
            Vec3d::new(3.0, -3.0, 0.0),
        );
        assert_vec_eq(
            translated.to_local(&point_in_global).get_local_point(),
            Vec3d::new(2.0, 3.0, 0.0),
        );

        standard.rotate(RegionedAngle::new(90.0, -180.0, 180.0));

        assert_vec_eq(standard.x_axis(), Vec3d::new(0.0, -1.0, 0.0));
        assert_vec_eq(standard.y_axis(), Vec3d::new(1.0, 0.0, 0.0));
        assert_vec_eq(
            standard.to_global(&CoordinatedPoint::new(
                global_coordinate_system(),
                Vec3d::new(1.0, 0.0, 0.0),
            )),
            Vec3d::new(0.0, -1.0, 0.0),
        );
    }

    #[test]
    fn wrapped_coordinate_system_methods_cover_round_trips_angles_and_alignment() {
        let translated = translated_coordinate_system();
        let point = CoordinatedPoint::new(translated.clone(), Vec3d::new(2.0, 3.0, 0.0));
        let mut global = global_coordinate_system();
        let rotated = AnonymizedCoordinateSystem::Indirect(WrappedCoordinateSystem::new(
            Vec3d::new(0.0, 0.0, 0.0),
            Vec3d::new(0.0, 1.0, 0.0),
            Vec3d::new(-1.0, 0.0, 0.0),
        ));
        let duplicate = global.duplicate();
        let mut alignable = global_coordinate_system();

        assert_vec_eq(
            translated.to_global(&point).get_local_point(),
            Vec3d::new(12.0, -2.0, 0.0),
        );
        assert_vec_eq(
            translated
                .convert_from_global(&CoordinatedPoint::new(
                    global_coordinate_system(),
                    Vec3d::new(12.0, -2.0, 0.0),
                ))
                .get_local_point(),
            Vec3d::new(2.0, 3.0, 0.0),
        );
        assert_vec_eq(
            translated
                .to_local(&CoordinatedPoint::new(
                    global.clone(),
                    Vec3d::new(12.0, -2.0, 0.0),
                ))
                .get_local_point(),
            Vec3d::new(2.0, 3.0, 0.0),
        );
        assert_float_eq(global.get_angle_between(&rotated).angle_degrees, 90.0);
        assert_float_eq(rotated.get_angle_between(&global).angle_degrees, -90.0);

        global.rotate(RegionedAngle::new(90.0, -180.0, 180.0));

        assert_vec_eq(
            global
                .to_global(&CoordinatedPoint::new(
                    global.clone(),
                    Vec3d::new(1.0, 0.0, 0.0),
                ))
                .get_local_point(),
            Vec3d::new(0.0, -1.0, 0.0),
        );
        assert_vec_eq(
            duplicate
                .to_global(&CoordinatedPoint::new(
                    duplicate.clone(),
                    Vec3d::new(1.0, 0.0, 0.0),
                ))
                .get_local_point(),
            Vec3d::new(1.0, 0.0, 0.0),
        );

        alignable.align_x_axis_with(&rotated);

        assert_float_eq(alignable.get_angle_between(&rotated).angle_degrees, 0.0);
    }

    #[test]
    fn coordinated_point_methods_cover_conversion_rotation_and_distance() {
        let translated = translated_coordinate_system();
        let rotated = rotated_coordinate_system();
        let point = CoordinatedPoint::new(translated.clone(), Vec3d::new(2.0, 3.0, 0.0));
        let same_global =
            CoordinatedPoint::new(global_coordinate_system(), Vec3d::new(12.0, -2.0, 0.0));
        let plus_result = point.plus(Vec3d::new(1.0, -2.0, 0.0));
        let rotated_point =
            CoordinatedPoint::new(global_coordinate_system(), Vec3d::new(2.0, 0.0, 0.0)).rotate(
                &CoordinatedPoint::new(global_coordinate_system(), Vec3d::new(0.0, 0.0, 0.0)),
                RegionedAngle::new(90.0, -180.0, 180.0),
            );

        assert!(point == same_global);
        assert_vec_eq(
            point
                .convert_to(global_coordinate_system())
                .get_local_point(),
            Vec3d::new(12.0, -2.0, 0.0),
        );
        assert_vec_eq(
            point.convert_to(rotated).get_local_point(),
            Vec3d::new(-4.0, -11.0, 0.0),
        );
        assert_vec_eq(plus_result.get_local_point(), Vec3d::new(3.0, 1.0, 0.0));
        assert_global_point_eq(&rotated_point, Vec3d::new(0.0, 2.0, 0.0));
        assert_float_eq(
            point.distance_to(&CoordinatedPoint::new(
                global_coordinate_system(),
                Vec3d::new(15.0, 2.0, 0.0),
            )),
            5.0,
        );
        assert_float_eq(point.get_x(), 2.0);
        assert_float_eq(point.get_y(), 3.0);
        assert_float_eq(point.get_z(), 0.0);
        assert_vec_eq(point.get_local_point(), Vec3d::new(2.0, 3.0, 0.0));
    }

    #[test]
    fn coordinated_line_methods_cover_length_conversion_and_intersection() {
        let translated = translated_coordinate_system();
        let line = CoordinatedLine::new(
            CoordinatedPoint::new(translated.clone(), Vec3d::new(2.0, 3.0, 0.0)),
            CoordinatedPoint::new(translated.clone(), Vec3d::new(5.0, 7.0, 0.0)),
        );
        let converted = line.convert_to(global_coordinate_system());
        let diagonal = CoordinatedLine::new(
            CoordinatedPoint::new(global_coordinate_system(), Vec3d::new(0.0, 0.0, 0.0)),
            CoordinatedPoint::new(global_coordinate_system(), Vec3d::new(4.0, 4.0, 0.0)),
        );
        let cross_system = CoordinatedLine::new(
            CoordinatedPoint::new(translated.clone(), Vec3d::new(-10.0, 9.0, 0.0)),
            CoordinatedPoint::new(translated.clone(), Vec3d::new(-6.0, 5.0, 0.0)),
        );
        let separate = CoordinatedLine::new(
            CoordinatedPoint::new(global_coordinate_system(), Vec3d::new(5.0, 0.0, 0.0)),
            CoordinatedPoint::new(global_coordinate_system(), Vec3d::new(6.0, 0.0, 0.0)),
        );

        assert_float_eq(line.length(), 5.0);
        assert_vec_eq(
            converted.get_start().get_local_point(),
            Vec3d::new(12.0, -2.0, 0.0),
        );
        assert_vec_eq(
            converted.get_end().get_local_point(),
            Vec3d::new(15.0, 2.0, 0.0),
        );
        assert_vec_eq(line.to_global_line().start(), Vec3d::new(12.0, -2.0, 0.0));
        assert_vec_eq(line.to_global_line().end(), Vec3d::new(15.0, 2.0, 0.0));
        assert!(diagonal.intersects(&cross_system));
        assert!(!diagonal.intersects(&separate));

        let intersection = diagonal.get_intersection_point(&cross_system);

        assert!(intersection.is_some());
        assert_global_point_eq(&intersection.unwrap(), Vec3d::new(2.0, 2.0, 0.0));
        assert_global_point_eq(&line.get_start(), Vec3d::new(12.0, -2.0, 0.0));
        assert_global_point_eq(&line.get_end(), Vec3d::new(15.0, 2.0, 0.0));
    }

    #[test]
    fn coordinated_rectangle_methods_cover_conversion_and_intersection_lines() {
        let rectangle = CoordinatedRectangle::new(
            CoordinatedPoint::new(global_coordinate_system(), Vec3d::new(0.0, 0.0, 0.0)),
            CoordinatedPoint::new(global_coordinate_system(), Vec3d::new(4.0, 4.0, 0.0)),
        );
        let source = Rectangle::new(Vec3d::new(10.0, -5.0, 0.0), Vec3d::new(14.0, -1.0, 0.0));
        let from_rectangle =
            CoordinatedRectangle::new_from_rectangle(&source, &translated_coordinate_system());
        let converted = rectangle.convert_to(&rotated_coordinate_system());
        let overlapping = CoordinatedRectangle::new_from_rectangle(
            &Rectangle::new(Vec3d::new(2.0, 2.0, 0.0), Vec3d::new(6.0, 6.0, 0.0)),
            &translated_coordinate_system(),
        );
        let line = CoordinatedLine::new(
            CoordinatedPoint::new(global_coordinate_system(), Vec3d::new(-1.0, 2.0, 0.0)),
            CoordinatedPoint::new(global_coordinate_system(), Vec3d::new(5.0, 2.0, 0.0)),
        );

        assert_vec_eq(
            rectangle.to_global_rectangle().get_top_left(),
            Vec3d::new(0.0, 0.0, 0.0),
        );
        assert_vec_eq(
            rectangle.to_global_rectangle().get_bottom_right(),
            Vec3d::new(4.0, 4.0, 0.0),
        );
        assert!(from_rectangle.to_global_rectangle() == source);
        assert!(converted.to_global_rectangle() == rectangle.to_global_rectangle());
        assert!(rectangle.intersects(&overlapping));

        let intersection = rectangle.get_intersection_line(&line);

        assert!(intersection.is_some());
        let intersection = intersection.unwrap();
        assert_global_point_eq(&intersection.get_start(), Vec3d::new(0.0, 2.0, 0.0));
        assert_global_point_eq(&intersection.get_end(), Vec3d::new(4.0, 2.0, 0.0));
    }

    #[test]
    fn coordinated_point_convert_to_same_global_system_is_no_op() {
        let global_coordinate_system = global_coordinate_system();
        let point =
            CoordinatedPoint::new(global_coordinate_system.clone(), Vec3d::new(2.0, 3.0, 0.0));

        let converted = point.convert_to(global_coordinate_system);

        assert_vec_eq(converted.get_local_point(), Vec3d::new(2.0, 3.0, 0.0));
    }

    #[test]
    fn coordinated_circle_and_circle_line_helper_cover_global_and_local_cases() {
        let translated = translated_coordinate_system();
        let circle = CoordinatedCircle::new(
            CoordinatedPoint::new(translated.clone(), Vec3d::new(2.0, 3.0, 0.0)),
            5.0,
        );
        let converted = circle.convert_to(&global_coordinate_system());
        let overlapping = CoordinatedCircle::new(
            CoordinatedPoint::new(global_coordinate_system(), Vec3d::new(16.0, -2.0, 0.0)),
            2.0,
        );
        let bbox = circle.get_bounding_box();
        let secant_points = get_circle_line_intersection_points(
            &Circle::new(Vec3d::new(0.0, 0.0, 0.0), 2.0),
            &Line::new(Vec3d::new(-3.0, 0.0, 0.0), Vec3d::new(3.0, 0.0, 0.0)),
        );
        let tangent_points = get_circle_line_intersection_points(
            &Circle::new(Vec3d::new(0.0, 0.0, 0.0), 2.0),
            &Line::new(Vec3d::new(2.0, -3.0, 0.0), Vec3d::new(2.0, 3.0, 0.0)),
        );
        let no_points = get_circle_line_intersection_points(
            &Circle::new(Vec3d::new(0.0, 0.0, 0.0), 2.0),
            &Line::new(Vec3d::new(3.0, 3.0, 0.0), Vec3d::new(5.0, 5.0, 0.0)),
        );
        let line = CoordinatedLine::new(
            CoordinatedPoint::new(global_coordinate_system(), Vec3d::new(6.0, -2.0, 0.0)),
            CoordinatedPoint::new(global_coordinate_system(), Vec3d::new(18.0, -2.0, 0.0)),
        );

        assert_global_point_eq(&circle.get_center(), Vec3d::new(12.0, -2.0, 0.0));
        assert_float_eq(circle.get_radius(), 5.0);
        assert_global_point_eq(&converted.get_center(), Vec3d::new(12.0, -2.0, 0.0));
        assert!(circle.intersects(&overlapping));
        assert_vec_eq(
            bbox.to_global_rectangle().get_top_left(),
            Vec3d::new(7.0, -7.0, 0.0),
        );
        assert_vec_eq(
            bbox.to_global_rectangle().get_bottom_right(),
            Vec3d::new(17.0, 3.0, 0.0),
        );
        assert_float_eq(circle.get_area(), 78.539_816_339_744_83);
        assert!(circle.contains_point(&CoordinatedPoint::new(
            global_coordinate_system(),
            Vec3d::new(15.0, 2.0, 0.0),
        )));
        assert!(!circle.contains_point(&CoordinatedPoint::new(
            global_coordinate_system(),
            Vec3d::new(18.0, 2.0, 0.0),
        )));

        let intersection = circle.get_intersection_line(&line);

        assert!(intersection.is_some());
        let intersection = intersection.unwrap();
        assert_global_point_eq(&intersection.get_start(), Vec3d::new(7.0, -2.0, 0.0));
        assert_global_point_eq(&intersection.get_end(), Vec3d::new(17.0, -2.0, 0.0));
        assert_eq!(secant_points.len(), 2);
        assert_vec_eq(secant_points[0], Vec3d::new(-2.0, 0.0, 0.0));
        assert_vec_eq(secant_points[1], Vec3d::new(2.0, 0.0, 0.0));
        assert_eq!(tangent_points.len(), 2);
        assert_vec_eq(tangent_points[0], Vec3d::new(2.0, 0.0, 0.0));
        assert_vec_eq(tangent_points[1], Vec3d::new(2.0, 0.0, 0.0));
        assert!(no_points.is_empty());
    }

    #[test]
    fn coordinated_regioned_angle_and_polar_coordinates_cover_signed_conversion() {
        let global = global_coordinate_system();
        let rotated = AnonymizedCoordinateSystem::Direct(CoordinateSystem::new(
            Vec3d::new(0.0, 0.0, 0.0),
            Vec3d::new(0.0, 1.0, 0.0),
            Vec3d::new(-1.0, 0.0, 0.0),
        ));
        let line1 = CoordinatedLine::new(
            CoordinatedPoint::new(global.clone(), Vec3d::new(0.0, 0.0, 0.0)),
            CoordinatedPoint::new(global.clone(), Vec3d::new(2.0, 0.0, 0.0)),
        );
        let line2 = CoordinatedLine::new(
            CoordinatedPoint::new(global.clone(), Vec3d::new(0.0, 0.0, 0.0)),
            CoordinatedPoint::new(global.clone(), Vec3d::new(0.0, 2.0, 0.0)),
        );
        let angle =
            CoordinatedRegionedAngle::new(global.clone(), RegionedAngle::new(30.0, -180.0, 180.0));
        let converted_angle = angle.convert_to(rotated.clone());
        let polar = PolarCoordinates::new(2.0, converted_angle.clone());
        let converted_polar = PolarCoordinates::new(
            2.0,
            CoordinatedRegionedAngle::new(global.clone(), RegionedAngle::new(45.0, -180.0, 180.0)),
        )
        .convert_to(rotated.clone());
        let cartesian = polar.to_cartesian();
        let converted_cartesian = converted_polar.to_cartesian();
        let from_lines = CoordinatedRegionedAngle::new_from_lines(&line1, &line2, -180.0, 180.0);

        assert_float_eq(from_lines.get_angle_degrees(), 90.0);
        assert_float_eq(converted_angle.get_angle_degrees(), -60.0);
        assert_float_eq(converted_angle.get_min_degrees(), -180.0);
        assert_float_eq(converted_angle.get_max_degrees(), 180.0);
        assert_float_eq(
            converted_angle
                .get_coordinate_system()
                .get_angle_between(&rotated)
                .angle_degrees,
            0.0,
        );
        assert_float_eq(polar.get_radius(), 2.0);
        assert_float_eq(polar.get_angle().get_angle_degrees(), -60.0);
        assert_float_eq(
            polar
                .get_coordinate_system()
                .get_angle_between(&rotated)
                .angle_degrees,
            0.0,
        );
        assert_vec_eq(
            cartesian.get_local_point(),
            Vec3d::new(1.0, -1.732_050_807_568_877_2, 0.0),
        );
        assert_global_point_eq(&cartesian, Vec3d::new(1.732_050_807_568_877_2, 1.0, 0.0));
        assert_float_eq(converted_polar.get_angle().get_angle_degrees(), -45.0);
        assert_vec_eq(
            converted_cartesian.get_local_point(),
            Vec3d::new(std::f64::consts::SQRT_2, -std::f64::consts::SQRT_2, 0.0),
        );
        assert_global_point_eq(
            &converted_cartesian,
            Vec3d::new(std::f64::consts::SQRT_2, std::f64::consts::SQRT_2, 0.0),
        );
    }
}

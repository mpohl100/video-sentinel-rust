use rs_math3d::Vector;
use rs_math3d::{CrossProduct, FloatVector, Vec3d, Vector3};
use std::sync::{Arc, Mutex};

#[derive(Clone)]
pub struct Line {
    start: Vec3d,
    end: Vec3d,
}

impl PartialEq for Line {
    fn eq(&self, other: &Self) -> bool {
        let start_diff = self.start - other.start;
        let end_diff = self.end - other.end;
        start_diff.length() < 1e-6 && end_diff.length() < 1e-6
    }
}

impl Line {
    pub fn new(start: Vec3d, end: Vec3d) -> Self {
        Self { start, end }
    }

    pub fn intersects(&self, other: &Line) -> bool {
        let p = self.start;
        let r = self.end - self.start;
        let q = other.start;
        let s = other.end - other.start;

        let r_cross_s = Vector3::<f64>::cross(&r, &s);
        let q_minus_p = q - p;

        if r_cross_s.length() < 1e-6 {
            // Lines are parallel
            return false;
        }

        let t = Vector3::<f64>::cross(&q_minus_p, &s).length() / r_cross_s.length();
        let u = Vector3::<f64>::cross(&q_minus_p, &r).length() / r_cross_s.length();

        (0.0..=1.0).contains(&t) && (0.0..=1.0).contains(&u)
    }

    pub fn get_intersection_point(&self, other: &Line) -> Vec3d {
        let p = self.start;
        let r = self.end - self.start;
        let q = other.start;
        let s = other.end - other.start;

        let r_cross_s = Vector3::<f64>::cross(&r, &s);
        let q_minus_p = q - p;

        if r_cross_s.length() < 1e-6 {
            // Lines are parallel, return the midpoint of the overlapping segment as the intersection point
            return (self.start + self.end) / 2.0;
        }

        let t = Vector3::<f64>::cross(&q_minus_p, &s).length() / r_cross_s.length();
        p + r * t
    }

    pub fn angle_between(&self, other: &Line) -> RegionedAngle {
        let v1 = self.end - self.start;
        let v2 = other.end - other.start;
        // calculate the angle between v1 and v2 using the dot product
        let dot_product = Vector3::<f64>::dot(&v1, &v2);
        let v1_length = v1.length();
        let v2_length = v2.length();
        if v1_length < 1e-6 || v2_length < 1e-6 {
            return RegionedAngle {
                angle_degrees: 0.0,
                min_degrees: -180.0,
                max_degrees: 180.0,
            }; // avoid division by zero, treat zero-length vectors as having zero angle between them
        }
        let cos_angle = dot_product / (v1_length * v2_length);
        RegionedAngle {
            angle_degrees: cos_angle.acos().to_degrees(),
            min_degrees: -180.0,
            max_degrees: 180.0,
        }
    }
}

#[derive(Clone)]
pub struct RegionedAngle {
    pub angle_degrees: f64,
    pub min_degrees: f64,
    pub max_degrees: f64,
}

impl RegionedAngle {
    pub fn new(degrees: f64, min_degrees: f64, max_degrees: f64) -> Self {
        let mut regioned_angle = Self {
            angle_degrees: degrees,
            min_degrees,
            max_degrees,
        };
        regioned_angle.adjust();
        regioned_angle
    }

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

    pub fn new_from_lines(line1: Line, line2: Line, min_degrees: f64, max_degrees: f64) -> Self {
        let angle_radians = line1.angle_between(&line2).radians();
        let angle_degrees = angle_radians.to_degrees();
        let mut regioned_angle = Self::new(angle_degrees, min_degrees, max_degrees);
        regioned_angle.adjust();
        regioned_angle
    }

    pub fn radians(&self) -> f64 {
        self.angle_degrees.to_radians()
    }

    pub fn get_min_degrees(&self) -> f64 {
        self.min_degrees
    }

    pub fn get_max_degrees(&self) -> f64 {
        self.max_degrees
    }

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
}

#[derive(Clone, PartialEq)]
pub struct Rectangle {
    lines: Vec<Line>,
}

impl Rectangle {
    pub fn new(top_left: Vec3d, bottom_right: Vec3d) -> Self {
        let top_right = Vec3d::new(bottom_right.x, top_left.y, 0.0);
        let bottom_left = Vec3d::new(top_left.x, bottom_right.y, 0.0);
        let lines = vec![
            Line::new(top_left, top_right),
            Line::new(top_right, bottom_right),
            Line::new(bottom_right, bottom_left),
            Line::new(bottom_left, top_left),
        ];
        Self { lines }
    }

    pub fn new_from_lines(lines: Vec<Line>) -> Self {
        Self { lines }
    }

    pub fn get_area(&self) -> f64 {
        let width = (self.lines[0].end.x - self.lines[0].start.x).abs();
        let height = (self.lines[1].end.y - self.lines[1].start.y).abs();
        width * height
    }

    pub fn get_center(&self) -> Vec3d {
        let center_x = (self.lines[0].start.x + self.lines[0].end.x) / 2.0;
        let center_y = (self.lines[1].start.y + self.lines[1].end.y) / 2.0;
        Vec3d::new(center_x, center_y, 0.0)
    }

    pub fn get_top_left(&self) -> Vec3d {
        self.lines[0].start
    }

    pub fn get_bottom_right(&self) -> Vec3d {
        self.lines[1].end
    }

    pub fn get_width(&self) -> f64 {
        (self.lines[0].end.x - self.lines[0].start.x).abs()
    }

    pub fn get_height(&self) -> f64 {
        (self.lines[1].end.y - self.lines[1].start.y).abs()
    }

    pub fn intersects(&self, other: &Rectangle) -> bool {
        for line1 in &self.lines {
            for line2 in &other.lines {
                if line1.intersects(line2) {
                    return true;
                }
            }
        }
        false
    }
}

pub fn expand_rectangle(rectangle: &Rectangle, expansion: f64) -> Rectangle {
    let top_left = rectangle.get_top_left();
    let bottom_right = rectangle.get_bottom_right();
    let expanded_top_left = Vec3d::new(top_left.x - expansion, top_left.y - expansion, 0.0);
    let expanded_bottom_right =
        Vec3d::new(bottom_right.x + expansion, bottom_right.y + expansion, 0.0);
    Rectangle::new(expanded_top_left, expanded_bottom_right)
}

#[derive(Clone)]
pub struct Circle {
    center: Vec3d,
    radius: f64,
}

impl PartialEq for Circle {
    fn eq(&self, other: &Self) -> bool {
        let center_diff = self.center - other.center;
        (center_diff.length() < 1e-6) && (self.radius - other.radius).abs() < 1e-6
    }
}

impl Circle {
    pub fn new(center: Vec3d, radius: f64) -> Self {
        Self { center, radius }
    }

    pub fn intersects(&self, other: &Circle) -> bool {
        let center_diff = self.center - other.center;
        center_diff.length() < (self.radius + other.radius)
    }

    pub fn get_bounding_box(&self) -> Rectangle {
        let top_left = Vec3d::new(
            self.center.x - self.radius,
            self.center.y - self.radius,
            0.0,
        );
        let bottom_right = Vec3d::new(
            self.center.x + self.radius,
            self.center.y + self.radius,
            0.0,
        );
        Rectangle::new(top_left, bottom_right)
    }

    pub fn get_area(&self) -> f64 {
        std::f64::consts::PI * self.radius * self.radius
    }

    pub fn get_center(&self) -> Vec3d {
        self.center
    }

    pub fn get_radius(&self) -> f64 {
        self.radius
    }
}

struct CoordinateSystem {
    origin: Vec3d,
    x_axis: Vec3d,
    y_axis: Vec3d,
}

impl CoordinateSystem {
    pub fn new(origin: Vec3d, x_axis: Vec3d, y_axis: Vec3d) -> Self {
        Self {
            origin,
            x_axis,
            y_axis,
        }
    }

    pub fn rotate(&mut self, angle: RegionedAngle) {
        let angle_radians = angle.radians();
        let cos_angle = angle_radians.cos();
        let sin_angle = angle_radians.sin();
        let new_x_axis = Vec3d::new(
            self.x_axis.x * cos_angle - self.y_axis.x * sin_angle,
            self.x_axis.y * cos_angle - self.y_axis.y * sin_angle,
            0.0,
        );
        let new_y_axis = Vec3d::new(
            self.x_axis.x * sin_angle + self.y_axis.x * cos_angle,
            self.x_axis.y * sin_angle + self.y_axis.y * cos_angle,
            0.0,
        );
        self.x_axis = new_x_axis;
        self.y_axis = new_y_axis;
    }

    pub fn to_local(&self, point: CoordinatedPoint) -> CoordinatedPoint {
        let global_point = self.to_global(point.clone());
        let local_coordinates = self.to_local_coordinates(global_point);
        CoordinatedPoint::new(
            WrappedCoordinateSystem::new(self.origin, self.x_axis, self.y_axis),
            local_coordinates,
        )
    }

    pub fn convert_from_global(&self, point: Vec3d) -> CoordinatedPoint {
        let local_coordinates = self.to_local_coordinates(point);
        CoordinatedPoint::new(
            WrappedCoordinateSystem::new(self.origin, self.x_axis, self.y_axis),
            local_coordinates,
        )
    }

    pub fn to_global(&self, point: CoordinatedPoint) -> Vec3d {
        let local = point.local_coordinates;
        self.origin + self.x_axis * local.x + self.y_axis * local.y
    }

    fn to_local_coordinates(&self, global: Vec3d) -> Vec3d {
        let relative = global - self.origin;
        let x = Vector3::<f64>::dot(&relative, &self.x_axis)
            / Vector3::<f64>::dot(&self.x_axis, &self.x_axis);
        let y = Vector3::<f64>::dot(&relative, &self.y_axis)
            / Vector3::<f64>::dot(&self.y_axis, &self.y_axis);
        Vec3d::new(x, y, 0.0)
    }
}

#[derive(Clone)]
pub struct WrappedCoordinateSystem {
    coordinate_system: Arc<Mutex<CoordinateSystem>>,
}

impl WrappedCoordinateSystem {
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

    pub fn to_local(&self, point: CoordinatedPoint) -> CoordinatedPoint {
        let cs = self.coordinate_system.lock().unwrap();
        cs.to_local(point)
    }

    pub fn from_global(&self, point: Vec3d) -> CoordinatedPoint {
        let cs = self.coordinate_system.lock().unwrap();
        cs.convert_from_global(point)
    }

    pub fn to_global(&self, point: CoordinatedPoint) -> Vec3d {
        let cs = self.coordinate_system.lock().unwrap();
        cs.to_global(point)
    }

    pub fn get_angle_between(&self, other: &WrappedCoordinateSystem) -> RegionedAngle {
        let cs1 = self.coordinate_system.lock().unwrap();
        let cs2 = other.coordinate_system.lock().unwrap();
        let self_x_axis = cs1.x_axis;
        let other_x_axis = cs2.x_axis;
        let line1 = Line::new(Vec3d::new(0.0, 0.0, 0.0), self_x_axis);
        let line2 = Line::new(Vec3d::new(0.0, 0.0, 0.0), other_x_axis);
        let angle_radians = line1.angle_between(&line2).radians();

        RegionedAngle::new(angle_radians.to_degrees(), -180.0, 180.0)
    }

    pub fn duplicate(&self) -> Self {
        let cs = self.coordinate_system.lock().unwrap();
        WrappedCoordinateSystem::new(cs.origin, cs.x_axis, cs.y_axis)
    }

    pub fn align_x_axis_with(&self, other: &WrappedCoordinateSystem) {
        let cs1 = self.coordinate_system.lock().unwrap();
        let cs2 = other.coordinate_system.lock().unwrap();
        let line1 = Line::new(Vec3d::new(0.0, 0.0, 0.0), cs1.x_axis);
        let line2 = Line::new(Vec3d::new(0.0, 0.0, 0.0), cs2.x_axis);
        let angle_to_rotate = line1.angle_between(&line2);
        drop(cs1);
        drop(cs2);
        self.rotate(angle_to_rotate);
    }
}

#[derive(Clone)]
pub struct CoordinatedPoint {
    wrapped_coordinate_system: WrappedCoordinateSystem,
    local_coordinates: Vec3d,
}

impl PartialEq for CoordinatedPoint {
    fn eq(&self, other: &Self) -> bool {
        let global_self = self.wrapped_coordinate_system.to_global(self.clone());
        let global_other = other.wrapped_coordinate_system.to_global(other.clone());
        let diff = global_self - global_other;
        diff.length() < 1e-6
    }
}

impl CoordinatedPoint {
    pub fn new(
        wrapped_coordinate_system: WrappedCoordinateSystem,
        local_coordinates: Vec3d,
    ) -> Self {
        Self {
            wrapped_coordinate_system,
            local_coordinates,
        }
    }

    pub fn convert_to(
        &self,
        wrapped_coordinate_system: WrappedCoordinateSystem,
    ) -> CoordinatedPoint {
        let global_point = self.wrapped_coordinate_system.to_global(self.clone());
        wrapped_coordinate_system.from_global(global_point)
    }

    pub fn plus(&self, other: Vec3d) -> CoordinatedPoint {
        let new_local = self.local_coordinates + other;
        CoordinatedPoint::new(self.wrapped_coordinate_system.clone(), new_local)
    }

    pub fn rotate(&self, around: CoordinatedPoint, angle: RegionedAngle) -> CoordinatedPoint {
        let around_local = around.convert_to(self.wrapped_coordinate_system.clone());
        let translated_x = self.local_coordinates.x - around_local.local_coordinates.x;
        let translated_y = self.local_coordinates.y - around_local.local_coordinates.y;
        let cos_angle = angle.radians().cos();
        let sin_angle = angle.radians().sin();
        let rotated_x = translated_x * cos_angle - translated_y * sin_angle;
        let rotated_y = translated_x * sin_angle + translated_y * cos_angle;
        let new_local = Vec3d::new(
            rotated_x + around_local.local_coordinates.x,
            rotated_y + around_local.local_coordinates.y,
            0.0,
        );
        CoordinatedPoint::new(self.wrapped_coordinate_system.clone(), new_local)
    }

    pub fn distance_to(&self, other: CoordinatedPoint) -> f64 {
        let global_self = self.wrapped_coordinate_system.to_global(self.clone());
        let global_other = other.wrapped_coordinate_system.to_global(other.clone());
        (global_self - global_other).length()
    }

    pub fn get_x(&self) -> f64 {
        self.local_coordinates.x
    }

    pub fn get_y(&self) -> f64 {
        self.local_coordinates.y
    }

    pub fn get_z(&self) -> f64 {
        self.local_coordinates.z
    }

    pub fn get_local_point(&self) -> Vec3d {
        self.local_coordinates
    }
}

#[derive(Clone)]
pub struct CoordinatedLine {
    start: CoordinatedPoint,
    end: CoordinatedPoint,
}

impl CoordinatedLine {
    pub fn new(start: CoordinatedPoint, end: CoordinatedPoint) -> Self {
        Self { start, end }
    }

    pub fn length(&self) -> f64 {
        self.start.distance_to(self.end.clone())
    }

    pub fn convert_to(
        &self,
        wrapped_coordinate_system: WrappedCoordinateSystem,
    ) -> CoordinatedLine {
        CoordinatedLine {
            start: self.start.convert_to(wrapped_coordinate_system.clone()),
            end: self.end.convert_to(wrapped_coordinate_system),
        }
    }

    pub fn to_global_line(&self) -> Line {
        let global_start = self
            .start
            .wrapped_coordinate_system
            .to_global(self.start.clone());
        let global_end = self
            .end
            .wrapped_coordinate_system
            .to_global(self.end.clone());
        Line::new(global_start, global_end)
    }

    pub fn get_intersection_point(&self, other: CoordinatedLine) -> Option<CoordinatedPoint> {
        let global_line1 = self.to_global_line();
        let global_line2 = other.to_global_line();
        if global_line1.intersects(&global_line2) {
            // For simplicity, we will return the midpoint of the intersection as the intersection point
            let global_intersection_point = global_line1.get_intersection_point(&global_line2);
            let target_coordinate_system = self.start.wrapped_coordinate_system.clone();
            let intersection_point =
                target_coordinate_system.from_global(global_intersection_point);
            Some(intersection_point)
        } else {
            None
        }
    }

    pub fn intersects(&self, other: CoordinatedLine) -> bool {
        let global_line1 = self.to_global_line();
        let global_line2 = other.to_global_line();
        global_line1.intersects(&global_line2)
    }

    pub fn get_start(&self) -> CoordinatedPoint {
        self.start.clone()
    }

    pub fn get_end(&self) -> CoordinatedPoint {
        self.end.clone()
    }
}

#[derive(Clone)]
pub struct CoordinatedRectangle {
    lines: Vec<CoordinatedLine>,
}

impl CoordinatedRectangle {
    pub fn new(top_left: CoordinatedPoint, bottom_right: CoordinatedPoint) -> Self {
        let top_right = CoordinatedPoint::new(
            top_left.wrapped_coordinate_system.clone(),
            Vec3d::new(
                bottom_right.local_coordinates.x,
                top_left.local_coordinates.y,
                0.0,
            ),
        );
        let bottom_left = CoordinatedPoint::new(
            top_left.wrapped_coordinate_system.clone(),
            Vec3d::new(
                top_left.local_coordinates.x,
                bottom_right.local_coordinates.y,
                0.0,
            ),
        );
        let lines = vec![
            CoordinatedLine::new(top_left.clone(), top_right.clone()),
            CoordinatedLine::new(top_right.clone(), bottom_right.clone()),
            CoordinatedLine::new(bottom_right.clone(), bottom_left.clone()),
            CoordinatedLine::new(bottom_left.clone(), top_left.clone()),
        ];
        Self { lines }
    }

    pub fn new_from_rectangle(
        rectangle: Rectangle,
        wrapped_coordinate_system: WrappedCoordinateSystem,
    ) -> Self {
        let lines = rectangle
            .lines
            .iter()
            .map(|line| {
                let start = wrapped_coordinate_system.from_global(line.start);
                let end = wrapped_coordinate_system.from_global(line.end);
                CoordinatedLine::new(start, end)
            })
            .collect();
        Self { lines }
    }

    pub fn convert_to(
        &self,
        wrapped_coordinate_system: WrappedCoordinateSystem,
    ) -> CoordinatedRectangle {
        CoordinatedRectangle {
            lines: self
                .lines
                .iter()
                .map(|line| line.convert_to(wrapped_coordinate_system.clone()))
                .collect(),
        }
    }

    pub fn to_global_rectangle(&self) -> Rectangle {
        let global_lines: Vec<Line> = self
            .lines
            .iter()
            .map(|line| line.to_global_line())
            .collect();
        Rectangle::new_from_lines(global_lines)
    }

    pub fn intersects(&self, other: &CoordinatedRectangle) -> bool {
        let global_rectangle1 = self.to_global_rectangle();
        let global_rectangle2 = other.to_global_rectangle();
        global_rectangle1.intersects(&global_rectangle2)
    }

    pub fn get_intersection_line(&self, line: CoordinatedLine) -> Option<CoordinatedLine> {
        let global_rectangle = self.to_global_rectangle();
        let global_line = line.to_global_line();
        let mut intersection_points = Vec::new();
        for rect_line in &global_rectangle.lines {
            if rect_line.intersects(&global_line) {
                let global_intersection_point = rect_line.get_intersection_point(&global_line);
                let target_coordinate_system = line.start.wrapped_coordinate_system.clone();
                let intersection_point =
                    target_coordinate_system.from_global(global_intersection_point);
                intersection_points.push(intersection_point);
            }
        }
        assert!(intersection_points.len() <= 2);
        if intersection_points.len() == 2 {
            Some(CoordinatedLine::new(
                intersection_points[0].clone(),
                intersection_points[1].clone(),
            ))
        } else {
            None
        }
    }
}

#[derive(Clone)]
pub struct CoordinatedCircle {
    center: CoordinatedPoint,
    radius: f64,
}

impl CoordinatedCircle {
    pub fn new(center: CoordinatedPoint, radius: f64) -> Self {
        Self { center, radius }
    }

    pub fn convert_to(
        &self,
        wrapped_coordinate_system: WrappedCoordinateSystem,
    ) -> CoordinatedCircle {
        CoordinatedCircle {
            center: self.center.convert_to(wrapped_coordinate_system.clone()),
            radius: self.radius,
        }
    }

    pub fn to_global_circle(&self) -> Circle {
        let global_center = self
            .center
            .wrapped_coordinate_system
            .to_global(self.center.clone());
        Circle::new(global_center, self.radius)
    }

    pub fn intersects(&self, other: &CoordinatedCircle) -> bool {
        let global_circle1 = self.to_global_circle();
        let global_circle2 = other.to_global_circle();
        global_circle1.intersects(&global_circle2)
    }

    pub fn get_center(&self) -> CoordinatedPoint {
        self.center.clone()
    }

    pub fn get_radius(&self) -> f64 {
        self.radius
    }

    pub fn get_bounding_box(&self) -> CoordinatedRectangle {
        let top_left = CoordinatedPoint::new(
            self.center.wrapped_coordinate_system.clone(),
            Vec3d::new(
                self.center.local_coordinates.x - self.radius,
                self.center.local_coordinates.y - self.radius,
                0.0,
            ),
        );
        let bottom_right = CoordinatedPoint::new(
            self.center.wrapped_coordinate_system.clone(),
            Vec3d::new(
                self.center.local_coordinates.x + self.radius,
                self.center.local_coordinates.y + self.radius,
                0.0,
            ),
        );
        CoordinatedRectangle::new(top_left, bottom_right)
    }

    pub fn get_area(&self) -> f64 {
        std::f64::consts::PI * self.radius * self.radius
    }

    pub fn contains_point(&self, point: CoordinatedPoint) -> bool {
        let global_center = self
            .center
            .wrapped_coordinate_system
            .to_global(self.center.clone());
        let global_point = self
            .center
            .wrapped_coordinate_system
            .to_global(point.clone());
        (global_center - global_point).length() <= self.radius
    }

    pub fn get_intersection_line(&self, line: CoordinatedLine) -> Option<CoordinatedLine> {
        let global_circle = self.to_global_circle();
        let global_line = line.to_global_line();
        let intersection_points = get_circle_line_intersection_points(&global_circle, &global_line);
        if intersection_points.len() == 2 {
            let target_coordinate_system = line.start.wrapped_coordinate_system.clone();
            Some(CoordinatedLine::new(
                target_coordinate_system.from_global(intersection_points[0]),
                target_coordinate_system.from_global(intersection_points[1]),
            ))
        } else {
            None
        }
    }
}

fn get_circle_line_intersection_points(circle: &Circle, line: &Line) -> Vec<Vec3d> {
    let d = line.end - line.start;
    let f = line.start - circle.center;

    let a = Vector3::<f64>::dot(&d, &d);
    let b = 2.0 * Vector3::<f64>::dot(&f, &d);
    let c = Vector3::<f64>::dot(&f, &f) - circle.radius * circle.radius;

    let discriminant = b * b - 4.0 * a * c;
    if discriminant < 0.0 {
        vec![] // No intersection
    } else {
        let sqrt_discriminant = discriminant.sqrt();
        let t1 = (-b - sqrt_discriminant) / (2.0 * a);
        let t2 = (-b + sqrt_discriminant) / (2.0 * a);
        let mut points = Vec::new();
        if (0.0..=1.0).contains(&t1) {
            points.push(line.start + d * t1);
        }
        if (0.0..=1.0).contains(&t2) {
            points.push(line.start + d * t2);
        }
        points
    }
}

pub struct CoordinatedRegionedAngle {
    wrapped_coordinate_system: WrappedCoordinateSystem,
    regioned_angle: RegionedAngle,
}

impl CoordinatedRegionedAngle {
    pub fn new(
        wrapped_coordinate_system: WrappedCoordinateSystem,
        regioned_angle: RegionedAngle,
    ) -> Self {
        Self {
            wrapped_coordinate_system,
            regioned_angle,
        }
    }

    pub fn new_from_lines(
        line1: CoordinatedLine,
        line2: CoordinatedLine,
        min_degrees: f64,
        max_degrees: f64,
    ) -> Self {
        let global_line1 = line1.to_global_line();
        let global_line2 = line2.to_global_line();
        let angle_radians = global_line1.angle_between(&global_line2).radians();
        let angle_degrees = angle_radians.to_degrees();
        let regioned_angle = RegionedAngle::new(angle_degrees, min_degrees, max_degrees);
        CoordinatedRegionedAngle::new(
            line1.start.wrapped_coordinate_system.clone(),
            regioned_angle,
        )
    }

    pub fn convert_to(
        &self,
        wrapped_coordinate_system: WrappedCoordinateSystem,
    ) -> CoordinatedRegionedAngle {
        let angle_between_coordinate_systems = self
            .wrapped_coordinate_system
            .get_angle_between(&wrapped_coordinate_system);
        let new_regioned_angle = self
            .regioned_angle
            .add_angle(angle_between_coordinate_systems);
        CoordinatedRegionedAngle::new(wrapped_coordinate_system, new_regioned_angle)
    }

    pub fn get_regioned_angle(&self) -> RegionedAngle {
        self.regioned_angle.clone()
    }

    pub fn get_coordinate_system(&self) -> WrappedCoordinateSystem {
        self.wrapped_coordinate_system.clone()
    }

    pub fn get_angle_degrees(&self) -> f64 {
        self.regioned_angle.angle_degrees
    }

    pub fn get_min_degrees(&self) -> f64 {
        self.regioned_angle.min_degrees
    }

    pub fn get_max_degrees(&self) -> f64 {
        self.regioned_angle.max_degrees
    }
}

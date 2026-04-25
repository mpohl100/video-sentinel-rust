use rs_math3d::{CrossProduct, FloatVector, Vec3d, Vector3};
use rs_math3d::Vector;
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

        t >= 0.0 && t <= 1.0 && u >= 0.0 && u <= 1.0
    }

    pub fn angle_between(&self, other: &Line) -> RegionedAngle {
        let v1 = self.end - self.start;
        let v2 = other.end - other.start;
        // calculate the angle between v1 and v2 using the dot product
        let dot_product = Vector3::<f64>::dot(&v1, &v2);
        let v1_length = v1.length();
        let v2_length = v2.length();
        if v1_length < 1e-6 || v2_length < 1e-6 {
            return RegionedAngle { angle_degrees: 0.0, min_degrees: -180.0, max_degrees: 180.0 }; // avoid division by zero, treat zero-length vectors as having zero angle between them
        }
        let cos_angle = dot_product / (v1_length * v2_length);
        RegionedAngle { angle_degrees: cos_angle.acos().to_degrees(), min_degrees: -180.0, max_degrees: 180.0 }
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
        let angle_radians = start_to_mid_line.angle_between(&start_to_end_line).radians();
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
    lines: Vec<Line>
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
    let expanded_bottom_right = Vec3d::new(bottom_right.x + expansion, bottom_right.y + expansion, 0.0);
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

impl Circle{
    pub fn new(center: Vec3d, radius: f64) -> Self {
        Self { center, radius }
    }

    pub fn intersects(&self, other: &Circle) -> bool {
        let center_diff = self.center - other.center;
        center_diff.length() < (self.radius + other.radius)
    }

    pub fn get_bounding_box(&self) -> Rectangle {
        let top_left = Vec3d::new(self.center.x - self.radius, self.center.y - self.radius, 0.0);
        let bottom_right = Vec3d::new(self.center.x + self.radius, self.center.y + self.radius, 0.0);
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
        Self { origin, x_axis, y_axis }
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
        CoordinatedPoint::new(WrappedCoordinateSystem::new(self.origin, self.x_axis, self.y_axis), local_coordinates)
    }

    pub fn from_global(&self, point: Vec3d) -> CoordinatedPoint {
        let local_coordinates = self.to_local_coordinates(point);
        CoordinatedPoint::new(WrappedCoordinateSystem::new(self.origin, self.x_axis, self.y_axis), local_coordinates)
    }

    pub fn to_global(&self, point: CoordinatedPoint) -> Vec3d {
        let local = point.local_coordinates;
        self.origin + self.x_axis * local.x + self.y_axis * local.y
    }

    fn to_local_coordinates(&self, global: Vec3d) -> Vec3d {
        let relative = global - self.origin;
        let x = Vector3::<f64>::dot(&relative, &self.x_axis) / Vector3::<f64>::dot(&self.x_axis, &self.x_axis);
        let y = Vector3::<f64>::dot(&relative, &self.y_axis) / Vector3::<f64>::dot(&self.y_axis, &self.y_axis);
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
        cs.from_global(point)
    }

    pub fn to_global(&self, point: CoordinatedPoint) -> Vec3d {
        let cs = self.coordinate_system.lock().unwrap();
        cs.to_global(point)
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
    pub fn new(wrapped_coordinate_system: WrappedCoordinateSystem, local_coordinates: Vec3d) -> Self {
        Self {
            wrapped_coordinate_system,
            local_coordinates,
        }
    }

    pub fn convert_to(&self, wrapped_coordinate_system: WrappedCoordinateSystem) -> CoordinatedPoint {
        let global_point = self.wrapped_coordinate_system.to_global(self.clone());
        wrapped_coordinate_system.from_global(global_point)
    }

    pub fn plus(&self, other: Vec3d) -> CoordinatedPoint {
        let new_local = self.local_coordinates + other;
        CoordinatedPoint::new(self.wrapped_coordinate_system.clone(), new_local)
    }

    pub fn rotate(&self,around: CoordinatedPoint, angle: RegionedAngle) -> CoordinatedPoint {
        let around_local = around.convert_to(self.wrapped_coordinate_system.clone());
        let translated_x = self.local_coordinates.x - around_local.local_coordinates.x;
        let translated_y = self.local_coordinates.y - around_local.local_coordinates.y;
        let cos_angle = angle.radians().cos();
        let sin_angle = angle.radians().sin();
        let rotated_x = translated_x * cos_angle - translated_y * sin_angle;
        let rotated_y = translated_x * sin_angle + translated_y * cos_angle;
        let new_local = Vec3d::new(rotated_x + around_local.local_coordinates.x, rotated_y + around_local.local_coordinates.y, 0.0);
        CoordinatedPoint::new(self.wrapped_coordinate_system.clone(), new_local)
    }

    pub fn distance_to(&self, other: CoordinatedPoint) -> f64 {
        let global_self = self.wrapped_coordinate_system.to_global(self.clone());
        let global_other = other.wrapped_coordinate_system.to_global(other.clone());
        (global_self - global_other).length()
    }
}



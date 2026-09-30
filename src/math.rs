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

        let t = Vector3::<f64>::cross(&q_minus_p, &s).z / r_cross_s.z;
        let u = Vector3::<f64>::cross(&q_minus_p, &r).z / r_cross_s.z;

        (0.0..=1.0).contains(&t) && (0.0..=1.0).contains(&u)
    }

    pub fn get_intersection_point(&self, other: &Line) -> Point {
        let p = self.start;
        let r = self.end - self.start;
        let q = other.start;
        let s = other.end - other.start;

        let r_cross_s = Vector3::<f64>::cross(&r, &s);
        let q_minus_p = q - p;

        if r_cross_s.length() < 1e-6 {
            // Lines are parallel, return the midpoint of the overlapping segment as the intersection point
            return Point::new((self.start + self.end) / 2.0);
        }

        let t = Vector3::<f64>::cross(&q_minus_p, &s).z / r_cross_s.z;
        Point::new(p + r * t)
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

    pub fn get_start(&self) -> Point {
        Point::new(self.start)
    }

    pub fn get_end(&self) -> Point {
        Point::new(self.end)
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

    pub fn inverted(&self) -> Self {
        Self::new(-self.angle_degrees, self.min_degrees, self.max_degrees)
    }
}

#[derive(Clone)]
pub struct Point {
    pub point: Vec3d,
}

impl PartialEq for Point {
    fn eq(&self, other: &Self) -> bool {
        let epsilon = 1e-6;
        (self.point.x - other.point.x).abs() < epsilon
            && (self.point.y - other.point.y).abs() < epsilon
            && (self.point.z - other.point.z).abs() < epsilon
    }
}

impl Point {
    pub fn new(point: Vec3d) -> Self {
        Self { point }
    }

    pub fn get_x(&self) -> f64 {
        self.point.x
    }

    pub fn get_y(&self) -> f64 {
        self.point.y
    }
}

#[derive(Clone, PartialEq)]
pub struct Rectangle {
    points: Vec<Point>,
}

impl Rectangle {
    pub fn new(top_left: Vec3d, bottom_right: Vec3d) -> Self {
        let points = vec![
            Point { point: top_left },
            Point {
                point: bottom_right,
            },
        ];
        Self { points }
    }


    pub fn get_intersection_line(&self, line: Line) -> Option<Line> {
        let mut intersection_points = Vec::new();
        for rect_line in &self.get_lines() {
            if rect_line.intersects(&line) {
                let intersection_point = rect_line.get_intersection_point(&line);
                intersection_points.push(intersection_point);
            }
        }
        // make sure the intersection points are unique
        intersection_points.sort_by(|a, b| {
            a.get_x()
                .partial_cmp(&b.get_x())
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        intersection_points.dedup_by(|a, b| {
            (a.get_x() - b.get_x()).abs() < 1e-8 && (a.get_y() - b.get_y()).abs() < 1e-8
        });
        assert!(intersection_points.len() <= 2);
        if intersection_points.len() == 2 {
            Some(Line::new(
                intersection_points[0].point,
                intersection_points[1].point,
            ))
        } else if intersection_points.len() == 1 {
            // return the one line of [input_line.begin, intersection_point] or [input_line.end, intersection_point] depending on which one is inside the coordinated rectangle
            let input_line_begin = line.start.clone();
            let input_line_end = line.end.clone();
            let intersection_point = intersection_points[0].clone();
            if self.contains_point(input_line_begin.clone()) {
                Some(Line::new(input_line_begin, intersection_point.point))
            } else if self.contains_point(input_line_end.clone()) {
                Some(Line::new(input_line_end, intersection_point.point))
            } else {
                None
            }
        } else {
            None
        }
    }

    pub fn contains_point(&self, point: Vec3d) -> bool {
        let top_left = self.get_top_left();
        let bottom_right = self.get_bottom_right();
        !(point.x < top_left.x
            || point.x > bottom_right.x
            || point.y < top_left.y
            || point.y > bottom_right.y)
    }

    pub fn get_area(&self) -> f64 {
        let width = (self.points[1].point.x - self.points[0].point.x).abs();
        let height = (self.points[1].point.y - self.points[0].point.y).abs();
        width * height
    }

    pub fn get_center(&self) -> Vec3d {
        let center_x = (self.points[0].point.x + self.points[1].point.x) / 2.0;
        let center_y = (self.points[0].point.y + self.points[1].point.y) / 2.0;
        Vec3d::new(center_x, center_y, 0.0)
    }

    pub fn get_top_left(&self) -> Vec3d {
        self.points[0].point
    }

    pub fn get_bottom_right(&self) -> Vec3d {
        self.points[1].point
    }

    pub fn get_width(&self) -> f64 {
        (self.points[1].point.x - self.points[0].point.x).abs()
    }

    pub fn get_height(&self) -> f64 {
        (self.points[1].point.y - self.points[0].point.y).abs()
    }

    pub fn get_lines(&self) -> Vec<Line> {
        let top_left = self.get_top_left();
        let bottom_right = self.get_bottom_right();
        let top_right = Vec3d::new(bottom_right.x, top_left.y, 0.0);
        let bottom_left = Vec3d::new(top_left.x, bottom_right.y, 0.0);

        let line1 = Line::new(top_left, top_right);
        let line2 = Line::new(top_right, bottom_right);
        let line3 = Line::new(bottom_right, bottom_left);
        let line4 = Line::new(bottom_left, top_left);
        vec![line1, line2, line3, line4]
    }

    pub fn intersects(&self, other: &Rectangle) -> bool {
        for line1 in &self.get_lines() {
            for line2 in &other.get_lines() {
                if line1.intersects(line2) {
                    return true;
                }
            }
        }
        false
    }

    pub fn overlaps(&self, other: &Rectangle) -> bool {
        let self_top_left = self.get_top_left();
        let self_bottom_right = self.get_bottom_right();
        let other_top_left = other.get_top_left();
        let other_bottom_right = other.get_bottom_right();

        !(self_bottom_right.x < other_top_left.x
            || self_top_left.x > other_bottom_right.x
            || self_bottom_right.y < other_top_left.y
            || self_top_left.y > other_bottom_right.y)
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

#[derive(Clone)]
pub struct CoordinateSystem {
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
        let global_point = point.coordinate_system.to_global(point.clone());
        let local_coordinates = self.to_local_coordinates(global_point.get_local_point());
        CoordinatedPoint::new(
            AnonymizedCoordinateSystem::Direct(self.clone()),
            local_coordinates,
        )
    }

    pub fn convert_from_global(&self, point: Vec3d) -> CoordinatedPoint {
        let local_coordinates = self.to_local_coordinates(point);
        CoordinatedPoint::new(
            AnonymizedCoordinateSystem::Direct(self.clone()),
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

    pub fn get_angle_between(&self, other: &AnonymizedCoordinateSystem) -> RegionedAngle {
        let self_x_axis = self.x_axis;
        let other_x_axis = match other {
            AnonymizedCoordinateSystem::Direct(cs) => cs.x_axis,
            AnonymizedCoordinateSystem::Indirect(wrapped_cs) => {
                wrapped_cs.coordinate_system.lock().unwrap().x_axis
            }
        };
        let dot = Vector3::<f64>::dot(&self_x_axis, &other_x_axis);
        let cross = Vector3::<f64>::cross(&self_x_axis, &other_x_axis);

        RegionedAngle::new(cross.z.atan2(dot).to_degrees(), -180.0, 180.0)
    }

    pub fn align_x_axis_with(&mut self, other: &AnonymizedCoordinateSystem) {
        let angle_between = self.get_angle_between(other);
        let angle_to_rotate = RegionedAngle::new(
            -angle_between.angle_degrees,
            angle_between.min_degrees,
            angle_between.max_degrees,
        );
        self.rotate(angle_to_rotate);
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

    pub fn get_angle_between(&self, other: &AnonymizedCoordinateSystem) -> RegionedAngle {
        let CoordinateSystem { x_axis, .. } = &self.coordinate_system.lock().unwrap().clone();
        let self_x_axis = *x_axis;
        let other_x_axis = match other {
            AnonymizedCoordinateSystem::Direct(cs) => cs.x_axis,
            AnonymizedCoordinateSystem::Indirect(wrapped_cs) => {
                wrapped_cs.coordinate_system.lock().unwrap().x_axis
            }
        };
        let dot = Vector3::<f64>::dot(&self_x_axis, &other_x_axis);
        let cross = Vector3::<f64>::cross(&self_x_axis, &other_x_axis);

        RegionedAngle::new(cross.z.atan2(dot).to_degrees(), -180.0, 180.0)
    }

    pub fn duplicate(&self) -> Self {
        let cs = self.coordinate_system.lock().unwrap();
        WrappedCoordinateSystem::new(cs.origin, cs.x_axis, cs.y_axis)
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

#[derive(Clone)]
pub enum AnonymizedCoordinateSystem {
    Direct(CoordinateSystem),
    Indirect(WrappedCoordinateSystem),
}

impl AnonymizedCoordinateSystem {
    pub fn new_direct(coordinate_system: CoordinateSystem) -> Self {
        AnonymizedCoordinateSystem::Direct(coordinate_system)
    }

    pub fn new_indirect(coordinate_system: CoordinateSystem) -> Self {
        AnonymizedCoordinateSystem::Indirect(WrappedCoordinateSystem::new(
            coordinate_system.origin,
            coordinate_system.x_axis,
            coordinate_system.y_axis,
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

    pub fn duplicate(&self) -> Self {
        match self {
            AnonymizedCoordinateSystem::Direct(cs) => {
                AnonymizedCoordinateSystem::Direct(cs.clone())
            }
            AnonymizedCoordinateSystem::Indirect(wrapped_cs) => {
                AnonymizedCoordinateSystem::Indirect(wrapped_cs.duplicate())
            }
        }
    }

    pub fn to_global(&self, point: CoordinatedPoint) -> CoordinatedPoint {
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

    pub fn to_local(&self, global_point: CoordinatedPoint) -> CoordinatedPoint {
        match self {
            AnonymizedCoordinateSystem::Direct(cs) => cs.to_local(global_point),
            AnonymizedCoordinateSystem::Indirect(wrapped_cs) => wrapped_cs.to_local(global_point),
        }
    }

    pub fn from_global(&self, global_point: CoordinatedPoint) -> CoordinatedPoint {
        let global_coordinates = global_point.get_local_point();
        match self {
            AnonymizedCoordinateSystem::Direct(cs) => cs.convert_from_global(global_coordinates),
            AnonymizedCoordinateSystem::Indirect(wrapped_cs) => {
                wrapped_cs.from_global(global_coordinates)
            }
        }
    }

    pub fn get_angle_between(&self, other: AnonymizedCoordinateSystem) -> RegionedAngle {
        match self {
            AnonymizedCoordinateSystem::Direct(cs) => cs.get_angle_between(&other),
            AnonymizedCoordinateSystem::Indirect(wrapped_cs) => {
                wrapped_cs.get_angle_between(&other)
            }
        }
    }

    pub fn align_x_axis_with(&mut self, other: AnonymizedCoordinateSystem) {
        match self {
            AnonymizedCoordinateSystem::Direct(cs) => cs.align_x_axis_with(&other),
            AnonymizedCoordinateSystem::Indirect(wrapped_cs) => {
                wrapped_cs.align_x_axis_with(&other)
            }
        }
    }
}

#[derive(Clone)]
pub struct CoordinatedPoint {
    coordinate_system: AnonymizedCoordinateSystem,
    local_coordinates: Vec3d,
}

impl PartialEq for CoordinatedPoint {
    fn eq(&self, other: &Self) -> bool {
        let global_self = self.coordinate_system.to_global(self.clone());
        let global_other = other.coordinate_system.to_global(other.clone());
        let diff = global_self.get_local_point() - global_other.get_local_point();
        diff.length() < 1e-6
    }
}

impl CoordinatedPoint {
    pub fn new(coordinate_system: AnonymizedCoordinateSystem, local_coordinates: Vec3d) -> Self {
        Self {
            coordinate_system,
            local_coordinates,
        }
    }

    pub fn get_coordinate_system(&self) -> AnonymizedCoordinateSystem {
        self.coordinate_system.clone()
    }

    pub fn to_global_point(&self) -> CoordinatedPoint {
        let global_coordinate_system =
            AnonymizedCoordinateSystem::new_direct(CoordinateSystem::new(
                Vec3d::new(0.0, 0.0, 0.0),
                Vec3d::new(1.0, 0.0, 0.0),
                Vec3d::new(0.0, 1.0, 0.0),
            ));
        self.convert_to(global_coordinate_system)
    }

    pub fn get_local_point(&self) -> Vec3d {
        self.local_coordinates
    }

    pub fn set_y(&mut self, y: f64) {
        self.local_coordinates.y = y;
    }

    pub fn set_x(&mut self, x: f64) {
        self.local_coordinates.x = x;
    }

    pub fn set_z(&mut self, z: f64) {
        self.local_coordinates.z = z;
    }

    pub fn convert_to(&self, coordinate_system: AnonymizedCoordinateSystem) -> CoordinatedPoint {
        let global_point = self
            .coordinate_system
            .to_global(self.clone())
            .get_local_point();
        let local_coordinates = coordinate_system
            .from_global(CoordinatedPoint::new(
                AnonymizedCoordinateSystem::new_direct(CoordinateSystem::new(
                    Vec3d::new(0.0, 0.0, 0.0),
                    Vec3d::new(1.0, 0.0, 0.0),
                    Vec3d::new(0.0, 1.0, 0.0),
                )),
                global_point,
            ))
            .get_local_point();
        CoordinatedPoint::new(coordinate_system, local_coordinates)
    }

    pub fn plus(&self, other: Vec3d) -> CoordinatedPoint {
        let new_local = self.local_coordinates + other;
        CoordinatedPoint::new(self.coordinate_system.clone(), new_local)
    }

    pub fn rotate(&self, around: CoordinatedPoint, angle: RegionedAngle) -> CoordinatedPoint {
        let around_local = around.convert_to(self.coordinate_system.clone());
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
        CoordinatedPoint::new(self.coordinate_system.clone(), new_local)
    }

    pub fn distance_to(&self, other: CoordinatedPoint) -> f64 {
        let global_self = self.coordinate_system.to_global(self.clone());
        let global_other = other.coordinate_system.to_global(other.clone());
        (global_self.get_local_point() - global_other.get_local_point()).length()
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

    pub fn convert_to(&self, coordinate_system: AnonymizedCoordinateSystem) -> CoordinatedLine {
        CoordinatedLine {
            start: self.start.convert_to(coordinate_system.clone()),
            end: self.end.convert_to(coordinate_system),
        }
    }

    pub fn get_line(&self) -> Line {
        self.to_global_line()
    }

    pub fn to_global_line(&self) -> Line {
        let global_start = self.start.coordinate_system.to_global(self.start.clone());
        let global_end = self.end.coordinate_system.to_global(self.end.clone());
        Line::new(global_start.get_local_point(), global_end.get_local_point())
    }

    pub fn get_intersection_point(&self, other: CoordinatedLine) -> Option<CoordinatedPoint> {
        let global_line1 = self.to_global_line();
        let global_line2 = other.to_global_line();
        let global_coordinate_system =
            AnonymizedCoordinateSystem::new_direct(CoordinateSystem::new(
                Vec3d::new(0.0, 0.0, 0.0),
                Vec3d::new(1.0, 0.0, 0.0),
                Vec3d::new(0.0, 1.0, 0.0),
            ));
        if global_line1.intersects(&global_line2) {
            // For simplicity, we will return the midpoint of the intersection as the intersection point
            let intersection_point = global_line1.get_intersection_point(&global_line2);
            let global_intersection_point =
                CoordinatedPoint::new(global_coordinate_system.clone(), intersection_point.point);
            let target_coordinate_system = self.start.coordinate_system.clone();
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
    points: Vec<CoordinatedPoint>,
}

impl CoordinatedRectangle {
    pub fn new(top_left: CoordinatedPoint, bottom_right: CoordinatedPoint) -> Self {
        let points = vec![top_left, bottom_right];
        Self { points }
    }

    pub fn new_from_rectangle(
        rectangle: Rectangle,
        coordinate_system: AnonymizedCoordinateSystem,
    ) -> Self {
        let global_coordinate_system =
            AnonymizedCoordinateSystem::new_direct(CoordinateSystem::new(
                Vec3d::new(0.0, 0.0, 0.0),
                Vec3d::new(1.0, 0.0, 0.0),
                Vec3d::new(0.0, 1.0, 0.0),
            ));
        let points = rectangle
            .points
            .iter()
            .map(|point| {
                let coordinated_point =
                    CoordinatedPoint::new(global_coordinate_system.clone(), point.point);
                coordinated_point.convert_to(coordinate_system.clone())
            })
            .collect();
        Self { points }
    }

    pub fn convert_to(
        &self,
        coordinate_system: AnonymizedCoordinateSystem,
    ) -> CoordinatedRectangle {
        CoordinatedRectangle {
            points: self
                .points
                .iter()
                .map(|point| point.convert_to(coordinate_system.clone()))
                .collect(),
        }
    }

    pub fn get_local_rectangle(&self) -> Rectangle {
        let tl = self.points[0].get_local_point();
        let br = self.points[1].get_local_point();
        Rectangle::new(tl, br)
    }

    pub fn to_global_rectangle(&self) -> Rectangle {
        let tl = self.points[0].to_global_point().get_local_point();
        let br = self.points[1].to_global_point().get_local_point();
        Rectangle::new(tl, br)
    }

    pub fn overlaps(&self, other: &CoordinatedRectangle) -> bool {
        let global_rectangle1 = self.to_global_rectangle();
        let global_rectangle2 = other.to_global_rectangle();
        global_rectangle1.overlaps(&global_rectangle2)
    }

    pub fn intersects(&self, other: &CoordinatedRectangle) -> bool {
        let global_rectangle1 = self.to_global_rectangle();
        let global_rectangle2 = other.to_global_rectangle();
        global_rectangle1.intersects(&global_rectangle2)
    }

    pub fn get_intersection_line(&self, line: CoordinatedLine) -> Option<CoordinatedLine> {
        let global_rectangle = self.to_global_rectangle();
        let global_line = line.to_global_line();
        let global_coordinate_system =
            AnonymizedCoordinateSystem::new_direct(CoordinateSystem::new(
                Vec3d::new(0.0, 0.0, 0.0),
                Vec3d::new(1.0, 0.0, 0.0),
                Vec3d::new(0.0, 1.0, 0.0),
            ));
        match global_rectangle.get_intersection_line(global_line) {
            Some(intersection_line) => {
                return Some(CoordinatedLine::new(
                    CoordinatedPoint::new(global_coordinate_system.clone(), intersection_line.start),
                    CoordinatedPoint::new(global_coordinate_system.clone(), intersection_line.end),
                ));
            }
            None => None,
        }
    }

    pub fn get_top_left(&self) -> CoordinatedPoint {
        self.points[0].clone()
    }

    pub fn get_top_right(&self) -> CoordinatedPoint {
        CoordinatedPoint::new(
            self.points[0].coordinate_system.clone(),
            Vec3d::new(
                self.points[1].local_coordinates.x,
                self.points[0].local_coordinates.y,
                0.0,
            ),
        )
    }

    pub fn get_bottom_left(&self) -> CoordinatedPoint {
        CoordinatedPoint::new(
            self.points[0].coordinate_system.clone(),
            Vec3d::new(
                self.points[0].local_coordinates.x,
                self.points[1].local_coordinates.y,
                0.0,
            ),
        )
    }

    pub fn get_bottom_right(&self) -> CoordinatedPoint {
        self.points[1].clone()
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
        wrapped_coordinate_system: AnonymizedCoordinateSystem,
    ) -> CoordinatedCircle {
        CoordinatedCircle {
            center: self.center.convert_to(wrapped_coordinate_system.clone()),
            radius: self.radius,
        }
    }

    pub fn to_global_circle(&self) -> Circle {
        let global_center = self.center.coordinate_system.to_global(self.center.clone());
        Circle::new(global_center.get_local_point(), self.radius)
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
            self.center.coordinate_system.clone(),
            Vec3d::new(
                self.center.local_coordinates.x - self.radius,
                self.center.local_coordinates.y - self.radius,
                0.0,
            ),
        );
        let bottom_right = CoordinatedPoint::new(
            self.center.coordinate_system.clone(),
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
        let global_center = self.center.coordinate_system.to_global(self.center.clone());
        let global_point = point.coordinate_system.to_global(point.clone());
        (global_center.get_local_point() - global_point.get_local_point()).length() <= self.radius
    }

    pub fn get_intersection_line(&self, line: CoordinatedLine) -> Option<CoordinatedLine> {
        let global_circle = self.to_global_circle();
        let global_line = line.to_global_line();
        let intersection_points_raw =
            get_circle_line_intersection_points(&global_circle, &global_line);
        let global_coordinate_system =
            AnonymizedCoordinateSystem::new_direct(CoordinateSystem::new(
                Vec3d::new(0.0, 0.0, 0.0),
                Vec3d::new(1.0, 0.0, 0.0),
                Vec3d::new(0.0, 1.0, 0.0),
            ));
        let intersection_points = intersection_points_raw
            .iter()
            .map(|p| CoordinatedPoint::new(global_coordinate_system.clone(), *p))
            .collect::<Vec<_>>();
        if intersection_points.len() == 2 {
            let target_coordinate_system = line.start.coordinate_system.clone();
            Some(CoordinatedLine::new(
                target_coordinate_system.from_global(intersection_points[0].clone()),
                target_coordinate_system.from_global(intersection_points[1].clone()),
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

#[derive(Clone)]
pub struct CoordinatedRegionedAngle {
    coordinate_system: AnonymizedCoordinateSystem,
    regioned_angle: RegionedAngle,
}

impl CoordinatedRegionedAngle {
    pub fn new(
        coordinate_system: AnonymizedCoordinateSystem,
        regioned_angle: RegionedAngle,
    ) -> Self {
        Self {
            coordinate_system,
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
        CoordinatedRegionedAngle::new(line1.start.coordinate_system.clone(), regioned_angle)
    }

    pub fn convert_to(
        &self,
        coordinate_system: AnonymizedCoordinateSystem,
    ) -> CoordinatedRegionedAngle {
        let angle_between_coordinate_systems = self
            .coordinate_system
            .get_angle_between(coordinate_system.clone());
        let new_regioned_angle = self.regioned_angle.add_angle(RegionedAngle::new(
            -angle_between_coordinate_systems.angle_degrees,
            self.regioned_angle.min_degrees,
            self.regioned_angle.max_degrees,
        ));
        CoordinatedRegionedAngle::new(coordinate_system, new_regioned_angle)
    }

    pub fn get_regioned_angle(&self) -> RegionedAngle {
        self.regioned_angle.clone()
    }

    pub fn get_coordinate_system(&self) -> AnonymizedCoordinateSystem {
        self.coordinate_system.clone()
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

#[derive(Clone)]
pub struct PolarCoordinates {
    radius: f64,
    angle: CoordinatedRegionedAngle,
}

impl PolarCoordinates {
    pub fn new(radius: f64, angle: CoordinatedRegionedAngle) -> Self {
        Self { radius, angle }
    }

    pub fn convert_to(&self, coordinate_system: AnonymizedCoordinateSystem) -> PolarCoordinates {
        let new_angle = self.angle.convert_to(coordinate_system);
        PolarCoordinates::new(self.radius, new_angle)
    }

    pub fn to_cartesian(&self) -> CoordinatedPoint {
        let angle_radians = self.angle.get_regioned_angle().radians();
        let x = self.radius * angle_radians.cos();
        let y = self.radius * angle_radians.sin();
        CoordinatedPoint::new(self.angle.get_coordinate_system(), Vec3d::new(x, y, 0.0))
    }

    pub fn get_radius(&self) -> f64 {
        self.radius
    }

    pub fn get_angle(&self) -> CoordinatedRegionedAngle {
        self.angle.clone()
    }

    pub fn get_coordinate_system(&self) -> AnonymizedCoordinateSystem {
        self.angle.get_coordinate_system()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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

    fn assert_global_point_eq(point: CoordinatedPoint, expected: Vec3d) {
        let global_point = point.convert_to(global_coordinate_system());
        assert_vec_eq(global_point.get_local_point(), expected);
    }

    #[test]
    fn line_methods_cover_equality_intersections_and_angles() {
        let line = Line::new(Vec3d::new(0.0, 0.0, 0.0), Vec3d::new(2.0, 2.0, 0.0));
        let almost_equal = Line::new(Vec3d::new(0.0, 0.0, 0.0), Vec3d::new(2.0, 2.0000005, 0.0));
        let different = Line::new(Vec3d::new(0.0, 0.0, 0.0), Vec3d::new(2.0, 2.01, 0.0));
        let crossing = Line::new(Vec3d::new(0.0, 2.0, 0.0), Vec3d::new(2.0, 0.0, 0.0));
        let parallel = Line::new(Vec3d::new(0.0, 1.0, 0.0), Vec3d::new(2.0, 1.0, 0.0));
        let horizontal = Line::new(Vec3d::new(0.0, 0.0, 0.0), Vec3d::new(2.0, 0.0, 0.0));
        let vertical = Line::new(Vec3d::new(1.0, -1.0, 0.0), Vec3d::new(1.0, 1.0, 0.0));
        let disjoint = Line::new(Vec3d::new(3.0, 0.0, 0.0), Vec3d::new(4.0, 0.0, 0.0));
        let zero_length = Line::new(Vec3d::new(1.0, 1.0, 0.0), Vec3d::new(1.0, 1.0, 0.0));

        assert_vec_eq(line.start, Vec3d::new(0.0, 0.0, 0.0));
        assert_vec_eq(line.end, Vec3d::new(2.0, 2.0, 0.0));
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
        assert_float_eq(pi_angle.radians(), 3.141592653589793);
        assert_float_eq(pi_angle.get_min_degrees(), -180.0);
        assert_float_eq(pi_angle.get_max_degrees(), 180.0);
        assert_float_eq(sum.angle_degrees, -150.0);
    }

    #[test]
    fn rectangle_and_expansion_methods_cover_dimensions_and_intersections() {
        let rectangle = Rectangle::new(Vec3d::new(1.0, 2.0, 0.0), Vec3d::new(5.0, 8.0, 0.0));
        let from_lines = Rectangle::new(rectangle.points[0].point, rectangle.points[1].point);
        let overlapping = Rectangle::new(Vec3d::new(4.0, 4.0, 0.0), Vec3d::new(7.0, 10.0, 0.0));
        let disjoint = Rectangle::new(Vec3d::new(6.0, 9.0, 0.0), Vec3d::new(8.0, 12.0, 0.0));
        let expanded = expand_rectangle(&rectangle, 1.5);

        assert_eq!(rectangle.points.len(), 2);
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
        let almost_equal = Circle::new(Vec3d::new(2.0, -1.0, 0.0), 2.0000005);
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
        assert_float_eq(circle.get_area(), 12.566370614359172);
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

        assert_vec_eq(translated.origin, Vec3d::new(10.0, -5.0, 0.0));
        assert_vec_eq(translated.x_axis, Vec3d::new(1.0, 0.0, 0.0));
        assert_vec_eq(translated.y_axis, Vec3d::new(0.0, 1.0, 0.0));
        assert_vec_eq(local_point.get_local_point(), Vec3d::new(2.0, 3.0, 0.0));
        assert_vec_eq(
            translated.to_global(CoordinatedPoint::new(
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
            translated.to_local(point_in_global).get_local_point(),
            Vec3d::new(2.0, 3.0, 0.0),
        );

        standard.rotate(RegionedAngle::new(90.0, -180.0, 180.0));

        assert_vec_eq(standard.x_axis, Vec3d::new(0.0, -1.0, 0.0));
        assert_vec_eq(standard.y_axis, Vec3d::new(1.0, 0.0, 0.0));
        assert_vec_eq(
            standard.to_global(CoordinatedPoint::new(
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
            translated.to_global(point.clone()).get_local_point(),
            Vec3d::new(12.0, -2.0, 0.0),
        );
        assert_vec_eq(
            translated
                .from_global(CoordinatedPoint::new(
                    global_coordinate_system(),
                    Vec3d::new(12.0, -2.0, 0.0),
                ))
                .get_local_point(),
            Vec3d::new(2.0, 3.0, 0.0),
        );
        assert_vec_eq(
            translated
                .to_local(CoordinatedPoint::new(
                    global.clone(),
                    Vec3d::new(12.0, -2.0, 0.0),
                ))
                .get_local_point(),
            Vec3d::new(2.0, 3.0, 0.0),
        );
        assert_float_eq(
            global.get_angle_between(rotated.clone()).angle_degrees,
            90.0,
        );
        assert_float_eq(
            rotated.get_angle_between(global.clone()).angle_degrees,
            -90.0,
        );

        global.rotate(RegionedAngle::new(90.0, -180.0, 180.0));

        assert_vec_eq(
            global
                .to_global(CoordinatedPoint::new(
                    global.clone(),
                    Vec3d::new(1.0, 0.0, 0.0),
                ))
                .get_local_point(),
            Vec3d::new(0.0, -1.0, 0.0),
        );
        assert_vec_eq(
            duplicate
                .to_global(CoordinatedPoint::new(
                    duplicate.clone(),
                    Vec3d::new(1.0, 0.0, 0.0),
                ))
                .get_local_point(),
            Vec3d::new(1.0, 0.0, 0.0),
        );

        alignable.align_x_axis_with(rotated.clone());

        assert_float_eq(alignable.get_angle_between(rotated).angle_degrees, 0.0);
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
                CoordinatedPoint::new(global_coordinate_system(), Vec3d::new(0.0, 0.0, 0.0)),
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
        assert_global_point_eq(rotated_point, Vec3d::new(0.0, 2.0, 0.0));
        assert_float_eq(
            point.distance_to(CoordinatedPoint::new(
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
        assert_vec_eq(line.to_global_line().start, Vec3d::new(12.0, -2.0, 0.0));
        assert_vec_eq(line.to_global_line().end, Vec3d::new(15.0, 2.0, 0.0));
        assert!(diagonal.intersects(cross_system.clone()));
        assert!(!diagonal.intersects(separate));

        let intersection = diagonal.get_intersection_point(cross_system);

        assert!(intersection.is_some());
        assert_global_point_eq(intersection.unwrap(), Vec3d::new(2.0, 2.0, 0.0));
        assert_global_point_eq(line.get_start(), Vec3d::new(12.0, -2.0, 0.0));
        assert_global_point_eq(line.get_end(), Vec3d::new(15.0, 2.0, 0.0));
    }

    #[test]
    fn coordinated_rectangle_methods_cover_conversion_and_intersection_lines() {
        let rectangle = CoordinatedRectangle::new(
            CoordinatedPoint::new(global_coordinate_system(), Vec3d::new(0.0, 0.0, 0.0)),
            CoordinatedPoint::new(global_coordinate_system(), Vec3d::new(4.0, 4.0, 0.0)),
        );
        let source = Rectangle::new(Vec3d::new(10.0, -5.0, 0.0), Vec3d::new(14.0, -1.0, 0.0));
        let from_rectangle = CoordinatedRectangle::new_from_rectangle(
            source.clone(),
            translated_coordinate_system(),
        );
        let converted = rectangle.convert_to(rotated_coordinate_system());
        let overlapping = CoordinatedRectangle::new_from_rectangle(
            Rectangle::new(Vec3d::new(2.0, 2.0, 0.0), Vec3d::new(6.0, 6.0, 0.0)),
            translated_coordinate_system(),
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

        let intersection = rectangle.get_intersection_line(line);

        assert!(intersection.is_some());
        let intersection = intersection.unwrap();
        assert_global_point_eq(intersection.get_start(), Vec3d::new(0.0, 2.0, 0.0));
        assert_global_point_eq(intersection.get_end(), Vec3d::new(4.0, 2.0, 0.0));
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
        let converted = circle.convert_to(global_coordinate_system());
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

        assert_global_point_eq(circle.get_center(), Vec3d::new(12.0, -2.0, 0.0));
        assert_float_eq(circle.get_radius(), 5.0);
        assert_global_point_eq(converted.get_center(), Vec3d::new(12.0, -2.0, 0.0));
        assert!(circle.intersects(&overlapping));
        assert_vec_eq(
            bbox.to_global_rectangle().get_top_left(),
            Vec3d::new(7.0, -7.0, 0.0),
        );
        assert_vec_eq(
            bbox.to_global_rectangle().get_bottom_right(),
            Vec3d::new(17.0, 3.0, 0.0),
        );
        assert_float_eq(circle.get_area(), 78.53981633974483);
        assert!(circle.contains_point(CoordinatedPoint::new(
            global_coordinate_system(),
            Vec3d::new(15.0, 2.0, 0.0),
        )));
        assert!(!circle.contains_point(CoordinatedPoint::new(
            global_coordinate_system(),
            Vec3d::new(18.0, 2.0, 0.0),
        )));

        let intersection = circle.get_intersection_line(line);

        assert!(intersection.is_some());
        let intersection = intersection.unwrap();
        assert_global_point_eq(intersection.get_start(), Vec3d::new(7.0, -2.0, 0.0));
        assert_global_point_eq(intersection.get_end(), Vec3d::new(17.0, -2.0, 0.0));
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
        let from_lines = CoordinatedRegionedAngle::new_from_lines(line1, line2, -180.0, 180.0);

        assert_float_eq(from_lines.get_angle_degrees(), 90.0);
        assert_float_eq(converted_angle.get_angle_degrees(), -60.0);
        assert_float_eq(converted_angle.get_min_degrees(), -180.0);
        assert_float_eq(converted_angle.get_max_degrees(), 180.0);
        assert_float_eq(
            converted_angle
                .get_coordinate_system()
                .get_angle_between(rotated.clone())
                .angle_degrees,
            0.0,
        );
        assert_float_eq(polar.get_radius(), 2.0);
        assert_float_eq(polar.get_angle().get_angle_degrees(), -60.0);
        assert_float_eq(
            polar
                .get_coordinate_system()
                .get_angle_between(rotated.clone())
                .angle_degrees,
            0.0,
        );
        assert_vec_eq(
            cartesian.get_local_point(),
            Vec3d::new(1.0, -1.7320508075688772, 0.0),
        );
        assert_global_point_eq(cartesian, Vec3d::new(1.7320508075688772, 1.0, 0.0));
        assert_float_eq(converted_polar.get_angle().get_angle_degrees(), -45.0);
        assert_vec_eq(
            converted_cartesian.get_local_point(),
            Vec3d::new(1.4142135623730951, -1.414213562373095, 0.0),
        );
        assert_global_point_eq(
            converted_cartesian,
            Vec3d::new(1.414213562373095, 1.4142135623730951, 0.0),
        );
    }
}

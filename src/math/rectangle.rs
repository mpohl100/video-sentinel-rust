#![warn(clippy::pedantic)]
#![allow(
    clippy::missing_errors_doc,
    clippy::missing_panics_doc,
    clippy::too_many_lines,
    clippy::unused_async_trait_impl
)]

use rs_math3d::Vec3d;

use super::{Line, Point};

#[derive(Clone, PartialEq)]
pub struct Rectangle {
    points: Vec<Point>,
}

impl Rectangle {
    pub(super) fn points(&self) -> &Vec<Point> {
        &self.points
    }
}

impl Rectangle {
    #[must_use]
    pub fn new(top_left: Vec3d, bottom_right: Vec3d) -> Self {
        let points = vec![
            Point { point: top_left },
            Point {
                point: bottom_right,
            },
        ];
        Self { points }
    }

    #[must_use]
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
            let input_line_begin = line.start();
            let input_line_end = line.end();
            let intersection_point = intersection_points[0];
            if self.contains_point(input_line_begin) {
                Some(Line::new(input_line_begin, intersection_point.point))
            } else if self.contains_point(input_line_end) {
                Some(Line::new(input_line_end, intersection_point.point))
            } else {
                None
            }
        } else {
            None
        }
    }

    #[must_use]
    pub fn contains_point(&self, point: Vec3d) -> bool {
        let top_left = self.get_top_left();
        let bottom_right = self.get_bottom_right();
        !(point.x < top_left.x
            || point.x > bottom_right.x
            || point.y < top_left.y
            || point.y > bottom_right.y)
    }

    #[must_use]
    pub fn get_area(&self) -> f64 {
        let width = (self.points[1].point.x - self.points[0].point.x).abs();
        let height = (self.points[1].point.y - self.points[0].point.y).abs();
        width * height
    }

    #[must_use]
    pub fn get_center(&self) -> Vec3d {
        let center_x = f64::midpoint(self.points[0].point.x, self.points[1].point.x);
        let center_y = f64::midpoint(self.points[0].point.y, self.points[1].point.y);
        Vec3d::new(center_x, center_y, 0.0)
    }

    #[must_use]
    pub fn get_top_left(&self) -> Vec3d {
        self.points[0].point
    }

    #[must_use]
    pub fn get_bottom_right(&self) -> Vec3d {
        self.points[1].point
    }

    #[must_use]
    pub fn get_width(&self) -> f64 {
        (self.points[1].point.x - self.points[0].point.x).abs()
    }

    #[must_use]
    pub fn get_height(&self) -> f64 {
        (self.points[1].point.y - self.points[0].point.y).abs()
    }

    #[must_use]
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

    #[must_use]
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

    #[must_use]
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

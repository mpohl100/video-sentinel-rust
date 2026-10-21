#![warn(clippy::pedantic)]
#![allow(
    clippy::missing_errors_doc,
    clippy::missing_panics_doc,
    clippy::too_many_lines,
    clippy::unused_async_trait_impl
)]

use crate::math::Rectangle as MathRectangle;
use rs_math3d::Vec3d;

#[derive(Clone, Copy)]
pub struct Rectangle {
    top_left: Vec3d,
    bottom_right: Vec3d,
}

impl Rectangle {
    pub(super) fn top_left(&self) -> Vec3d {
        self.top_left
    }

    pub(super) fn bottom_right(&self) -> Vec3d {
        self.bottom_right
    }
}

impl Rectangle {
    #[must_use]
    pub fn new(top_left: Vec3d, bottom_right: Vec3d) -> Self {
        Self {
            top_left,
            bottom_right,
        }
    }

    #[must_use]
    pub fn new_from_math_rectangle(rectangle: &MathRectangle) -> Self {
        Self {
            top_left: rectangle.get_top_left(),
            bottom_right: rectangle.get_bottom_right(),
        }
    }

    #[must_use]
    pub fn new_from_dims(top_left: Vec3d, width: f64, height: f64) -> Self {
        Self {
            top_left,
            bottom_right: Vec3d::new(top_left.x + width, top_left.y + height, 0.0),
        }
    }

    #[must_use]
    pub fn get_top_left(&self) -> Vec3d {
        self.top_left
    }

    #[must_use]
    pub fn get_bottom_right(&self) -> Vec3d {
        self.bottom_right
    }

    #[must_use]
    pub fn get_area(&self) -> f64 {
        let width = self.bottom_right.x - self.top_left.x + 1.0;
        let height = self.bottom_right.y - self.top_left.y + 1.0;
        width * height
    }

    #[must_use]
    pub fn overlaps(&self, other: &Rectangle) -> bool {
        !(self.bottom_right.x < other.top_left.x
            || self.top_left.x > other.bottom_right.x
            || self.bottom_right.y < other.top_left.y
            || self.top_left.y > other.bottom_right.y)
    }

    #[must_use]
    pub fn get_width(&self) -> f64 {
        self.bottom_right.x - self.top_left.x + 1.0
    }

    #[must_use]
    pub fn get_height(&self) -> f64 {
        self.bottom_right.y - self.top_left.y + 1.0
    }
}

#![warn(clippy::pedantic)]
#![allow(
    clippy::missing_errors_doc,
    clippy::missing_panics_doc,
    clippy::too_many_lines,
    clippy::unused_async_trait_impl
)]

use rs_math3d::Vec3d;

use super::{Rectangle, rotate_onehundred_eighty_degrees};

#[derive(Clone)]
pub struct RelativeRectangle {
    top_left: Vec3d,
    dims: Vec3d,
}

impl RelativeRectangle {
    #[cfg(test)]
    pub(super) fn top_left(&self) -> Vec3d {
        self.top_left
    }

    #[cfg(test)]
    pub(super) fn dims(&self) -> Vec3d {
        self.dims
    }

    #[must_use]
    pub fn new(top_left: Vec3d, dims: Vec3d) -> Self {
        Self { top_left, dims }
    }

    #[must_use]
    pub fn new_from_rectangles(first: Rectangle, second: Rectangle) -> Self {
        let rel_x = (first.get_top_left().x - second.get_top_left().x) / second.get_width();
        let rel_y = (first.get_top_left().y - second.get_top_left().y) / second.get_height();
        let rel_width = first.get_width() / second.get_width();
        let rel_height = first.get_height() / second.get_height();
        Self {
            top_left: Vec3d::new(rel_x, rel_y, 0.0),
            dims: Vec3d::new(rel_width, rel_height, 0.0),
        }
    }

    #[must_use]
    pub fn invert(&self) -> RelativeRectangle {
        // convert the C++ code to rust code
        let tl = self.top_left;
        let br = self.top_left + self.dims;

        let middle_point = Vec3d::new(0.5, 0.5, 0.0);
        let x_vec = Vec3d::new(1.0, 0.0, 0.0);
        let y_vec = Vec3d::new(0.0, 1.0, 0.0);

        let tl_new = rotate_onehundred_eighty_degrees(br, middle_point, x_vec, y_vec);
        let br_new = rotate_onehundred_eighty_degrees(tl, middle_point, x_vec, y_vec);

        let new_top_left = Vec3d::new(tl_new.x, tl_new.y, 0.0);
        let new_bottom_right = Vec3d::new(br_new.x, br_new.y, 0.0);
        RelativeRectangle {
            top_left: new_top_left,
            dims: new_bottom_right - new_top_left,
        }
    }

    #[must_use]
    pub fn multiply_with_rectangle(&self, rectangle: Rectangle) -> Rectangle {
        let abs_x = rectangle.get_top_left().x + self.top_left.x * rectangle.get_width();
        let abs_y = rectangle.get_top_left().y + self.top_left.y * rectangle.get_height();
        let abs_width = self.dims.x * rectangle.get_width();
        let abs_height = self.dims.y * rectangle.get_height();
        Rectangle::new_from_dims(Vec3d::new(abs_x, abs_y, 0.0), abs_width, abs_height)
    }
}

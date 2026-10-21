#![warn(clippy::pedantic)]
#![allow(
    clippy::missing_errors_doc,
    clippy::missing_panics_doc,
    clippy::too_many_lines,
    clippy::unused_async_trait_impl
)]

use rs_math3d::Vec3d;
use std::sync::{Arc, Mutex};

use super::{Rectangle, RelativeRectangle};

#[derive(Clone)]
pub struct WrappedRelativeRectangle {
    relative_rectangle: Arc<Mutex<RelativeRectangle>>,
}

impl WrappedRelativeRectangle {
    #[must_use]
    pub fn new(relative_rectangle: RelativeRectangle) -> Self {
        Self {
            relative_rectangle: Arc::new(Mutex::new(relative_rectangle)),
        }
    }

    #[must_use]
    pub fn new_from_rectangles(first: Rectangle, second: Rectangle) -> Self {
        Self::new(RelativeRectangle::new_from_rectangles(first, second))
    }

    #[must_use]
    pub fn multiply_with_rectangle(&self, rectangle: Rectangle) -> Rectangle {
        let relative_rectangle = self.relative_rectangle.lock().unwrap();
        relative_rectangle.multiply_with_rectangle(rectangle)
    }

    #[must_use]
    pub fn to_rectangle(&self) -> Rectangle {
        let unit_scale_reference =
            Rectangle::new(Vec3d::new(0.0, 0.0, 0.0), Vec3d::new(0.0, 0.0, 0.0));
        // Rectangle width/height are inclusive (+1), so identical points represent a unit scale.
        self.multiply_with_rectangle(unit_scale_reference)
    }

    #[must_use]
    pub fn overlaps(&self, other: &Rectangle) -> bool {
        self.to_rectangle().overlaps(other)
    }
}

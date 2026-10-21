mod annotated_slice;
mod basic_params;
mod cached_data;
mod color;
mod funcs;
mod gray_pixel_reader;
mod preliminary_annotated_slice;
mod preliminary_slice;
mod rectangle;
mod relative_rectangle;
mod rgb_pixel_reader;
mod slice;
mod slice_line;
mod slice_matrix;
mod wrapped_relative_rectangle;
mod wrapped_rgb_image;

pub use annotated_slice::AnnotatedSlice;
pub use basic_params::BasicParams;
pub use cached_data::CachedData;
pub use color::Color;
pub use funcs::{calculate_slices, find_connected_slices};
use funcs::{
    f64_from_usize, i32_from_u32, i32_from_usize, point_in_polygon,
    rotate_onehundred_eighty_degrees, u32_from_i32, u32_from_usize, usize_from_f64, usize_from_u32,
};
use gray_pixel_reader::GrayPixelReader;
use preliminary_annotated_slice::PreliminaryAnnotatedSlice;
use preliminary_slice::PreliminarySlice;
pub use rectangle::Rectangle;
pub use relative_rectangle::RelativeRectangle;
use rgb_pixel_reader::RgbPixelReader;
pub use slice::Slice;
pub use slice_line::SliceLine;
pub use slice_matrix::SliceMatrix;
pub use wrapped_relative_rectangle::WrappedRelativeRectangle;
pub use wrapped_rgb_image::WrappedRgbImage;

#![warn(clippy::pedantic)]
#![allow(
    clippy::missing_errors_doc,
    clippy::missing_panics_doc,
    clippy::too_many_lines,
    clippy::unused_async_trait_impl
)]

use crate::math::AnonymizedCoordinateSystem;
use crate::math::CoordinateSystem;
use crate::math::CoordinatedPoint;
use num_traits::ToPrimitive;
use opencv::core::{self, Vector as CvVector};
use opencv::imgproc;
use rs_math3d::Vector;
use rs_math3d::{Vec3d, Vector3};

use super::{
    AnnotatedSlice, BasicParams, GrayPixelReader, PreliminaryAnnotatedSlice, PreliminarySlice,
    Rectangle, RgbPixelReader, Slice, SliceLine, SliceMatrix, WrappedRgbImage,
};

pub(super) fn i32_from_u32(value: u32) -> i32 {
    value
        .to_i32()
        .expect("u32 image dimension should fit within i32")
}

pub(super) fn i32_from_usize(value: usize) -> i32 {
    value.to_i32().expect("usize index should fit within i32")
}

pub(super) fn u32_from_i32(value: i32) -> u32 {
    value
        .to_u32()
        .expect("i32 coordinate should be non-negative")
}

pub(super) fn u32_from_usize(value: usize) -> u32 {
    value.to_u32().expect("usize index should fit within u32")
}

pub(super) fn usize_from_u32(value: u32) -> usize {
    value
        .to_usize()
        .expect("u32 image dimension should fit within usize")
}

pub(super) fn usize_from_f64(value: f64) -> usize {
    value
        .to_usize()
        .expect("f64 coordinate should fit within usize")
}

pub(super) fn f64_from_usize(value: usize) -> f64 {
    value
        .to_f64()
        .expect("usize value should convert losslessly for image coordinates")
}

pub(super) fn f32_from_f64(value: f64) -> f32 {
    value
        .to_f32()
        .expect("polygon coordinate should fit within f32")
}

pub(super) fn point_in_polygon(point: (f64, f64), polygon: &[(i32, i32)]) -> bool {
    if polygon.len() < 3 {
        return false;
    }

    let mut contour = CvVector::<core::Point>::new();
    for &(x, y) in polygon {
        contour.push(core::Point::new(x, y));
    }

    imgproc::point_polygon_test(
        &contour,
        core::Point2f::new(f32_from_f64(point.0), f32_from_f64(point.1)),
        false,
    )
    .is_ok_and(|distance| distance >= 0.0)
}

pub(super) fn rotate_onehundred_eighty_degrees(
    point: Vec3d,
    center: Vec3d,
    x_vec: Vec3d,
    y_vec: Vec3d,
) -> Vec3d {
    let first_mirror = mirror(point, center, x_vec);
    mirror(first_mirror, center, y_vec)
}

pub(super) fn mirror(point: Vec3d, center: Vec3d, direction: Vec3d) -> Vec3d {
    let to_center = point - center;
    let projection_length = Vector3::<f64>::dot(&to_center, &direction);
    let projection = direction * projection_length;
    point - projection * 2.0
}

pub(super) fn compute_smoothed_gradient_squared(
    gray_image: &GrayPixelReader<'_>,
    x: usize,
    y: usize,
) -> f64 {
    let compute_gradient = |x_in: usize, y_in: usize| unsafe {
        let tl = gray_image.value_unchecked(x_in - 1, y_in - 1);
        let tc = gray_image.value_unchecked(x_in, y_in - 1);
        let tr = gray_image.value_unchecked(x_in + 1, y_in - 1);
        let cl = gray_image.value_unchecked(x_in - 1, y_in);
        let cr = gray_image.value_unchecked(x_in + 1, y_in);
        let bl = gray_image.value_unchecked(x_in - 1, y_in + 1);
        let bc = gray_image.value_unchecked(x_in, y_in + 1);
        let br = gray_image.value_unchecked(x_in + 1, y_in + 1);

        let sqrt2 = std::f64::consts::FRAC_1_SQRT_2;
        let grad_tl_br = br - tl;
        let grad_cl_cr = cr - cl;
        let grad_bl_tr = tr - bl;
        let grad_bc_tc = tc - bc;

        let grad_x = grad_cl_cr + grad_tl_br * sqrt2 + grad_bl_tr * sqrt2;
        let grad_y = -grad_bc_tc + grad_tl_br * sqrt2 - grad_bl_tr * sqrt2;

        grad_x * grad_x + grad_y * grad_y
    };

    let gradients = [
        compute_gradient(x - 1, y - 1),
        compute_gradient(x, y - 1),
        compute_gradient(x + 1, y - 1),
        compute_gradient(x - 1, y),
        compute_gradient(x, y),
        compute_gradient(x + 1, y),
        compute_gradient(x - 1, y + 1),
        compute_gradient(x, y + 1),
        compute_gradient(x + 1, y + 1),
    ];

    let sum: f64 = gradients.iter().sum();
    sum / 9.0
}

pub(super) fn compute_smoothed_gradient_channel(
    image: &RgbPixelReader<'_>,
    x: usize,
    y: usize,
    channel: usize,
) -> f64 {
    let compute_gradient = |x_in: usize, y_in: usize| unsafe {
        let tl = image.channel_unchecked(x_in - 1, y_in - 1, channel);
        let tc = image.channel_unchecked(x_in, y_in - 1, channel);
        let tr = image.channel_unchecked(x_in + 1, y_in - 1, channel);
        let cl = image.channel_unchecked(x_in - 1, y_in, channel);
        let cr = image.channel_unchecked(x_in + 1, y_in, channel);
        let bl = image.channel_unchecked(x_in - 1, y_in + 1, channel);
        let bc = image.channel_unchecked(x_in, y_in + 1, channel);
        let br = image.channel_unchecked(x_in + 1, y_in + 1, channel);

        let sqrt2 = std::f64::consts::FRAC_1_SQRT_2;
        let grad_tl_br = br - tl;
        let grad_cl_cr = cr - cl;
        let grad_bl_tr = tr - bl;
        let grad_bc_tc = tc - bc;

        let grad_x = grad_cl_cr + grad_tl_br * sqrt2 + grad_bl_tr * sqrt2;
        let grad_y = -grad_bc_tc + grad_tl_br * sqrt2 - grad_bl_tr * sqrt2;

        grad_x * grad_x + grad_y * grad_y
    };

    let gradients = [
        compute_gradient(x - 1, y - 1),
        compute_gradient(x, y - 1),
        compute_gradient(x + 1, y - 1),
        compute_gradient(x - 1, y),
        compute_gradient(x, y),
        compute_gradient(x + 1, y),
        compute_gradient(x - 1, y + 1),
        compute_gradient(x, y + 1),
        compute_gradient(x + 1, y + 1),
    ];

    let sum: f64 = gradients.iter().sum();
    sum / 9.0
}

pub(super) fn emplace_current_slice(
    current_slice: &mut Option<PreliminaryAnnotatedSlice>,
    current_line: &mut SliceLine,
) {
    if let Some(slice) = current_slice.take() {
        let annotated_slice = AnnotatedSlice::new(
            Slice::new(&slice.get_slice().get_start(), &slice.get_slice().get_end()),
            current_line.get_line_number(),
        );
        // the slices will be sorted and unique by construction here
        current_line.add_unchecked(annotated_slice);
    }
}

#[must_use]
pub fn calculate_slices(
    image: &WrappedRgbImage,
    rectangle: Rectangle,
    params: BasicParams,
) -> SliceMatrix {
    // checkt the rectangle is within the bounds of the image
    if rectangle.top_left().x < 0.0
        || rectangle.top_left().y < 0.0
        || rectangle.bottom_right().x > f64::from(image.width())
        || rectangle.bottom_right().y > f64::from(image.height())
    {
        panic!("Rectangle is out of bounds of the image");
    }

    let mut slice_matrix = SliceMatrix::new(image.clone());

    if params.do_grayscale() {
        let gray_image = image
            .grayscale_mat()
            .expect("failed to convert image to grayscale");
        let gray_reader =
            GrayPixelReader::new(&gray_image).expect("failed to access grayscale image");
        let mut current_slice = None;
        let global_coordinate_system = AnonymizedCoordinateSystem::Direct(CoordinateSystem::new(
            Vec3d::new(0.0, 0.0, 0.0),
            Vec3d::new(1.0, 0.0, 0.0),
            Vec3d::new(0.0, 1.0, 0.0),
        ));

        let gradient_threshold = f64::from(params.gradient_threshold());
        let threshold_squared = gradient_threshold * gradient_threshold;
        for y in usize_from_f64(rectangle.top_left().y) + 2
            ..usize_from_f64(rectangle.bottom_right().y) - 2
        {
            let mut current_line = SliceLine::new(y, Vec::new());
            for x in usize_from_f64(rectangle.top_left().x) + 2
                ..usize_from_f64(rectangle.bottom_right().x) - 2
            {
                let gradient = compute_smoothed_gradient_squared(&gray_reader, x, y);
                if gradient <= threshold_squared {
                    if current_slice.is_none() {
                        current_slice = Some(PreliminaryAnnotatedSlice::new(
                            PreliminarySlice::new(
                                CoordinatedPoint::new(
                                    global_coordinate_system.clone(),
                                    Vec3d::new(f64_from_usize(x), f64_from_usize(y), 0.0),
                                ),
                                CoordinatedPoint::new(
                                    global_coordinate_system.clone(),
                                    Vec3d::new(f64_from_usize(x), f64_from_usize(y), 0.0),
                                ),
                            ),
                            y,
                        ));
                    } else if let Some(slice) = &mut current_slice {
                        *slice = PreliminaryAnnotatedSlice::new(
                            PreliminarySlice::new(
                                slice.slice().get_start(),
                                CoordinatedPoint::new(
                                    slice.slice().get_end().get_coordinate_system().clone(),
                                    Vec3d::new(f64_from_usize(x), f64_from_usize(y), 0.0),
                                ),
                            ),
                            slice.line_number(),
                        );
                    }
                } else {
                    emplace_current_slice(&mut current_slice, &mut current_line);
                }
            }
            emplace_current_slice(&mut current_slice, &mut current_line);
            slice_matrix.add(current_line);
        }
        slice_matrix
    } else {
        let rgb_reader = RgbPixelReader::new(image).expect("failed to access RGB image");
        let mut current_slice = None;
        let gradient_threshold = f64::from(params.gradient_threshold());
        let threshold_squared = gradient_threshold * gradient_threshold;
        for y in usize_from_f64(rectangle.top_left().y) + 2
            ..usize_from_f64(rectangle.bottom_right().y) - 2
        {
            let mut current_line = SliceLine::new(y, Vec::new());
            for x in usize_from_f64(rectangle.top_left().x) + 2
                ..usize_from_f64(rectangle.bottom_right().x) - 2
            {
                let gradient_0 = compute_smoothed_gradient_channel(&rgb_reader, x, y, 0);
                let gradient_1 = compute_smoothed_gradient_channel(&rgb_reader, x, y, 1);
                let gradient_2 = compute_smoothed_gradient_channel(&rgb_reader, x, y, 2);

                if gradient_0 <= threshold_squared
                    && gradient_1 <= threshold_squared
                    && gradient_2 <= threshold_squared
                {
                    let global_coordinate_system =
                        AnonymizedCoordinateSystem::Direct(CoordinateSystem::new(
                            Vec3d::new(0.0, 0.0, 0.0),
                            Vec3d::new(1.0, 0.0, 0.0),
                            Vec3d::new(0.0, 1.0, 0.0),
                        ));
                    if current_slice.is_none() {
                        current_slice = Some(PreliminaryAnnotatedSlice::new(
                            PreliminarySlice::new(
                                CoordinatedPoint::new(
                                    global_coordinate_system.clone(),
                                    Vec3d::new(f64_from_usize(x), f64_from_usize(y), 0.0),
                                ),
                                CoordinatedPoint::new(
                                    global_coordinate_system.clone(),
                                    Vec3d::new(f64_from_usize(x), f64_from_usize(y), 0.0),
                                ),
                            ),
                            y,
                        ));
                    } else if let Some(slice) = &mut current_slice {
                        *slice = PreliminaryAnnotatedSlice::new(
                            PreliminarySlice::new(
                                slice.slice().get_start(),
                                CoordinatedPoint::new(
                                    global_coordinate_system.clone(),
                                    Vec3d::new(f64_from_usize(x), f64_from_usize(y), 0.0),
                                ),
                            ),
                            slice.line_number(),
                        );
                    }
                } else {
                    emplace_current_slice(&mut current_slice, &mut current_line);
                }
            }
            emplace_current_slice(&mut current_slice, &mut current_line);
            slice_matrix.add(current_line);
        }
        slice_matrix
    }
    // print all lines of the slice matrix.
}

pub(super) fn go_direction(
    slice_matrix: &mut SliceMatrix,
    connected_matrix: &mut SliceMatrix,
    direction: i32,
) -> bool {
    if direction == -1 {
        let mut top_line_number = connected_matrix.get_top_line_number();
        if top_line_number.is_none() {
            let tl_slice = slice_matrix.get_top_left_slice();
            if let Some(tl_slice) = tl_slice {
                connected_matrix.add_slices(tl_slice.clone());
                slice_matrix.remove_slices(&tl_slice);
            } else {
                return false;
            }
            top_line_number = connected_matrix.get_top_line_number();
        }
        let mut added_something = false;
        let mut current_line = top_line_number.expect("Did not find a top line number");
        loop {
            let next_line = connected_matrix.get_line(current_line);
            if let Some(line) = next_line {
                let touching_slices = slice_matrix.find_touching_slices(line, -1);
                if let Some(touching_slices) = touching_slices {
                    connected_matrix.add_slices(touching_slices.clone());
                    slice_matrix.remove_slices(&touching_slices);
                    added_something = true;
                    current_line = touching_slices.get_line_number();
                } else {
                    break;
                }
            } else {
                break;
            }
        }
        return added_something;
    } else if direction == 1 {
        let mut bottom_line_number = connected_matrix.get_bottom_line_number();
        if bottom_line_number.is_none() {
            let tl_slice = slice_matrix.get_top_left_slice();
            if let Some(tl_slice) = tl_slice {
                connected_matrix.add_slices(tl_slice.clone());
                slice_matrix.remove_slices(&tl_slice);
            } else {
                return false;
            }
            bottom_line_number = connected_matrix.get_bottom_line_number();
        }
        let mut added_something = false;
        let mut current_line = bottom_line_number.expect("Did not find a bottom line number");
        loop {
            let next_line = connected_matrix.get_line(current_line);
            if let Some(line) = next_line {
                let touching_slices = slice_matrix.find_touching_slices(line, 1);
                if let Some(touching_slices) = touching_slices {
                    connected_matrix.add_slices(touching_slices.clone());
                    slice_matrix.remove_slices(&touching_slices);
                    added_something = true;
                    current_line = touching_slices.get_line_number();
                } else {
                    break;
                }
            } else {
                break;
            }
        }
        return added_something;
    }
    false
}

pub(super) fn find_next_connected_slice_matrix(
    slice_matrix: &mut SliceMatrix,
) -> Option<SliceMatrix> {
    let mut connected_matrix = SliceMatrix::new(slice_matrix.image().clone());
    let mut direction = -1; // -1 = go top to bottom, 1 = go bottom to top
    let mut found_nothing_counter = 0;
    loop {
        let added_something = go_direction(slice_matrix, &mut connected_matrix, direction);
        if direction == -1 {
            direction = 1;
        } else {
            direction = -1;
        }
        if added_something {
            found_nothing_counter = 0;
        } else {
            found_nothing_counter += 1;
        }
        if found_nothing_counter >= 2 {
            break;
        }
    }
    if connected_matrix.lines().is_empty() {
        None
    } else {
        Some(connected_matrix)
    }
}

pub fn find_connected_slices(slice_matrix: &mut SliceMatrix) -> Vec<SliceMatrix> {
    let mut connected_matrices = Vec::new();

    while let Some(connected_slice_matrix) = find_next_connected_slice_matrix(slice_matrix) {
        connected_matrices.push(connected_slice_matrix);
    }

    connected_matrices
}

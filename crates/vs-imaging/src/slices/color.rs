#![warn(clippy::pedantic)]
#![allow(
    clippy::missing_errors_doc,
    clippy::missing_panics_doc,
    clippy::too_many_lines,
    clippy::unused_async_trait_impl
)]

#[derive(Clone, Eq, PartialEq, PartialOrd, Ord)]
pub enum Color {
    Red,
    Green,
    Blue,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::math::AnonymizedCoordinateSystem;
    use crate::math::CoordinateSystem;
    use crate::math::CoordinatedCircle;
    use crate::math::CoordinatedPoint;
    use crate::math::CoordinatedRectangle;
    use crate::slices::funcs::compute_smoothed_gradient_channel;
    use crate::slices::funcs::compute_smoothed_gradient_squared;
    use crate::slices::funcs::emplace_current_slice;
    use crate::slices::funcs::find_next_connected_slice_matrix;
    use crate::slices::funcs::go_direction;
    use crate::slices::funcs::mirror;

    use crate::math::Rectangle as MathRectangle;

    use crate::slices::*;

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

    fn point_in(cs: AnonymizedCoordinateSystem, x: f64, y: f64) -> CoordinatedPoint {
        CoordinatedPoint::new(cs, Vec3d::new(x, y, 0.0))
    }

    fn point(x: f64, y: f64) -> CoordinatedPoint {
        point_in(global_coordinate_system(), x, y)
    }

    fn slice(x1: f64, y: f64, x2: f64) -> Slice {
        Slice::new(&point(x1, y), &point(x2, y))
    }

    fn preliminary_annotated_slice(
        x1: f64,
        y: f64,
        x2: f64,
        line_number: usize,
    ) -> PreliminaryAnnotatedSlice {
        PreliminaryAnnotatedSlice::new(
            PreliminarySlice::new(point(x1, y), point(x2, y)),
            line_number,
        )
    }

    fn annotated_slice(x1: f64, y: f64, x2: f64, line_number: usize) -> AnnotatedSlice {
        AnnotatedSlice::new(slice(x1, y, x2), line_number)
    }

    fn slice_line(line_number: usize, ranges: &[(f64, f64)]) -> SliceLine {
        let slices = ranges
            .iter()
            .map(|(start, end)| {
                annotated_slice(*start, f64_from_usize(line_number), *end, line_number)
            })
            .collect();
        SliceLine::new(line_number, slices)
    }

    fn solid_image(width: u32, height: u32, color: [u8; 3]) -> WrappedRgbImage {
        WrappedRgbImage::new_with_color(width, height, color)
    }

    fn slice_matrix(lines: Vec<SliceLine>) -> SliceMatrix {
        let mut matrix = SliceMatrix::new(solid_image(64, 64, [10, 20, 30]));
        for line in lines {
            matrix.add(line);
        }
        matrix
    }

    #[test]
    fn slice_methods_cover_equality_and_coordinate_conversion() {
        let translated_slice = Slice::new(
            &point_in(translated_coordinate_system(), 2.0, 3.0),
            &point_in(translated_coordinate_system(), 5.0, 3.0),
        );
        let global_slice = Slice::new(&point(12.0, -2.0), &point(15.0, -2.0));
        let rotated = translated_slice.convert_to(&rotated_coordinate_system());

        assert!(translated_slice == global_slice);
        assert_vec_eq(
            translated_slice.get_start().get_local_point(),
            Vec3d::new(12.0, -2.0, 0.0),
        );
        assert_vec_eq(
            translated_slice.get_end().get_local_point(),
            Vec3d::new(15.0, -2.0, 0.0),
        );
        assert_vec_eq(
            translated_slice
                .convert_to_global()
                .get_start()
                .get_local_point(),
            Vec3d::new(12.0, -2.0, 0.0),
        );
        assert_vec_eq(
            translated_slice
                .convert_to_global()
                .get_end()
                .get_local_point(),
            Vec3d::new(15.0, -2.0, 0.0),
        );
        assert_vec_eq(
            rotated.get_start().get_local_point(),
            Vec3d::new(-4.0, -11.0, 0.0),
        );
        assert_vec_eq(
            rotated.get_end().get_local_point(),
            Vec3d::new(-4.0, -11.0, 0.0),
        );
    }

    #[test]
    fn annotated_slice_methods_cover_touch_mass_midpoint_and_accessors() {
        let current = annotated_slice(0.0, 1.0, 10.0, 1);
        let touching = annotated_slice(5.0, 2.0, 15.0, 2);
        let disjoint = annotated_slice(11.0, 2.0, 20.0, 2);
        let same_line = annotated_slice(5.0, 1.0, 15.0, 1);

        assert!(current.touches(&touching));
        assert!(!current.touches(&disjoint));
        assert!(!current.touches(&same_line));
        assert_float_eq(current.get_mass(), 11.0);
        assert_vec_eq(current.get_midpoint(), Vec3d::new(5.0, 1.0, 0.0));
        assert!(current.get_slice() == slice(0.0, 1.0, 10.0));
        assert_vec_eq(
            current.get_start().get_local_point(),
            Vec3d::new(0.0, 1.0, 0.0),
        );
        assert_vec_eq(
            current.get_end().get_local_point(),
            Vec3d::new(10.0, 1.0, 0.0),
        );
    }

    #[test]
    fn slice_line_methods_cover_sorting_dedup_and_merging() {
        let mut line = SliceLine::new(3, vec![annotated_slice(5.0, 3.0, 8.0, 3)]);
        line.add(annotated_slice(0.0, 3.0, 2.0, 3));
        line.add(annotated_slice(0.0, 3.0, 2.0, 3));
        line.add_slices(&SliceLine::new(
            3,
            vec![
                annotated_slice(7.0, 3.0, 10.0, 3),
                annotated_slice(12.0, 3.0, 14.0, 3),
            ],
        ));

        assert_eq!(line.get_line_number(), 3);
        assert_eq!(line.get_slices().len(), 4);
        assert_float_eq(line.get_slices()[0].get_start().get_x(), 0.0);
        assert_float_eq(line.get_slices()[1].get_start().get_x(), 5.0);

        line.expand_by(1.0);
        line.merge_overlapping_slices();

        assert_eq!(line.get_slices().len(), 2);
        assert_float_eq(line.get_slices()[0].get_start().get_x(), -1.0);
        assert_float_eq(line.get_slices()[0].get_end().get_x(), 3.0);
        assert_float_eq(line.get_slices()[1].get_start().get_x(), 4.0);
        assert_float_eq(line.get_slices()[1].get_end().get_x(), 15.0);
    }

    #[test]
    #[should_panic(expected = "Slice line number does not match SliceLine's line number")]
    fn slice_line_add_rejects_mismatched_line_numbers() {
        let mut line = SliceLine::new(4, Vec::new());
        line.add(annotated_slice(0.0, 5.0, 1.0, 5));
    }

    #[test]
    fn wrapped_rgb_image_methods_cover_direct_pixels_and_ascii_art() {
        let image = solid_image(3, 2, [7, 8, 9]);
        let ascii = WrappedRgbImage::new_from_ascii_art(".#\n#.");

        assert_eq!(image.get_pixel(1, 1), [7, 8, 9]);
        assert_eq!(ascii.get_pixel(0, 0), [0, 0, 0]);
        assert_eq!(ascii.get_pixel(1, 0), [255, 255, 255]);
        assert_eq!(ascii.get_pixel(0, 1), [255, 255, 255]);
        assert_eq!(ascii.get_pixel(1, 1), [0, 0, 0]);
    }

    #[test]
    fn slice_matrix_methods_cover_queries_mutation_and_cached_geometry() {
        let matrix = slice_matrix(vec![
            slice_line(1, &[(1.0, 3.0)]),
            slice_line(2, &[(2.0, 4.0)]),
            slice_line(4, &[(10.0, 12.0)]),
        ]);

        assert_eq!(matrix.get_top_line_number(), Some(1));
        assert_eq!(matrix.get_bottom_line_number(), Some(4));
        assert_eq!(matrix.get_top_left_slice().unwrap().get_slices().len(), 1);
        assert_eq!(matrix.get_line_below(1).unwrap().get_line_number(), 2);
        assert!(matrix.get_line_above(1).is_none());
        assert_eq!(matrix.get_line_above(2).unwrap().get_line_number(), 1);
        assert_eq!(matrix.get_line(4).unwrap().get_slices().len(), 1);
        assert_eq!(
            matrix
                .find_touching_slices(matrix.get_line(1).unwrap(), -1)
                .unwrap()
                .get_slices()
                .len(),
            1
        );
        assert!(
            matrix
                .find_touching_slices(matrix.get_line(2).unwrap(), -1)
                .is_none()
        );
        assert!(matrix.contains_point(&point(2.0, 1.0)));
        assert!(!matrix.contains_point(&point(5.0, 1.0)));
        assert_eq!(matrix.get_slice_lines().len(), 3);

        let longest = matrix
            .deduce_longest_distance_point(&point(0.0, 0.0))
            .unwrap();
        let bounding = matrix.get_bounding_box();
        let cached = matrix.calculate_cached_data();

        assert_vec_eq(longest.get_local_point(), Vec3d::new(12.0, 5.0, 0.0));
        assert_vec_eq(bounding.get_top_left(), Vec3d::new(1.0, 1.0, 0.0));
        assert_vec_eq(bounding.get_bottom_right(), Vec3d::new(12.0, 4.0, 0.0));
        assert_vec_eq(
            cached
                .get_bounding_box()
                .to_global_rectangle()
                .get_top_left(),
            Vec3d::new(1.0, 1.0, 0.0),
        );
        assert_vec_eq(
            cached
                .get_bounding_box()
                .to_global_rectangle()
                .get_bottom_right(),
            Vec3d::new(12.0, 4.0, 0.0),
        );
        assert_vec_eq(
            cached.get_center_of_mass().get_local_point(),
            Vec3d::new(5.333_333_333_333_333, 2.333_333_333_333_333_5, 0.0),
        );
        assert_float_eq(
            cached.get_bounding_circle().get_radius(),
            5.906_681_715_556_45,
        );
        assert_float_eq(cached.get_area(), 9.0);
        assert_vec_eq(cached.get_average_color_vec(), Vec3d::new(10.0, 20.0, 30.0));
    }

    #[test]
    fn slice_matrix_add_and_remove_slices_keep_lines_sorted() {
        let mut matrix = slice_matrix(vec![slice_line(2, &[(2.0, 4.0)])]);
        matrix.add_slices(slice_line(1, &[(0.0, 1.0)]));
        matrix.add_slices(slice_line(3, &[(6.0, 8.0)]));
        matrix.add_slices(slice_line(2, &[(5.0, 5.0)]));

        assert_eq!(matrix.get_slice_lines()[0].get_line_number(), 1);
        assert_eq!(matrix.get_slice_lines()[1].get_slices().len(), 2);
        assert_eq!(matrix.get_slice_lines()[2].get_line_number(), 3);

        matrix.remove_slices(&slice_line(2, &[(2.0, 4.0)]));
        assert_eq!(matrix.get_line(2).unwrap().get_slices().len(), 1);

        matrix.remove_slices(&slice_line(2, &[(5.0, 5.0)]));
        assert!(matrix.get_line(2).is_none());
    }

    #[test]
    fn cached_data_and_basic_params_getters_return_original_values() {
        let bounding_box = CoordinatedRectangle::new(point(1.0, 2.0), point(4.0, 6.0));
        let bounding_circle = CoordinatedCircle::new(point(2.5, 4.0), 3.25);
        let center_of_mass = point(2.5, 4.0);
        let cached = CachedData::new(
            bounding_box.clone(),
            bounding_circle.clone(),
            center_of_mass.clone(),
            12.0,
            Vec3d::new(11.0, 22.0, 33.0),
        );
        let basic = BasicParams::new(true, 17);

        assert_vec_eq(
            cached
                .get_bounding_box()
                .to_global_rectangle()
                .get_top_left(),
            Vec3d::new(1.0, 2.0, 0.0),
        );
        assert_float_eq(cached.get_bounding_circle().get_radius(), 3.25);
        assert_vec_eq(
            cached.get_center_of_mass().get_local_point(),
            Vec3d::new(2.5, 4.0, 0.0),
        );
        assert_float_eq(cached.get_area(), 12.0);
        assert_vec_eq(cached.get_average_color_vec(), Vec3d::new(11.0, 22.0, 33.0));
        assert!(basic.do_grayscale());
        assert_eq!(basic.gradient_threshold(), 17);
    }

    #[test]
    fn rectangle_and_relative_rectangle_methods_cover_current_inclusive_geometry() {
        let math_rectangle =
            MathRectangle::new(Vec3d::new(2.0, 3.0, 0.0), Vec3d::new(6.0, 8.0, 0.0));
        let rectangle = Rectangle::new(Vec3d::new(1.0, 2.0, 0.0), Vec3d::new(4.0, 5.0, 0.0));
        let from_math = Rectangle::new_from_math_rectangle(&math_rectangle);
        let from_dims = Rectangle::new_from_dims(Vec3d::new(10.0, 20.0, 0.0), 3.0, 4.0);
        let relative = RelativeRectangle::new_from_rectangles(
            Rectangle::new(Vec3d::new(1.0, 2.0, 0.0), Vec3d::new(3.0, 5.0, 0.0)),
            Rectangle::new(Vec3d::new(0.0, 0.0, 0.0), Vec3d::new(9.0, 9.0, 0.0)),
        );
        let inverted = relative.invert();
        let multiplied = relative.multiply_with_rectangle(Rectangle::new(
            Vec3d::new(10.0, 20.0, 0.0),
            Vec3d::new(19.0, 29.0, 0.0),
        ));

        assert_vec_eq(rectangle.get_top_left(), Vec3d::new(1.0, 2.0, 0.0));
        assert_vec_eq(rectangle.get_bottom_right(), Vec3d::new(4.0, 5.0, 0.0));
        assert_float_eq(rectangle.get_area(), 16.0);
        assert!(rectangle.overlaps(&Rectangle::new(
            Vec3d::new(4.0, 5.0, 0.0),
            Vec3d::new(8.0, 8.0, 0.0)
        )));
        assert!(!rectangle.overlaps(&Rectangle::new(
            Vec3d::new(5.0, 6.0, 0.0),
            Vec3d::new(8.0, 8.0, 0.0)
        )));
        assert_float_eq(rectangle.get_width(), 4.0);
        assert_float_eq(rectangle.get_height(), 4.0);
        assert_vec_eq(from_math.get_top_left(), Vec3d::new(2.0, 3.0, 0.0));
        assert_vec_eq(from_math.get_bottom_right(), Vec3d::new(6.0, 8.0, 0.0));
        assert_vec_eq(from_dims.get_bottom_right(), Vec3d::new(13.0, 24.0, 0.0));
        assert_vec_eq(relative.top_left(), Vec3d::new(0.1, 0.2, 0.0));
        assert_vec_eq(relative.dims(), Vec3d::new(0.3, 0.4, 0.0));
        assert_vec_eq(inverted.top_left(), Vec3d::new(0.6, 0.4, 0.0));
        assert_vec_eq(
            inverted.dims(),
            Vec3d::new(0.300_000_000_000_000_04, 0.4, 0.0),
        );
        assert_vec_eq(multiplied.get_top_left(), Vec3d::new(11.0, 22.0, 0.0));
        assert_vec_eq(multiplied.get_bottom_right(), Vec3d::new(14.0, 26.0, 0.0));
    }

    #[test]
    fn wrapped_relative_rectangle_and_color_related_types_cover_accessors() {
        let wrapped = WrappedRelativeRectangle::new(RelativeRectangle::new(
            Vec3d::new(0.25, 0.5, 0.0),
            Vec3d::new(0.5, 0.25, 0.0),
        ));
        let from_rectangles = WrappedRelativeRectangle::new_from_rectangles(
            Rectangle::new(Vec3d::new(2.0, 2.0, 0.0), Vec3d::new(4.0, 4.0, 0.0)),
            Rectangle::new(Vec3d::new(0.0, 0.0, 0.0), Vec3d::new(9.0, 9.0, 0.0)),
        );
        let multiplied = wrapped.multiply_with_rectangle(Rectangle::new(
            Vec3d::new(0.0, 0.0, 0.0),
            Vec3d::new(0.0, 0.0, 0.0),
        ));
        let unit_rectangle = wrapped.to_rectangle();

        assert!(Color::Red < Color::Green);
        assert!(Color::Green < Color::Blue);
        assert_vec_eq(multiplied.get_top_left(), Vec3d::new(0.25, 0.5, 0.0));
        assert_vec_eq(multiplied.get_bottom_right(), Vec3d::new(0.75, 0.75, 0.0));
        assert_vec_eq(unit_rectangle.get_top_left(), Vec3d::new(0.25, 0.5, 0.0));
        assert!(wrapped.overlaps(&Rectangle::new(
            Vec3d::new(0.5, 0.6, 0.0),
            Vec3d::new(0.8, 0.9, 0.0)
        )));
        assert_vec_eq(
            from_rectangles.to_rectangle().get_top_left(),
            Vec3d::new(0.2, 0.2, 0.0),
        );
    }

    #[test]
    fn mirror_and_rotate_onehundred_eighty_degrees_transform_points_as_expected() {
        let mirrored = mirror(
            Vec3d::new(3.0, 4.0, 0.0),
            Vec3d::new(1.0, 1.0, 0.0),
            Vec3d::new(1.0, 0.0, 0.0),
        );
        let rotated = rotate_onehundred_eighty_degrees(
            Vec3d::new(3.0, 4.0, 0.0),
            Vec3d::new(1.0, 1.0, 0.0),
            Vec3d::new(1.0, 0.0, 0.0),
            Vec3d::new(0.0, 1.0, 0.0),
        );

        assert_vec_eq(mirrored, Vec3d::new(-1.0, 4.0, 0.0));
        assert_vec_eq(rotated, Vec3d::new(-1.0, -2.0, 0.0));
    }

    #[test]
    fn gradient_helpers_distinguish_uniform_and_edge_images() {
        let gray_mat = WrappedRgbImage::new_with_color(7, 7, [100, 100, 100])
            .grayscale_mat()
            .expect("failed to convert uniform image to grayscale");
        let gray = GrayPixelReader::new(&gray_mat).expect("failed to access grayscale image");
        let color = WrappedRgbImage::new_with_color(7, 7, [0, 0, 0]);
        for y in 0..7 {
            for x in 3..7 {
                color.set_pixel(x, y, [255, 0, 0]);
            }
        }
        let color = RgbPixelReader::new(&color).expect("failed to access RGB image");

        assert!(compute_smoothed_gradient_squared(&gray, 3, 3).abs() < f64::EPSILON);
        assert!(compute_smoothed_gradient_channel(&color, 3, 3, 0) > 0.0);
        assert!(compute_smoothed_gradient_channel(&color, 3, 3, 1).abs() < f64::EPSILON);
        assert!(compute_smoothed_gradient_channel(&color, 3, 3, 2).abs() < f64::EPSILON);
    }

    #[test]
    fn emplace_current_slice_moves_pending_slice_once() {
        let mut current_slice = Some(preliminary_annotated_slice(2.0, 6.0, 4.0, 6));
        let mut line = SliceLine::new(6, Vec::new());

        emplace_current_slice(&mut current_slice, &mut line);
        emplace_current_slice(&mut current_slice, &mut line);

        assert!(current_slice.is_none());
        assert_eq!(line.get_slices().len(), 1);
        assert_float_eq(line.get_slices()[0].get_start().get_x(), 2.0);
    }

    #[test]
    fn calculate_slices_builds_expected_runs_for_grayscale_and_color_modes() {
        let ascii = "#######\n#######\n#######\n#######\n#######\n#######\n#######";
        let image = WrappedRgbImage::new_from_ascii_art(ascii);
        let rectangle = Rectangle::new(Vec3d::new(0.0, 0.0, 0.0), Vec3d::new(7.0, 7.0, 0.0));
        let grayscale = calculate_slices(&image, rectangle, BasicParams::new(true, 0));
        let color = calculate_slices(&image, rectangle, BasicParams::new(false, 0));

        assert_eq!(grayscale.get_slice_lines().len(), 3);
        assert_eq!(grayscale.get_slice_lines()[0].get_line_number(), 2);
        assert_eq!(grayscale.get_slice_lines()[0].get_slices().len(), 1);
        assert_float_eq(
            grayscale.get_slice_lines()[0].get_slices()[0]
                .get_start()
                .get_x(),
            2.0,
        );
        assert_float_eq(
            grayscale.get_slice_lines()[0].get_slices()[0]
                .get_end()
                .get_x(),
            4.0,
        );
        assert_eq!(color.get_slice_lines().len(), 3);
        assert_float_eq(
            color.get_slice_lines()[2].get_slices()[0]
                .get_start()
                .get_x(),
            2.0,
        );
        assert_float_eq(
            color.get_slice_lines()[2].get_slices()[0].get_end().get_x(),
            4.0,
        );
    }

    #[test]
    #[should_panic(expected = "Rectangle is out of bounds of the image")]
    fn calculate_slices_panics_when_rectangle_is_out_of_bounds() {
        let image = WrappedRgbImage::new_from_ascii_art(
            "#######\n#######\n#######\n#######\n#######\n#######\n#######",
        );
        let _ = calculate_slices(
            &image,
            Rectangle::new(Vec3d::new(0.0, 0.0, 0.0), Vec3d::new(8.0, 7.0, 0.0)),
            BasicParams::new(true, 0),
        );
    }

    #[test]
    fn go_direction_and_find_next_connected_slice_matrix_extract_components() {
        let mut remaining = slice_matrix(vec![
            slice_line(1, &[(0.0, 10.0)]),
            slice_line(2, &[(0.0, 10.0)]),
        ]);
        let mut connected = SliceMatrix::new(solid_image(64, 64, [1, 1, 1]));

        assert!(go_direction(&mut remaining, &mut connected, -1));
        assert_eq!(connected.get_slice_lines().len(), 2);
        assert!(remaining.get_slice_lines().is_empty());

        let mut remaining = slice_matrix(vec![
            slice_line(1, &[(0.0, 10.0)]),
            slice_line(2, &[(0.0, 10.0)]),
        ]);
        let component = find_next_connected_slice_matrix(&mut remaining).unwrap();

        assert_eq!(component.get_slice_lines().len(), 2);
        assert!(remaining.get_slice_lines().is_empty());
    }

    #[test]
    fn connected_components_detect_one_simple_shape_across_two_rows() {
        let mut slices = slice_matrix(vec![
            slice_line(1, &[(0.0, 10.0)]),
            slice_line(2, &[(0.0, 10.0)]),
        ]);
        let shapes = find_connected_slices(&mut slices);
        assert_eq!(shapes.len(), 1);
    }

    #[test]
    fn connected_components_detect_two_separate_shapes_on_one_row() {
        let mut slices = slice_matrix(vec![slice_line(1, &[(0.0, 10.0), (21.0, 30.0)])]);
        let shapes = find_connected_slices(&mut slices);
        assert_eq!(shapes.len(), 2);
    }

    #[test]
    fn connected_components_detect_two_vertical_shapes_across_two_rows() {
        let mut slices = slice_matrix(vec![
            slice_line(1, &[(0.0, 10.0), (21.0, 30.0)]),
            slice_line(2, &[(0.0, 10.0), (21.0, 30.0)]),
        ]);
        let shapes = find_connected_slices(&mut slices);
        assert_eq!(shapes.len(), 2);
    }

    #[test]
    fn connected_components_keep_diagonal_gap_when_shape_moves_down_right() {
        let mut slices = slice_matrix(vec![
            slice_line(1, &[(0.0, 10.0)]),
            slice_line(2, &[(11.0, 20.0)]),
        ]);
        let shapes = find_connected_slices(&mut slices);
        assert_eq!(shapes.len(), 2);
    }

    #[test]
    fn connected_components_keep_diagonal_gap_when_shape_moves_down_left() {
        let mut slices = slice_matrix(vec![
            slice_line(1, &[(11.0, 20.0)]),
            slice_line(2, &[(0.0, 10.0)]),
        ]);
        let shapes = find_connected_slices(&mut slices);
        assert_eq!(shapes.len(), 2);
    }

    #[test]
    fn connected_components_merge_snake_layout_into_one_shape() {
        let mut slices = slice_matrix(vec![
            slice_line(1, &[(0.0, 10.0), (21.0, 50.0)]),
            slice_line(2, &[(0.0, 10.0), (21.0, 30.0), (41.0, 50.0)]),
            slice_line(3, &[(0.0, 10.0), (21.0, 30.0), (41.0, 50.0)]),
            slice_line(4, &[(0.0, 30.0), (41.0, 50.0)]),
        ]);
        let shapes = find_connected_slices(&mut slices);
        assert_eq!(shapes.len(), 1);
    }

    #[test]
    fn slice_matrix_expand_grows_single_blob_by_one_pixel_on_each_side() {
        let mut slices = slice_matrix(vec![slice_line(1, &[(0.0, 10.0)])]);
        slices.expand();

        assert_eq!(slices.get_slice_lines().len(), 3);
        assert_eq!(slices.get_slice_lines()[0].get_line_number(), 0);
        assert_eq!(slices.get_slice_lines()[1].get_line_number(), 1);
        assert_eq!(slices.get_slice_lines()[2].get_line_number(), 2);
        for line in slices.get_slice_lines() {
            assert_eq!(line.get_slices().len(), 1);
            assert_float_eq(line.get_slices()[0].get_start().get_x(), -1.0);
            assert_float_eq(line.get_slices()[0].get_end().get_x(), 11.0);
        }
    }

    #[test]
    fn slice_matrix_expand_grows_two_blobs_and_merges_the_middle_rows() {
        let mut slices = slice_matrix(vec![
            slice_line(1, &[(0.0, 10.0), (21.0, 30.0)]),
            slice_line(2, &[(0.0, 30.0)]),
            slice_line(3, &[(0.0, 10.0), (21.0, 30.0)]),
        ]);
        slices.expand();

        assert_eq!(slices.get_slice_lines().len(), 5);
        assert_eq!(slices.get_slice_lines()[0].get_line_number(), 0);
        assert_eq!(slices.get_slice_lines()[0].get_slices().len(), 2);
        assert_float_eq(
            slices.get_slice_lines()[0].get_slices()[0]
                .get_start()
                .get_x(),
            -1.0,
        );
        assert_float_eq(
            slices.get_slice_lines()[0].get_slices()[0]
                .get_end()
                .get_x(),
            11.0,
        );
        assert_float_eq(
            slices.get_slice_lines()[0].get_slices()[1]
                .get_start()
                .get_x(),
            20.0,
        );
        assert_float_eq(
            slices.get_slice_lines()[0].get_slices()[1]
                .get_end()
                .get_x(),
            31.0,
        );
        assert_eq!(slices.get_slice_lines()[1].get_slices().len(), 1);
        assert_eq!(slices.get_slice_lines()[2].get_slices().len(), 1);
        assert_eq!(slices.get_slice_lines()[3].get_slices().len(), 1);
        assert_eq!(slices.get_slice_lines()[4].get_slices().len(), 2);
        assert_float_eq(
            slices.get_slice_lines()[1].get_slices()[0]
                .get_start()
                .get_x(),
            -1.0,
        );
        assert_float_eq(
            slices.get_slice_lines()[1].get_slices()[0]
                .get_end()
                .get_x(),
            31.0,
        );
        assert_float_eq(
            slices.get_slice_lines()[2].get_slices()[0]
                .get_start()
                .get_x(),
            -1.0,
        );
        assert_float_eq(
            slices.get_slice_lines()[2].get_slices()[0]
                .get_end()
                .get_x(),
            31.0,
        );
        assert_float_eq(
            slices.get_slice_lines()[3].get_slices()[0]
                .get_start()
                .get_x(),
            -1.0,
        );
        assert_float_eq(
            slices.get_slice_lines()[3].get_slices()[0]
                .get_end()
                .get_x(),
            31.0,
        );
        assert_float_eq(
            slices.get_slice_lines()[4].get_slices()[0]
                .get_start()
                .get_x(),
            -1.0,
        );
        assert_float_eq(
            slices.get_slice_lines()[4].get_slices()[0]
                .get_end()
                .get_x(),
            11.0,
        );
        assert_float_eq(
            slices.get_slice_lines()[4].get_slices()[1]
                .get_start()
                .get_x(),
            20.0,
        );
        assert_float_eq(
            slices.get_slice_lines()[4].get_slices()[1]
                .get_end()
                .get_x(),
            31.0,
        );
    }
}

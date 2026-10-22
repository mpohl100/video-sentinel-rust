#![warn(clippy::pedantic)]
#![allow(
    clippy::missing_errors_doc,
    clippy::missing_panics_doc,
    clippy::too_many_lines,
    clippy::unused_async_trait_impl
)]

use crate::slices::Rectangle;
use crate::slices::RelativeRectangle;
use crate::traced_mosaics::TracedRelativeMosaic;
use rs_math3d::Vec3d;

use super::combine_boxes;

#[derive(Clone)]
pub struct ReferenceObject {
    object_id: String,
    mosaics: Vec<TracedRelativeMosaic>,
}

impl ReferenceObject {
    #[must_use]
    pub fn new(object_id: String, mosaics: Vec<TracedRelativeMosaic>) -> Self {
        let mut mosaics = mosaics;
        mosaics.sort_by(|a, b| {
            a.get_relative_mosaic()
                .get_bounding_box()
                .to_global_rectangle()
                .get_area()
                .partial_cmp(
                    &b.get_relative_mosaic()
                        .get_bounding_box()
                        .to_global_rectangle()
                        .get_area(),
                )
                .unwrap()
        });
        mosaics.reverse();
        ReferenceObject { object_id, mosaics }
    }

    #[must_use]
    pub fn get_mosaics(&self, until_index: usize) -> Vec<TracedRelativeMosaic> {
        self.mosaics[..until_index.min(self.mosaics.len())].to_vec()
    }

    #[must_use]
    pub fn get_id(&self) -> String {
        self.object_id.clone()
    }

    #[must_use]
    pub fn get_surrounding_bounding_box(&self) -> Rectangle {
        let mut min_x = f64::INFINITY;
        let mut min_y = f64::INFINITY;
        let mut max_x = f64::NEG_INFINITY;
        let mut max_y = f64::NEG_INFINITY;

        for mosaic in &self.mosaics {
            let bounding_box = mosaic
                .get_relative_mosaic()
                .get_mosaic()
                .get_bounding_box()
                .to_global_rectangle();
            min_x = min_x.min(bounding_box.get_top_left().x);
            min_y = min_y.min(bounding_box.get_top_left().y);
            max_x = max_x.max(bounding_box.get_bottom_right().x);
            max_y = max_y.max(bounding_box.get_bottom_right().y);
        }
        Rectangle::new(Vec3d::new(min_x, min_y, 0.0), Vec3d::new(max_x, max_y, 0.0))
    }

    pub(super) fn get_relative_rectangle_to_smallest(&self) -> RelativeRectangle {
        assert!(
            self.mosaics.len() >= 2,
            "At least 2 mosaics are required to calculate the relative rectangle"
        );
        let smallest_bounding_box = Rectangle::new_from_math_rectangle(
            &self
                .mosaics
                .last()
                .unwrap()
                .get_relative_mosaic()
                .get_mosaic()
                .get_bounding_box()
                .to_global_rectangle(),
        );
        let biggest_bounding_box = combine_boxes(
            self.mosaics[..self.mosaics.len() - 1]
                .iter()
                .map(|mosaic| {
                    Rectangle::new_from_math_rectangle(
                        &mosaic
                            .get_relative_mosaic()
                            .get_mosaic()
                            .get_bounding_box()
                            .to_global_rectangle(),
                    )
                })
                .collect(),
        );
        RelativeRectangle::new_from_rectangles(smallest_bounding_box, biggest_bounding_box)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::eye::TileParams;
    use crate::math::AnonymizedCoordinateSystem;
    use crate::math::CoordinateSystem;
    use crate::math::Rectangle as MathRectangle;

    use crate::mosaics::deduce_mosaics;
    use crate::object_detection::*;

    use crate::slices::Rectangle;
    use crate::slices::{BasicParams, WrappedRgbImage, calculate_slices, find_connected_slices};
    use crate::traced_mosaics::TracedRelativeMosaic;

    use crate::traces::TraceParams;
    use rs_math3d::Vec3d;

    const EPSILON: f64 = 1e-8;

    #[derive(Clone)]
    struct ColoredTestRectangle {
        top_left: Vec3d,
        bottom_right: Vec3d,
        color: &'static str,
        rotation_angle_degrees: f64,
    }

    #[derive(Clone)]
    struct ColoredTestCircle {
        center: Vec3d,
        radius: f64,
        color: &'static str,
    }

    #[derive(Default, Clone)]
    struct ShapesData {
        rectangles: Vec<ColoredTestRectangle>,
        circles: Vec<ColoredTestCircle>,
    }

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

    fn rgb_from_name(color: &str) -> [u8; 3] {
        match color {
            "red" => [255, 0, 0],
            "green" => [0, 255, 0],
            "blue" => [0, 0, 255],
            _ => [255, 255, 255],
        }
    }

    fn rotated_rectangle_vertices(rectangle: &ColoredTestRectangle) -> [(i32, i32); 4] {
        let center_x = f64::midpoint(rectangle.top_left.x, rectangle.bottom_right.x);
        let center_y = f64::midpoint(rectangle.top_left.y, rectangle.bottom_right.y);
        let half_width = (rectangle.bottom_right.x - rectangle.top_left.x) / 2.0;
        let half_height = (rectangle.bottom_right.y - rectangle.top_left.y) / 2.0;
        let angle = rectangle.rotation_angle_degrees.to_radians();
        let cos_angle = angle.cos();
        let sin_angle = angle.sin();
        let corners = [
            (-half_width, -half_height),
            (half_width, -half_height),
            (half_width, half_height),
            (-half_width, half_height),
        ];

        corners.map(|(local_x, local_y)| {
            let rotated_x = center_x + local_x * cos_angle - local_y * sin_angle;
            let rotated_y = center_y + local_x * sin_angle + local_y * cos_angle;
            (i32_from_f64(rotated_x), i32_from_f64(rotated_y))
        })
    }

    fn fill_rotated_rectangle(image: &WrappedRgbImage, rectangle: &ColoredTestRectangle) {
        image.fill_polygon(
            &rotated_rectangle_vertices(rectangle),
            rgb_from_name(rectangle.color),
        );
    }

    fn fill_circle(image: &WrappedRgbImage, circle: &ColoredTestCircle) {
        image.fill_circle(
            (i32_from_f64(circle.center.x), i32_from_f64(circle.center.y)),
            i32_from_f64(circle.radius),
            rgb_from_name(circle.color),
        );
    }

    fn create_test_image_with_shapes(
        shapes_data: &ShapesData,
        width: u32,
        height: u32,
    ) -> WrappedRgbImage {
        let image = WrappedRgbImage::new_with_color(width, height, [0, 0, 0]);
        for rectangle in &shapes_data.rectangles {
            fill_rotated_rectangle(&image, rectangle);
        }
        for circle in &shapes_data.circles {
            fill_circle(&image, circle);
        }
        image
    }

    fn basic_params() -> BasicParams {
        BasicParams::new(false, 15)
    }

    fn surrounding_rectangle(image: &WrappedRgbImage) -> Rectangle {
        let width = f64::from(image.width());
        let height = f64::from(image.height());
        Rectangle::new(Vec3d::new(0.0, 0.0, 0.0), Vec3d::new(width, height, 0.0))
    }

    fn deduce_all_mosaics(image: &WrappedRgbImage) -> Vec<TracedRelativeMosaic> {
        let trace_params = TraceParams::new(12);
        let rectangle = surrounding_rectangle(image);
        let math_rectangle =
            MathRectangle::new(rectangle.get_top_left(), rectangle.get_bottom_right());
        let slices = calculate_slices(image, rectangle, basic_params());
        let connected_slices = find_connected_slices(&mut slices.clone());
        deduce_mosaics(connected_slices, &math_rectangle)
            .into_iter()
            .map(|mosaic| TracedRelativeMosaic::new(mosaic, trace_params))
            .collect()
    }

    fn deduce_mosaic_at_position(
        image: &WrappedRgbImage,
        position: Vec3d,
    ) -> Option<TracedRelativeMosaic> {
        let global_coordinate_system = AnonymizedCoordinateSystem::Direct(CoordinateSystem::new(
            Vec3d::new(0.0, 0.0, 0.0),
            Vec3d::new(1.0, 0.0, 0.0),
            Vec3d::new(0.0, 1.0, 0.0),
        ));
        deduce_all_mosaics(image).into_iter().find(|mosaic| {
            mosaic.get_relative_mosaic().get_mosaic().contains_point(
                &crate::math::CoordinatedPoint::new(global_coordinate_system.clone(), position),
            )
        })
    }

    fn single_reference_object_from_image(
        image: &WrappedRgbImage,
        position: Vec3d,
        id: &str,
    ) -> ReferenceObject {
        ReferenceObject::new(
            id.to_string(),
            vec![deduce_mosaic_at_position(image, position).unwrap()],
        )
    }

    #[test]
    fn reference_object_new_sorts_by_area_and_get_mosaics_clamps_requested_length() {
        let large_image = create_test_image_with_shapes(
            &ShapesData {
                rectangles: vec![ColoredTestRectangle {
                    top_left: Vec3d::new(5.0, 5.0, 0.0),
                    bottom_right: Vec3d::new(25.0, 25.0, 0.0),
                    color: "white",
                    rotation_angle_degrees: 0.0,
                }],
                circles: Vec::new(),
            },
            50,
            50,
        );
        let large = deduce_mosaic_at_position(&large_image, Vec3d::new(15.0, 15.0, 0.0)).unwrap();
        let medium_image = create_test_image_with_shapes(
            &ShapesData {
                rectangles: vec![ColoredTestRectangle {
                    top_left: Vec3d::new(5.0, 5.0, 0.0),
                    bottom_right: Vec3d::new(20.0, 20.0, 0.0),
                    color: "white",
                    rotation_angle_degrees: 0.0,
                }],
                circles: Vec::new(),
            },
            40,
            40,
        );
        let medium = deduce_mosaic_at_position(&medium_image, Vec3d::new(12.0, 12.0, 0.0)).unwrap();
        let small_image = create_test_image_with_shapes(
            &ShapesData {
                rectangles: vec![ColoredTestRectangle {
                    top_left: Vec3d::new(5.0, 5.0, 0.0),
                    bottom_right: Vec3d::new(15.0, 15.0, 0.0),
                    color: "white",
                    rotation_angle_degrees: 0.0,
                }],
                circles: Vec::new(),
            },
            30,
            30,
        );
        let small = deduce_mosaic_at_position(&small_image, Vec3d::new(10.0, 10.0, 0.0)).unwrap();

        let reference = ReferenceObject::new(
            "ordered".to_string(),
            vec![small.clone(), large.clone(), medium.clone()],
        );

        let ordered = reference.get_mosaics(usize::MAX);
        assert_eq!(ordered.len(), 3);
        assert!(
            ordered[0].get_relative_mosaic().get_area()
                >= ordered[1].get_relative_mosaic().get_area()
        );
        assert!(
            ordered[1].get_relative_mosaic().get_area()
                >= ordered[2].get_relative_mosaic().get_area()
        );
        assert_float_eq(
            reference.get_mosaics(1)[0].get_relative_mosaic().get_area(),
            large.get_relative_mosaic().get_area(),
        );
    }

    #[test]
    fn reference_object_methods_return_id_surrounding_box_and_relative_rectangle() {
        let image = create_test_image_with_shapes(
            &ShapesData {
                rectangles: vec![
                    ColoredTestRectangle {
                        top_left: Vec3d::new(5.0, 5.0, 0.0),
                        bottom_right: Vec3d::new(25.0, 25.0, 0.0),
                        color: "white",
                        rotation_angle_degrees: 0.0,
                    },
                    ColoredTestRectangle {
                        top_left: Vec3d::new(35.0, 10.0, 0.0),
                        bottom_right: Vec3d::new(45.0, 20.0, 0.0),
                        color: "white",
                        rotation_angle_degrees: 0.0,
                    },
                ],
                circles: Vec::new(),
            },
            60,
            40,
        );
        let large = deduce_mosaic_at_position(&image, Vec3d::new(15.0, 15.0, 0.0)).unwrap();
        let small = deduce_mosaic_at_position(&image, Vec3d::new(40.0, 15.0, 0.0)).unwrap();
        let reference = ReferenceObject::new("ref-id".to_string(), vec![small, large.clone()]);

        let surrounding = reference.get_surrounding_bounding_box();
        let relative = reference
            .get_relative_rectangle_to_smallest()
            .multiply_with_rectangle(Rectangle::new_from_math_rectangle(
                &large
                    .get_relative_mosaic()
                    .get_mosaic()
                    .get_bounding_box()
                    .to_global_rectangle(),
            ));

        assert_eq!(reference.get_id(), "ref-id".to_string());
        assert_vec_eq(surrounding.get_top_left(), Vec3d::new(7.0, 7.0, 0.0));
        assert_vec_eq(surrounding.get_bottom_right(), Vec3d::new(43.0, 23.0, 0.0));
        assert_vec_eq(relative.get_top_left(), Vec3d::new(37.0, 12.0, 0.0));
        assert_vec_eq(relative.get_bottom_right(), Vec3d::new(44.0, 19.0, 0.0));
    }

    #[test]
    #[should_panic(
        expected = "At least 2 mosaics are required to calculate the relative rectangle"
    )]
    fn relative_rectangle_to_smallest_panics_with_single_mosaic() {
        let reference = single_reference_object_from_image(
            &create_test_image_with_shapes(
                &ShapesData {
                    rectangles: vec![ColoredTestRectangle {
                        top_left: Vec3d::new(5.0, 5.0, 0.0),
                        bottom_right: Vec3d::new(25.0, 25.0, 0.0),
                        color: "white",
                        rotation_angle_degrees: 0.0,
                    }],
                    circles: Vec::new(),
                },
                50,
                50,
            ),
            Vec3d::new(15.0, 15.0, 0.0),
            "single",
        );

        let _ = reference.get_relative_rectangle_to_smallest();
    }

    #[test]
    fn object_detection_params_new_preserves_fields() {
        let params = ObjectDetectionParams::new(
            TileParams::new(0.25, 0.5),
            0.125,
            TraceParams::new(24),
            0.85,
        );

        assert_float_eq(params.tile_params.relative_tile_x(), 0.25);
        assert_float_eq(params.tile_params.relative_tile_y(), 0.5);
        assert_float_eq(params.bucket_delta, 0.125);
        assert_eq!(params.trace_params.num_skeleton(), 24);
        assert_float_eq(params.target_similarity, 0.85);
    }

    #[test]
    fn combine_boxes_returns_smallest_box_covering_all_inputs() {
        let combined = combine_boxes(vec![
            Rectangle::new(Vec3d::new(10.0, 20.0, 0.0), Vec3d::new(15.0, 25.0, 0.0)),
            Rectangle::new(Vec3d::new(5.0, 30.0, 0.0), Vec3d::new(8.0, 35.0, 0.0)),
            Rectangle::new(Vec3d::new(12.0, 18.0, 0.0), Vec3d::new(20.0, 40.0, 0.0)),
        ]);

        assert_vec_eq(combined.get_top_left(), Vec3d::new(5.0, 18.0, 0.0));
        assert_vec_eq(combined.get_bottom_right(), Vec3d::new(20.0, 40.0, 0.0));
    }
}

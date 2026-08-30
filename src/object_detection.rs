use core::panic;

use rs_math3d::Vec3d;

use crate::bucketed_mosaics::BucketedMosaics;
use crate::eye::TileParams;
use crate::math::Rectangle as MathRectangle;
use crate::mosaics::WrappedMosaic;
use crate::mosaics::WrappedRelativeMosaic;
use crate::slices::Color;
use crate::slices::RelativeRectangle;
use crate::slices::{ColoredRectangle, Rectangle, WrappedRelativeRectangle};
use crate::traces::Trace;
use crate::traces::TraceParams;

#[derive(Clone)]
pub struct ReferenceObject {
    object_id: String,
    mosaics: Vec<WrappedMosaic>,
}

impl ReferenceObject {
    pub fn new(object_id: String, mosaics: Vec<WrappedMosaic>) -> Self {
        let mut mosaics = mosaics;
        mosaics.sort_by(|a, b| {
            a.get_bounding_box()
                .to_global_rectangle()
                .get_area()
                .partial_cmp(&b.get_bounding_box().to_global_rectangle().get_area())
                .unwrap()
        });
        mosaics.reverse();
        ReferenceObject { object_id, mosaics }
    }

    pub fn get_mosaics(&self, until_index: usize) -> Vec<WrappedMosaic> {
        self.mosaics[..until_index.min(self.mosaics.len())].to_vec()
    }

    pub fn get_id(&self) -> String {
        self.object_id.clone()
    }

    pub fn get_surrounding_bounding_box(&self) -> Rectangle {
        let mut min_x = f64::INFINITY;
        let mut min_y = f64::INFINITY;
        let mut max_x = f64::NEG_INFINITY;
        let mut max_y = f64::NEG_INFINITY;

        for mosaic in &self.mosaics {
            let bounding_box = mosaic.get_bounding_box().to_global_rectangle();
            min_x = min_x.min(bounding_box.get_top_left().x);
            min_y = min_y.min(bounding_box.get_top_left().y);
            max_x = max_x.max(bounding_box.get_bottom_right().x);
            max_y = max_y.max(bounding_box.get_bottom_right().y);
        }
        Rectangle::new(Vec3d::new(min_x, min_y, 0.0), Vec3d::new(max_x, max_y, 0.0))
    }

    fn get_relative_rectangle_to_smallest(&self) -> RelativeRectangle {
        if self.mosaics.len() < 2 {
            panic!("At least 2 mosaics are required to calculate the relative rectangle");
        }
        let smallest_bounding_box = Rectangle::new_from_math_rectangle(
            self.mosaics
                .last()
                .unwrap()
                .get_bounding_box()
                .to_global_rectangle(),
        );
        let biggest_bounding_box = combine_boxes(
            self.mosaics[..self.mosaics.len() - 1]
                .iter()
                .map(|mosaic| {
                    Rectangle::new_from_math_rectangle(
                        mosaic.get_bounding_box().to_global_rectangle(),
                    )
                })
                .collect(),
        );
        RelativeRectangle::new_from_rectangles(smallest_bounding_box, biggest_bounding_box)
    }
}

#[derive(Clone)]
pub struct ObjectDetectionParams {
    pub tile_params: TileParams,
    pub bucket_delta: f64,
    pub trace_params: TraceParams,
    pub target_similarity: f64,
}

impl ObjectDetectionParams {
    pub fn new(
        tile_params: TileParams,
        bucket_delta: f64,
        trace_params: TraceParams,
        target_similarity: f64,
    ) -> Self {
        ObjectDetectionParams {
            tile_params,
            bucket_delta,
            trace_params,
            target_similarity,
        }
    }
}

pub fn detect_objects(
    reference_object: ReferenceObject,
    bucketed_mosaics: &BucketedMosaics,
    object_detection_params: ObjectDetectionParams,
    surrounding_rectangle: Rectangle,
) -> Vec<ColoredRectangle> {
    let surrounding_math_rectangle = MathRectangle::new(
        surrounding_rectangle.get_top_left(),
        surrounding_rectangle.get_bottom_right(),
    );
    let biggest_mosaic = reference_object.get_mosaics(1)[0].clone();
    let wrapped_biggest_mosaic =
        WrappedRelativeMosaic::new(biggest_mosaic.clone(), surrounding_math_rectangle.clone());
    let biggest_trace = Trace::new_from_mosaic(
        biggest_mosaic.clone(),
        object_detection_params.trace_params.clone(),
    );
    let biggest_candidates = bucketed_mosaics
        .get_all_similar_mosaics(&wrapped_biggest_mosaic)
        .into_iter()
        .map(|wrapped_relative_mosaic| wrapped_relative_mosaic.get_mosaic())
        .collect::<Vec<_>>();
    let cloned_trace_params = object_detection_params.trace_params.clone();
    let mut candidates = biggest_candidates
        .into_iter()
        .filter(|candidate| {
            let candidate_trace =
                Trace::new_from_mosaic(candidate.clone(), cloned_trace_params.clone());
            candidate_trace.compare_with(object_detection_params.target_similarity, &biggest_trace)
                >= object_detection_params.target_similarity
        })
        .map(|candidate| ReferenceObject::new("dummy_id".to_string(), vec![candidate]))
        .collect::<Vec<_>>();
    for i in 1..reference_object.get_mosaics(usize::MAX).len() {
        if candidates.is_empty() {
            break;
        }
        let current_reference_object =
            ReferenceObject::new("dummy_id".to_string(), reference_object.get_mosaics(i + 1));
        let relative_rectangle = current_reference_object.get_relative_rectangle_to_smallest();
        let inverted_relative_rectangle = relative_rectangle.invert();
        let mut new_candidate_reference_objects = Vec::new();
        let current_mosaic = current_reference_object.get_mosaics(i + 1)[i].clone();
        let wrapped_current_mosaic =
            WrappedRelativeMosaic::new(current_mosaic.clone(), surrounding_math_rectangle.clone());
        let current_trace = Trace::new_from_mosaic(
            current_mosaic.clone(),
            object_detection_params.trace_params.clone(),
        );
        for candidate in candidates {
            let absolute_rectangle = combine_boxes(
                candidate
                    .get_mosaics(usize::MAX)
                    .iter()
                    .map(|mosaic| {
                        Rectangle::new_from_math_rectangle(
                            mosaic.get_bounding_box().to_global_rectangle(),
                        )
                    })
                    .collect(),
            );
            let suspected_region =
                relative_rectangle.multiply_with_rectangle(absolute_rectangle.clone());
            let inverted_suspected_region =
                inverted_relative_rectangle.multiply_with_rectangle(absolute_rectangle.clone());
            let combined_regions = combine_boxes(vec![
                suspected_region,
                absolute_rectangle,
                inverted_suspected_region,
            ]);
            let relative_combined_region = WrappedRelativeRectangle::new_from_rectangles(
                combined_regions,
                surrounding_rectangle.clone(),
            );
            let next_mosaic_candidates = bucketed_mosaics
                .get_similar_mosaics_from_rectangle(
                    &wrapped_current_mosaic.clone(),
                    relative_combined_region,
                )
                .into_iter()
                .map(|wrapped_relative_mosaic| wrapped_relative_mosaic.get_mosaic())
                .collect::<Vec<_>>();
            let real_candidates: Vec<_> = next_mosaic_candidates
                .into_iter()
                .filter(|next_mosaic_candidate| {
                    let next_candidate_trace = Trace::new_from_mosaic(
                        next_mosaic_candidate.clone(),
                        object_detection_params.trace_params.clone(),
                    );
                    next_candidate_trace
                        .compare_with(object_detection_params.target_similarity, &current_trace)
                        >= object_detection_params.target_similarity
                })
                .collect();

            for real_candidate in real_candidates {
                let mut candidate_mosaics = candidate.get_mosaics(i + 1);
                candidate_mosaics.push(real_candidate);
                let current_candidate_reference_object =
                    ReferenceObject::new("dummy_id".to_string(), candidate_mosaics);
                let current_candidate_reference_object_trace = Trace::new_from_mosaics(
                    current_candidate_reference_object.get_mosaics(usize::MAX),
                    object_detection_params.trace_params.clone(),
                );
                if current_candidate_reference_object_trace.compare_with(
                    object_detection_params.target_similarity,
                    &Trace::new_from_mosaics(
                        current_reference_object.get_mosaics(usize::MAX),
                        object_detection_params.trace_params.clone(),
                    ),
                ) >= object_detection_params.target_similarity
                {
                    new_candidate_reference_objects.push(current_candidate_reference_object);
                }
            }
        }
        candidates = new_candidate_reference_objects;
    }
    candidates
        .into_iter()
        .map(|candidate| {
            let bounding_box = combine_boxes(
                candidate
                    .get_mosaics(usize::MAX)
                    .iter()
                    .map(|mosaic| {
                        Rectangle::new_from_math_rectangle(
                            mosaic.get_bounding_box().to_global_rectangle(),
                        )
                    })
                    .collect(),
            );
            ColoredRectangle::new(
                bounding_box,
                Color::Green,
                candidate.get_mosaics(usize::MAX),
            )
        })
        .collect()
}

fn combine_boxes(boxes: Vec<Rectangle>) -> Rectangle {
    let mut min_x = f64::INFINITY;
    let mut min_y = f64::INFINITY;
    let mut max_x = f64::NEG_INFINITY;
    let mut max_y = f64::NEG_INFINITY;

    for bounding_box in boxes {
        min_x = min_x.min(bounding_box.get_top_left().x);
        min_y = min_y.min(bounding_box.get_top_left().y);
        max_x = max_x.max(bounding_box.get_bottom_right().x);
        max_y = max_y.max(bounding_box.get_bottom_right().y);
    }
    Rectangle::new(Vec3d::new(min_x, min_y, 0.0), Vec3d::new(max_x, max_y, 0.0))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bucketed_mosaics::BucketedMosaics;
    use crate::eye::calculate_rectangles_of_bucketed_mosaics;
    use crate::mosaics::{deduce_mosaics, WrappedMosaic};
    use crate::slices::{calculate_slices, find_connected_slices, BasicParams, WrappedRgbImage};
    use image::{ImageBuffer, Rgb};
    use imageproc::drawing::{draw_filled_circle_mut, draw_polygon_mut};
    use imageproc::point::Point;

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

    fn rgb_from_name(color: &str) -> Rgb<u8> {
        match color {
            "red" => Rgb([255, 0, 0]),
            "green" => Rgb([0, 255, 0]),
            "blue" => Rgb([0, 0, 255]),
            _ => Rgb([255, 255, 255]),
        }
    }

    fn rotated_rectangle_vertices(rectangle: &ColoredTestRectangle) -> [Point<i32>; 4] {
        let center_x = (rectangle.top_left.x + rectangle.bottom_right.x) / 2.0;
        let center_y = (rectangle.top_left.y + rectangle.bottom_right.y) / 2.0;
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
            Point::new(rotated_x.round() as i32, rotated_y.round() as i32)
        })
    }

    fn fill_rotated_rectangle(
        image: &mut ImageBuffer<Rgb<u8>, Vec<u8>>,
        rectangle: &ColoredTestRectangle,
    ) {
        draw_polygon_mut(
            image,
            &rotated_rectangle_vertices(rectangle),
            rgb_from_name(rectangle.color),
        );
    }

    fn fill_circle(image: &mut ImageBuffer<Rgb<u8>, Vec<u8>>, circle: &ColoredTestCircle) {
        draw_filled_circle_mut(
            image,
            (
                circle.center.x.round() as i32,
                circle.center.y.round() as i32,
            ),
            circle.radius.round() as i32,
            rgb_from_name(circle.color),
        );
    }

    fn create_test_image_with_shapes(shapes_data: &ShapesData, width: u32, height: u32) -> WrappedRgbImage {
        let mut image = ImageBuffer::from_pixel(width, height, Rgb([0, 0, 0]));
        for rectangle in &shapes_data.rectangles {
            fill_rotated_rectangle(&mut image, rectangle);
        }
        for circle in &shapes_data.circles {
            fill_circle(&mut image, circle);
        }
        WrappedRgbImage::new(image)
    }

    fn generate_shape_data() -> ShapesData {
        let mut shapes_data = ShapesData::default();
        shapes_data.rectangles.push(ColoredTestRectangle {
            top_left: Vec3d::new(5.0, 5.0, 0.0),
            bottom_right: Vec3d::new(25.0, 25.0, 0.0),
            color: "red",
            rotation_angle_degrees: 0.0,
        });
        shapes_data.rectangles.push(ColoredTestRectangle {
            top_left: Vec3d::new(35.0, 5.0, 0.0),
            bottom_right: Vec3d::new(55.0, 25.0, 0.0),
            color: "green",
            rotation_angle_degrees: 30.0,
        });
        shapes_data.rectangles.push(ColoredTestRectangle {
            top_left: Vec3d::new(65.0, 5.0, 0.0),
            bottom_right: Vec3d::new(85.0, 25.0, 0.0),
            color: "blue",
            rotation_angle_degrees: 60.0,
        });
        shapes_data.rectangles.push(ColoredTestRectangle {
            top_left: Vec3d::new(95.0, 5.0, 0.0),
            bottom_right: Vec3d::new(125.0, 35.0, 0.0),
            color: "white",
            rotation_angle_degrees: 90.0,
        });
        shapes_data.circles.push(ColoredTestCircle {
            center: Vec3d::new(20.0, 55.0, 0.0),
            radius: 15.0,
            color: "red",
        });
        shapes_data.circles.push(ColoredTestCircle {
            center: Vec3d::new(60.0, 55.0, 0.0),
            radius: 15.0,
            color: "green",
        });
        shapes_data.circles.push(ColoredTestCircle {
            center: Vec3d::new(100.0, 55.0, 0.0),
            radius: 15.0,
            color: "blue",
        });
        shapes_data.circles.push(ColoredTestCircle {
            center: Vec3d::new(140.0, 55.0, 0.0),
            radius: 20.0,
            color: "white",
        });
        shapes_data.circles.push(ColoredTestCircle {
            center: Vec3d::new(200.0, 55.0, 0.0),
            radius: 25.0,
            color: "black",
        });
        shapes_data.rectangles.push(ColoredTestRectangle {
            top_left: Vec3d::new(5.0, 85.0, 0.0),
            bottom_right: Vec3d::new(15.0, 105.0, 0.0),
            color: "red",
            rotation_angle_degrees: 0.0,
        });
        shapes_data.rectangles.push(ColoredTestRectangle {
            top_left: Vec3d::new(25.0, 85.0, 0.0),
            bottom_right: Vec3d::new(35.0, 105.0, 0.0),
            color: "green",
            rotation_angle_degrees: 30.0,
        });
        shapes_data.rectangles.push(ColoredTestRectangle {
            top_left: Vec3d::new(55.0, 85.0, 0.0),
            bottom_right: Vec3d::new(75.0, 125.0, 0.0),
            color: "blue",
            rotation_angle_degrees: 60.0,
        });
        shapes_data
    }

    fn basic_params() -> BasicParams {
        BasicParams::new(false, 15)
    }

    fn surrounding_rectangle(image: &WrappedRgbImage) -> Rectangle {
        let width = image.image.lock().unwrap().width() as f64;
        let height = image.image.lock().unwrap().height() as f64;
        Rectangle::new(Vec3d::new(0.0, 0.0, 0.0), Vec3d::new(width, height, 0.0))
    }

    fn deduce_all_mosaics(image: WrappedRgbImage) -> Vec<WrappedMosaic> {
        let rectangle = surrounding_rectangle(&image);
        let slices = calculate_slices(image.clone(), rectangle, basic_params());
        let connected_slices = find_connected_slices(&mut slices.clone());
        deduce_mosaics(connected_slices)
    }

    fn deduce_mosaic_at_position(
        image: WrappedRgbImage,
        position: Vec3d,
    ) -> Option<WrappedMosaic> {
        deduce_all_mosaics(image).into_iter().find(|mosaic| {
            mosaic.contains_point(crate::math::CoordinatedPoint::new(
                crate::math::WrappedCoordinateSystem::new(
                    Vec3d::new(0.0, 0.0, 0.0),
                    Vec3d::new(1.0, 0.0, 0.0),
                    Vec3d::new(0.0, 1.0, 0.0),
                ),
                position,
            ))
        })
    }

    fn build_bucketed_mosaics(
        image: WrappedRgbImage,
        tile_params: TileParams,
        bucket_delta: f64,
    ) -> BucketedMosaics {
        let surrounding = surrounding_rectangle(&image);
        let surrounding_math = MathRectangle::new(
            surrounding.get_top_left(),
            surrounding.get_bottom_right(),
        );
        let regions = calculate_rectangles_of_bucketed_mosaics(tile_params.clone());
        let mosaics = deduce_all_mosaics(image);
        let mut bucketed = BucketedMosaics::new(regions, bucket_delta);
        for mosaic in mosaics {
            bucketed.add_mosaic(WrappedRelativeMosaic::new(
                mosaic,
                surrounding_math.clone(),
            ));
        }
        bucketed
    }

    fn extract_center_y(rectangle: &Rectangle) -> f64 {
        (rectangle.get_top_left().y + rectangle.get_bottom_right().y) / 2.0
    }

    fn assert_all_green(results: &[ColoredRectangle]) {
        assert!(!results.is_empty());
        for result in results {
            assert!(result.get_color() == Color::Green);
            assert!(!result.get_mosaics().is_empty());
        }
    }

    fn standard_detection_params(target_similarity: f64) -> ObjectDetectionParams {
        ObjectDetectionParams::new(
            TileParams::new(0.2, 0.2),
            0.5,
            TraceParams::new(36, 0.2),
            target_similarity,
        )
    }

    fn trace_cpp_square_reference_object() -> ReferenceObject {
        let reference_image = create_test_image_with_shapes(
            &ShapesData {
                rectangles: vec![ColoredTestRectangle {
                    top_left: Vec3d::new(15.0, 15.0, 0.0),
                    bottom_right: Vec3d::new(35.0, 35.0, 0.0),
                    color: "red",
                    rotation_angle_degrees: 0.0,
                }],
                circles: Vec::new(),
            },
            50,
            50,
        );

        single_reference_object_from_image(
            reference_image,
            Vec3d::new(20.0, 20.0, 0.0),
            "square",
        )
    }

    fn trace_cpp_circle_reference_object() -> ReferenceObject {
        let reference_image = create_test_image_with_shapes(
            &ShapesData {
                rectangles: Vec::new(),
                circles: vec![ColoredTestCircle {
                    center: Vec3d::new(25.0, 25.0, 0.0),
                    radius: 25.0,
                    color: "red",
                }],
            },
            50,
            50,
        );

        single_reference_object_from_image(
            reference_image,
            Vec3d::new(25.0, 25.0, 0.0),
            "circle",
        )
    }

    fn trace_cpp_rectangle_reference_object() -> ReferenceObject {
        let reference_image = create_test_image_with_shapes(
            &ShapesData {
                rectangles: vec![ColoredTestRectangle {
                    top_left: Vec3d::new(15.0, 15.0, 0.0),
                    bottom_right: Vec3d::new(25.0, 35.0, 0.0),
                    color: "red",
                    rotation_angle_degrees: 0.0,
                }],
                circles: Vec::new(),
            },
            50,
            50,
        );

        single_reference_object_from_image(
            reference_image,
            Vec3d::new(20.0, 20.0, 0.0),
            "rectangle",
        )
    }

    fn single_reference_object_from_image(
        image: WrappedRgbImage,
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
        let large = deduce_mosaic_at_position(
            create_test_image_with_shapes(
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
        )
        .unwrap();
        let medium = deduce_mosaic_at_position(
            create_test_image_with_shapes(
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
            ),
            Vec3d::new(12.0, 12.0, 0.0),
        )
        .unwrap();
        let small = deduce_mosaic_at_position(
            create_test_image_with_shapes(
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
            ),
            Vec3d::new(10.0, 10.0, 0.0),
        )
        .unwrap();

        let reference = ReferenceObject::new(
            "ordered".to_string(),
            vec![small.clone(), large.clone(), medium.clone()],
        );

        let ordered = reference.get_mosaics(usize::MAX);
        assert_eq!(ordered.len(), 3);
        assert!(ordered[0].get_area() >= ordered[1].get_area());
        assert!(ordered[1].get_area() >= ordered[2].get_area());
        assert_float_eq(reference.get_mosaics(1)[0].get_area(), large.get_area());
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
        let large = deduce_mosaic_at_position(image.clone(), Vec3d::new(15.0, 15.0, 0.0)).unwrap();
        let small = deduce_mosaic_at_position(image, Vec3d::new(40.0, 15.0, 0.0)).unwrap();
        let reference = ReferenceObject::new("ref-id".to_string(), vec![small, large.clone()]);

        let surrounding = reference.get_surrounding_bounding_box();
        let relative = reference
            .get_relative_rectangle_to_smallest()
            .multiply_with_rectangle(Rectangle::new_from_math_rectangle(
                large.get_bounding_box().to_global_rectangle(),
            ));

        assert_eq!(reference.get_id(), "ref-id".to_string());
        assert_vec_eq(surrounding.get_top_left(), Vec3d::new(7.0, 7.0, 0.0));
        assert_vec_eq(surrounding.get_bottom_right(), Vec3d::new(43.0, 23.0, 0.0));
        assert_vec_eq(relative.get_top_left(), Vec3d::new(35.0, 10.0, 0.0));
        assert_vec_eq(relative.get_bottom_right(), Vec3d::new(43.0, 18.0, 0.0));
    }

    #[test]
    #[should_panic(expected = "At least 2 mosaics are required to calculate the relative rectangle")]
    fn relative_rectangle_to_smallest_panics_with_single_mosaic() {
        let reference = single_reference_object_from_image(
            create_test_image_with_shapes(
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
            TraceParams::new(24, 0.2),
            0.85,
        );

        assert_float_eq(params.tile_params.relative_tile_x(), 0.25);
        assert_float_eq(params.tile_params.relative_tile_y(), 0.5);
        assert_float_eq(params.bucket_delta, 0.125);
        assert_eq!(params.trace_params.num_skeleton(), 24);
        assert_float_eq(params.trace_params.close_slice_threshold(), 0.2);
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

    #[test]
    fn detect_objects_finds_square_results_from_trace_cpp_scene() {
        let scene = create_test_image_with_shapes(&generate_shape_data(), 300, 300);
        let reference = trace_cpp_square_reference_object();
        let bucketed = build_bucketed_mosaics(scene.clone(), TileParams::new(0.2, 0.2), 0.5);
        let results = detect_objects(
            reference,
            &bucketed,
            standard_detection_params(0.9),
            surrounding_rectangle(&scene),
        );

        assert_eq!(results.len(), 4);
        assert_all_green(&results);
        for result in &results {
            let center_y = extract_center_y(&result.get_rectangle());
            assert!((5.0..=35.0).contains(&center_y));
        }
    }

    #[test]
    fn detect_objects_finds_circle_results_from_trace_cpp_scene() {
        let scene = create_test_image_with_shapes(&generate_shape_data(), 300, 300);
        let reference = trace_cpp_circle_reference_object();
        let bucketed = build_bucketed_mosaics(scene.clone(), TileParams::new(0.2, 0.2), 0.5);
        let results = detect_objects(
            reference,
            &bucketed,
            standard_detection_params(0.9),
            surrounding_rectangle(&scene),
        );

        assert_eq!(results.len(), 5);
        assert_all_green(&results);
        for result in &results {
            let center_y = extract_center_y(&result.get_rectangle());
            assert!((45.0..=85.0).contains(&center_y));
        }
    }

    #[test]
    fn detect_objects_finds_rectangle_results_from_trace_cpp_scene() {
        let scene = create_test_image_with_shapes(&generate_shape_data(), 300, 300);
        let reference = trace_cpp_rectangle_reference_object();
        let bucketed = build_bucketed_mosaics(scene.clone(), TileParams::new(0.2, 0.2), 0.5);
        let results = detect_objects(
            reference,
            &bucketed,
            standard_detection_params(0.9),
            surrounding_rectangle(&scene),
        );

        assert_eq!(results.len(), 3);
        assert_all_green(&results);
        for result in &results {
            let center_y = extract_center_y(&result.get_rectangle());
            assert!((85.0..=115.0).contains(&center_y));
        }
    }

    #[test]
    fn detect_objects_with_two_reference_mosaics_respects_relative_layout() {
        let reference_image = create_test_image_with_shapes(
            &ShapesData {
                rectangles: vec![
                    ColoredTestRectangle {
                        top_left: Vec3d::new(10.0, 10.0, 0.0),
                        bottom_right: Vec3d::new(30.0, 30.0, 0.0),
                        color: "white",
                        rotation_angle_degrees: 0.0,
                    },
                    ColoredTestRectangle {
                        top_left: Vec3d::new(50.0, 10.0, 0.0),
                        bottom_right: Vec3d::new(70.0, 30.0, 0.0),
                        color: "white",
                        rotation_angle_degrees: 0.0,
                    },
                ],
                circles: Vec::new(),
            },
            80,
            50,
        );
        let reference = ReferenceObject::new(
            "pair".to_string(),
            vec![
                deduce_mosaic_at_position(reference_image.clone(), Vec3d::new(20.0, 20.0, 0.0)).unwrap(),
                deduce_mosaic_at_position(reference_image, Vec3d::new(60.0, 20.0, 0.0)).unwrap(),
            ],
        );
        let scene = create_test_image_with_shapes(
            &ShapesData {
                rectangles: vec![
                    ColoredTestRectangle {
                        top_left: Vec3d::new(10.0, 10.0, 0.0),
                        bottom_right: Vec3d::new(34.0, 34.0, 0.0),
                        color: "white",
                        rotation_angle_degrees: 0.0,
                    },
                    ColoredTestRectangle {
                        top_left: Vec3d::new(58.0, 14.0, 0.0),
                        bottom_right: Vec3d::new(70.0, 26.0, 0.0),
                        color: "white",
                        rotation_angle_degrees: 0.0,
                    },
                    ColoredTestRectangle {
                        top_left: Vec3d::new(100.0, 10.0, 0.0),
                        bottom_right: Vec3d::new(124.0, 34.0, 0.0),
                        color: "white",
                        rotation_angle_degrees: 0.0,
                    },
                    ColoredTestRectangle {
                        top_left: Vec3d::new(148.0, 14.0, 0.0),
                        bottom_right: Vec3d::new(160.0, 26.0, 0.0),
                        color: "white",
                        rotation_angle_degrees: 0.0,
                    },
                    ColoredTestRectangle {
                        top_left: Vec3d::new(100.0, 50.0, 0.0),
                        bottom_right: Vec3d::new(116.0, 66.0, 0.0),
                        color: "white",
                        rotation_angle_degrees: 0.0,
                    },
                ],
                circles: Vec::new(),
            },
            180,
            100,
        );
        let bucketed = build_bucketed_mosaics(scene.clone(), TileParams::new(0.25, 0.25), 0.5);
        let results = detect_objects(
            reference,
            &bucketed,
            ObjectDetectionParams::new(
                TileParams::new(0.25, 0.25),
                0.5,
                TraceParams::new(24, 0.2),
                0.86,
            ),
            surrounding_rectangle(&scene),
        );

        assert_eq!(results.len(), 2);
        assert_all_green(&results);
        let mut centers: Vec<f64> = results
            .iter()
            .map(|result| extract_center_y(&result.get_rectangle()))
            .collect();
        centers.sort_by(|left, right| left.partial_cmp(right).unwrap());
        assert_float_eq(centers[0], 19.0);
        assert_float_eq(centers[1], 19.0);
    }
}

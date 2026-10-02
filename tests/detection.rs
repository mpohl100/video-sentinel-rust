use image::{ImageBuffer, Rgb};
use imageproc::drawing::{draw_filled_circle_mut, draw_polygon_mut};
use imageproc::point::Point;
use rs_math3d::Vec3d;
use video_sentinel::bucketed_mosaics::BucketedMosaics;
use video_sentinel::eye::TileParams;
use video_sentinel::math::AnonymizedCoordinateSystem;
use video_sentinel::math::CoordinateSystem;
use video_sentinel::math::CoordinatedPoint;
use video_sentinel::math::Rectangle as MathRectangle;
use video_sentinel::mosaics::{Results, deduce_mosaics};
use video_sentinel::object_detection::{ObjectDetectionParams, ReferenceObject, detect_objects};
use video_sentinel::slices::{
    BasicParams, Color, ColoredRectangle, Rectangle, WrappedRgbImage, calculate_slices,
    find_connected_slices,
};
use video_sentinel::traced_mosaics::TracedRelativeMosaic;
use video_sentinel::traces::TraceParams;

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

fn create_test_image_with_shapes(
    shapes_data: &ShapesData,
    width: u32,
    height: u32,
) -> WrappedRgbImage {
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
        color: "green",
        rotation_angle_degrees: 0.0,
    });
    shapes_data.rectangles.push(ColoredTestRectangle {
        top_left: Vec3d::new(35.0, 5.0, 0.0),
        bottom_right: Vec3d::new(55.0, 25.0, 0.0),
        color: "green",
        rotation_angle_degrees: 0.0,
    });
    shapes_data.rectangles.push(ColoredTestRectangle {
        top_left: Vec3d::new(65.0, 5.0, 0.0),
        bottom_right: Vec3d::new(85.0, 25.0, 0.0),
        color: "green",
        rotation_angle_degrees: 0.0,
    });
    shapes_data.rectangles.push(ColoredTestRectangle {
        top_left: Vec3d::new(95.0, 5.0, 0.0),
        bottom_right: Vec3d::new(125.0, 35.0, 0.0),
        color: "green",
        rotation_angle_degrees: 0.0,
    });
    shapes_data.circles.push(ColoredTestCircle {
        center: Vec3d::new(20.0, 55.0, 0.0),
        radius: 15.0,
        color: "red",
    });
    shapes_data.circles.push(ColoredTestCircle {
        center: Vec3d::new(60.0, 55.0, 0.0),
        radius: 15.0,
        color: "red",
    });
    shapes_data.circles.push(ColoredTestCircle {
        center: Vec3d::new(100.0, 55.0, 0.0),
        radius: 15.0,
        color: "red",
    });
    shapes_data.circles.push(ColoredTestCircle {
        center: Vec3d::new(140.0, 55.0, 0.0),
        radius: 20.0,
        color: "red",
    });
    shapes_data.circles.push(ColoredTestCircle {
        center: Vec3d::new(200.0, 55.0, 0.0),
        radius: 25.0,
        color: "red",
    });
    shapes_data.rectangles.push(ColoredTestRectangle {
        top_left: Vec3d::new(5.0, 85.0, 0.0),
        bottom_right: Vec3d::new(15.0, 105.0, 0.0),
        color: "blue",
        rotation_angle_degrees: 0.0,
    });
    shapes_data.rectangles.push(ColoredTestRectangle {
        top_left: Vec3d::new(25.0, 85.0, 0.0),
        bottom_right: Vec3d::new(35.0, 105.0, 0.0),
        color: "blue",
        rotation_angle_degrees: 0.0,
    });
    shapes_data.rectangles.push(ColoredTestRectangle {
        top_left: Vec3d::new(55.0, 85.0, 0.0),
        bottom_right: Vec3d::new(75.0, 125.0, 0.0),
        color: "blue",
        rotation_angle_degrees: 60.0,
    });
    shapes_data
}

fn basic_params(do_grayscale: bool) -> BasicParams {
    BasicParams::new(do_grayscale, 15)
}

fn surrounding_rectangle(image: &WrappedRgbImage) -> Rectangle {
    let width = image.image.lock().unwrap().width() as f64;
    let height = image.image.lock().unwrap().height() as f64;
    Rectangle::new(Vec3d::new(0.0, 0.0, 0.0), Vec3d::new(width, height, 0.0))
}

fn deduce_all_mosaics(image: WrappedRgbImage, do_grayscale: bool) -> Vec<TracedRelativeMosaic> {
    let trace_params = TraceParams::new(12, 0.2);
    let rectangle = surrounding_rectangle(&image);
    let math_rectangle = MathRectangle::new(rectangle.get_top_left(), rectangle.get_bottom_right());
    let slices = calculate_slices(image.clone(), rectangle, basic_params(do_grayscale));
    let connected_slices = find_connected_slices(&mut slices.clone());
    deduce_mosaics(connected_slices, math_rectangle)
        .into_iter()
        .map(|mosaic| TracedRelativeMosaic::new(mosaic, trace_params.clone()))
        .collect()
}

fn deduce_mosaic_at_position(
    image: WrappedRgbImage,
    position: Vec3d,
    do_grayscale: bool,
) -> Option<TracedRelativeMosaic> {
    let global_coordinate_system = AnonymizedCoordinateSystem::Direct(CoordinateSystem::new(
        Vec3d::new(0.0, 0.0, 0.0),
        Vec3d::new(1.0, 0.0, 0.0),
        Vec3d::new(0.0, 1.0, 0.0),
    ));

    deduce_all_mosaics(image, do_grayscale)
        .into_iter()
        .find(|mosaic| {
            mosaic
                .get_relative_mosaic()
                .get_mosaic()
                .contains_point(CoordinatedPoint::new(
                    global_coordinate_system.clone(),
                    position,
                ))
        })
}

fn build_bucketed_mosaics(
    image: WrappedRgbImage,
    tile_params: TileParams,
    bucket_delta: f64,
    do_grayscale: bool,
) -> BucketedMosaics {
    let mut bucketed = BucketedMosaics::new(tile_params, bucket_delta);
    for mosaic in deduce_all_mosaics(image, do_grayscale) {
        bucketed.add_mosaic(mosaic);
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
        0.1,
        TraceParams::new(36, 0.2),
        target_similarity,
    )
}

fn single_reference_object_from_image(
    image: WrappedRgbImage,
    position: Vec3d,
    id: &str,
    do_grayscale: bool,
) -> ReferenceObject {
    ReferenceObject::new(
        id.to_string(),
        vec![deduce_mosaic_at_position(image, position, do_grayscale).unwrap()],
    )
}

fn trace_cpp_square_reference_object(do_grayscale: bool) -> ReferenceObject {
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
        do_grayscale,
    )
}

fn trace_cpp_circle_reference_object(do_grayscale: bool) -> ReferenceObject {
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
        do_grayscale,
    )
}

fn trace_cpp_rectangle_reference_object(do_grayscale: bool) -> ReferenceObject {
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
        do_grayscale,
    )
}

fn assert_detect_objects_finds_square_results_from_trace_cpp_scene(do_grayscale: bool) {
    let scene = create_test_image_with_shapes(&generate_shape_data(), 300, 300);
    let reference = trace_cpp_square_reference_object(do_grayscale);
    let bucketed =
        build_bucketed_mosaics(scene.clone(), TileParams::new(0.2, 0.2), 0.5, do_grayscale);
    let results = detect_objects(
        reference,
        &bucketed,
        standard_detection_params(0.8),
        surrounding_rectangle(&scene),
        Results::Absolute,
    );

    assert_eq!(results.len(), 4);
    assert_all_green(&results);
    for result in &results {
        let center_y = extract_center_y(&result.get_rectangle());
        assert!((5.0..=35.0).contains(&center_y));
    }
}

fn assert_detect_objects_finds_circle_results_from_trace_cpp_scene(do_grayscale: bool) {
    let scene = create_test_image_with_shapes(&generate_shape_data(), 300, 300);
    let reference = trace_cpp_circle_reference_object(do_grayscale);
    let bucketed =
        build_bucketed_mosaics(scene.clone(), TileParams::new(0.2, 0.2), 0.5, do_grayscale);
    let results = detect_objects(
        reference,
        &bucketed,
        standard_detection_params(0.8),
        surrounding_rectangle(&scene),
        Results::Absolute,
    );

    assert_eq!(results.len(), 5);
    assert_all_green(&results);
    for result in &results {
        let center_y = extract_center_y(&result.get_rectangle());
        assert!((45.0..=85.0).contains(&center_y));
    }
}

fn assert_detect_objects_finds_rectangle_results_from_trace_cpp_scene(do_grayscale: bool) {
    let scene = create_test_image_with_shapes(&generate_shape_data(), 300, 300);
    let reference = trace_cpp_rectangle_reference_object(do_grayscale);
    let bucketed =
        build_bucketed_mosaics(scene.clone(), TileParams::new(0.2, 0.2), 0.5, do_grayscale);
    let results = detect_objects(
        reference,
        &bucketed,
        standard_detection_params(0.8),
        surrounding_rectangle(&scene),
        Results::Absolute,
    );

    assert_eq!(results.len(), if do_grayscale { 3 } else { 2 });
    assert_all_green(&results);
    for result in &results {
        let center_y = extract_center_y(&result.get_rectangle());
        assert!((85.0..=115.0).contains(&center_y));
    }
}

fn assert_detect_objects_with_two_reference_mosaics_respects_relative_layout(do_grayscale: bool) {
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
            deduce_mosaic_at_position(
                reference_image.clone(),
                Vec3d::new(20.0, 20.0, 0.0),
                do_grayscale,
            )
            .unwrap(),
            deduce_mosaic_at_position(reference_image, Vec3d::new(60.0, 20.0, 0.0), do_grayscale)
                .unwrap(),
        ],
    );
    let scene = create_test_image_with_shapes(
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
                ColoredTestRectangle {
                    top_left: Vec3d::new(100.0, 10.0, 0.0),
                    bottom_right: Vec3d::new(120.0, 30.0, 0.0),
                    color: "white",
                    rotation_angle_degrees: 0.0,
                },
                ColoredTestRectangle {
                    top_left: Vec3d::new(140.0, 10.0, 0.0),
                    bottom_right: Vec3d::new(160.0, 30.0, 0.0),
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
    let bucketed = build_bucketed_mosaics(
        scene.clone(),
        TileParams::new(0.25, 0.25),
        0.5,
        do_grayscale,
    );
    let results = detect_objects(
        reference,
        &bucketed,
        ObjectDetectionParams::new(
            TileParams::new(0.25, 0.25),
            0.5,
            TraceParams::new(24, 0.2),
            0.8,
        ),
        surrounding_rectangle(&scene),
        Results::Absolute,
    );

    assert_eq!(results.len(), 1);
    assert_all_green(&results);
    let mut centers: Vec<f64> = results
        .iter()
        .map(|result| extract_center_y(&result.get_rectangle()))
        .collect();
    centers.sort_by(|left, right| left.partial_cmp(right).unwrap());
    assert_float_eq(centers[0], 20.0);
    assert_float_eq(centers[centers.len() - 1], 20.0);
}

macro_rules! grayscale_theory {
    ($test_name:ident, $assertion:ident, $do_grayscale:expr) => {
        #[test]
        fn $test_name() {
            $assertion($do_grayscale);
        }
    };
}

grayscale_theory!(
    detect_objects_finds_square_results_from_trace_cpp_scene_without_grayscale,
    assert_detect_objects_finds_square_results_from_trace_cpp_scene,
    false
);
grayscale_theory!(
    detect_objects_finds_square_results_from_trace_cpp_scene_with_grayscale,
    assert_detect_objects_finds_square_results_from_trace_cpp_scene,
    true
);
grayscale_theory!(
    detect_objects_finds_circle_results_from_trace_cpp_scene_without_grayscale,
    assert_detect_objects_finds_circle_results_from_trace_cpp_scene,
    false
);
grayscale_theory!(
    detect_objects_finds_circle_results_from_trace_cpp_scene_with_grayscale,
    assert_detect_objects_finds_circle_results_from_trace_cpp_scene,
    true
);
grayscale_theory!(
    detect_objects_finds_rectangle_results_from_trace_cpp_scene_without_grayscale,
    assert_detect_objects_finds_rectangle_results_from_trace_cpp_scene,
    false
);
grayscale_theory!(
    detect_objects_finds_rectangle_results_from_trace_cpp_scene_with_grayscale,
    assert_detect_objects_finds_rectangle_results_from_trace_cpp_scene,
    true
);
grayscale_theory!(
    detect_objects_with_two_reference_mosaics_respects_relative_layout_without_grayscale,
    assert_detect_objects_with_two_reference_mosaics_respects_relative_layout,
    false
);
grayscale_theory!(
    detect_objects_with_two_reference_mosaics_respects_relative_layout_with_grayscale,
    assert_detect_objects_with_two_reference_mosaics_respects_relative_layout,
    true
);

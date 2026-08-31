use image::{ImageBuffer, Rgb};
use imageproc::drawing::{draw_filled_circle_mut, draw_polygon_mut};
use imageproc::point::Point;
use rs_math3d::Vec3d;

use video_sentinel::math::Rectangle as MathRectangle;
use video_sentinel::mosaics::{WrappedMosaic, deduce_mosaics};
use video_sentinel::object_detection::ReferenceObject;
use video_sentinel::slices::{BasicParams, Rectangle, WrappedRgbImage, calculate_slices, find_connected_slices};
use video_sentinel::traces::{Trace, TraceParams};

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

fn deduce_mosaic_at_position(image: WrappedRgbImage, position: Vec3d) -> Option<WrappedMosaic> {
    deduce_all_mosaics(image).into_iter().find(|mosaic| {
        mosaic.contains_point(video_sentinel::math::CoordinatedPoint::new(
            video_sentinel::math::WrappedCoordinateSystem::new(
                Vec3d::new(0.0, 0.0, 0.0),
                Vec3d::new(1.0, 0.0, 0.0),
                Vec3d::new(0.0, 1.0, 0.0),
            ),
            position,
        ))
    })
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

    single_reference_object_from_image(reference_image, Vec3d::new(20.0, 20.0, 0.0), "square")
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

    single_reference_object_from_image(reference_image, Vec3d::new(25.0, 25.0, 0.0), "circle")
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

    single_reference_object_from_image(reference_image, Vec3d::new(20.0, 20.0, 0.0), "rectangle")
}

fn reference_object_methods_reference_object() -> ReferenceObject {
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
    ReferenceObject::new("ref-id".to_string(), vec![small, large])
}

fn pair_reference_object() -> ReferenceObject {
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

    ReferenceObject::new(
        "pair".to_string(),
        vec![
            deduce_mosaic_at_position(reference_image.clone(), Vec3d::new(20.0, 20.0, 0.0))
                .unwrap(),
            deduce_mosaic_at_position(reference_image, Vec3d::new(60.0, 20.0, 0.0)).unwrap(),
        ],
    )
}

fn print_reference_object_trace(
    name: &str,
    shape_description: &str,
    reference_object: ReferenceObject,
    params: TraceParams,
) {
    println!("=== {name} ===");
    println!("reference id: {}", reference_object.get_id());
    println!("reference shape: {shape_description}");
    println!("mosaic count: {}", reference_object.get_mosaics(usize::MAX).len());

    for (index, mosaic) in reference_object
        .get_mosaics(usize::MAX)
        .iter()
        .enumerate()
    {
        let bounding_box = mosaic.get_bounding_box().to_global_rectangle();
        let center = mosaic.get_center_of_mass();
        println!(
            "mosaic[{index}] area={:.8} bbox=(({:.8}, {:.8}), ({:.8}, {:.8})) center=({:.8}, {:.8}, {:.8})",
            mosaic.get_area(),
            bounding_box.get_top_left().x,
            bounding_box.get_top_left().y,
            bounding_box.get_bottom_right().x,
            bounding_box.get_bottom_right().y,
            center.get_x(),
            center.get_y(),
            center.get_z(),
        );
    }

    let trace = Trace::new_from_mosaics(reference_object.get_mosaics(usize::MAX), params);
    println!("{}", trace.dump_details());
}

fn main() {
    let _ = MathRectangle::new(Vec3d::new(0.0, 0.0, 0.0), Vec3d::new(1.0, 1.0, 0.0));

    print_reference_object_trace(
        "reference_object_methods_return_id_surrounding_box_and_relative_rectangle",
        "Square(10, 10) with Square(20, 20)",
        reference_object_methods_reference_object(),
        TraceParams::new(36, 0.2),
    );
    print_reference_object_trace(
        "detect_objects_finds_square_results_from_trace_cpp_scene",
        "Square(20, 20)",
        trace_cpp_square_reference_object(),
        TraceParams::new(36, 0.2),
    );
    print_reference_object_trace(
        "detect_objects_finds_circle_results_from_trace_cpp_scene",
        "Circle(radius=25)",
        trace_cpp_circle_reference_object(),
        TraceParams::new(36, 0.2),
    );
    print_reference_object_trace(
        "detect_objects_finds_rectangle_results_from_trace_cpp_scene",
        "Rectangle(10, 20)",
        trace_cpp_rectangle_reference_object(),
        TraceParams::new(36, 0.2),
    );
    print_reference_object_trace(
        "detect_objects_with_two_reference_mosaics_respects_relative_layout",
        "Square(20, 20) with Square(20, 20)",
        pair_reference_object(),
        TraceParams::new(24, 0.2),
    );
}

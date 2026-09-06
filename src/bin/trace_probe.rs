use image::{ImageBuffer, Rgb};
use imageproc::drawing::{draw_filled_circle_mut, draw_polygon_mut};
use imageproc::point::Point;
use rs_math3d::Vec3d;
use std::env;

use video_sentinel::math::{CoordinatedPoint, WrappedCoordinateSystem};
use video_sentinel::mosaics::{WrappedMosaic, deduce_mosaics};
use video_sentinel::object_detection::ReferenceObject;
use video_sentinel::slices::{
    AnnotatedSlice, BasicParams, Rectangle, Slice, SliceLine, SliceMatrix, WrappedRgbImage,
    calculate_slices, find_connected_slices,
};
use video_sentinel::traces::{Trace, TraceParams, set_trace_debug};

#[derive(Clone, Copy, PartialEq, Eq)]
enum ReferenceBuildMode {
    FromImage,
    FromSliceMatrix,
}

#[derive(Clone, Copy)]
enum SceneShapeKind {
    Square,
    Circle,
    Rectangle,
}

impl SceneShapeKind {
    fn as_str(self) -> &'static str {
        match self {
            SceneShapeKind::Square => "square",
            SceneShapeKind::Circle => "circle",
            SceneShapeKind::Rectangle => "rectangle",
        }
    }
}

#[derive(Clone, Copy)]
struct SceneShapeMarker {
    kind: SceneShapeKind,
    midpoint: Vec3d,
}

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

fn object_detection_scene_shapes() -> ShapesData {
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

fn rectangle_midpoint(rectangle: &ColoredTestRectangle) -> Vec3d {
    Vec3d::new(
        (rectangle.top_left.x + rectangle.bottom_right.x) / 2.0,
        (rectangle.top_left.y + rectangle.bottom_right.y) / 2.0,
        0.0,
    )
}

fn classify_rectangle(rectangle: &ColoredTestRectangle) -> SceneShapeKind {
    let width = (rectangle.bottom_right.x - rectangle.top_left.x).abs();
    let height = (rectangle.bottom_right.y - rectangle.top_left.y).abs();
    if (width - height).abs() < f64::EPSILON {
        SceneShapeKind::Square
    } else {
        SceneShapeKind::Rectangle
    }
}

fn scene_shape_markers(shapes_data: &ShapesData) -> Vec<SceneShapeMarker> {
    let rectangle_markers = shapes_data.rectangles.iter().map(|rectangle| SceneShapeMarker {
        kind: classify_rectangle(rectangle),
        midpoint: rectangle_midpoint(rectangle),
    });
    let circle_markers = shapes_data.circles.iter().map(|circle| SceneShapeMarker {
        kind: SceneShapeKind::Circle,
        midpoint: circle.center,
    });

    rectangle_markers.chain(circle_markers).collect()
}

fn distance_squared(left: Vec3d, right: Vec3d) -> f64 {
    let dx = left.x - right.x;
    let dy = left.y - right.y;
    let dz = left.z - right.z;
    dx * dx + dy * dy + dz * dz
}

fn classify_scene_mosaic(
    mosaic: &WrappedMosaic,
    scene_markers: &[SceneShapeMarker],
) -> Option<SceneShapeMarker> {
    let center = mosaic.get_center_of_mass();
    let center = Vec3d::new(center.get_x(), center.get_y(), center.get_z());
    scene_markers.iter().copied().min_by(|left, right| {
        distance_squared(center, left.midpoint)
            .partial_cmp(&distance_squared(center, right.midpoint))
            .unwrap()
    })
}

fn basic_params() -> BasicParams {
    BasicParams::new(false, 15)
}

fn global_coordinate_system() -> WrappedCoordinateSystem {
    WrappedCoordinateSystem::new(
        Vec3d::new(0.0, 0.0, 0.0),
        Vec3d::new(1.0, 0.0, 0.0),
        Vec3d::new(0.0, 1.0, 0.0),
    )
}

fn coordinated_point(x: f64, y: f64) -> CoordinatedPoint {
    CoordinatedPoint::new(global_coordinate_system(), Vec3d::new(x, y, 0.0))
}

fn blank_image(width: u32, height: u32) -> WrappedRgbImage {
    WrappedRgbImage::new(ImageBuffer::from_pixel(width, height, Rgb([0, 0, 0])))
}

fn add_horizontal_slice(
    slice_matrix: &mut SliceMatrix,
    line_number: usize,
    start_x: f64,
    end_x: f64,
) {
    let slice = Slice::new(
        coordinated_point(start_x, line_number as f64),
        coordinated_point(end_x, line_number as f64),
    );
    slice_matrix.add(SliceLine::new(
        line_number,
        vec![AnnotatedSlice::new(slice, line_number)],
    ));
}

fn rectangle_slice_matrix(
    width: u32,
    height: u32,
    top_left: Vec3d,
    bottom_right: Vec3d,
) -> SliceMatrix {
    let mut slice_matrix = SliceMatrix::new(blank_image(width, height));
    let start_x = top_left.x;
    let end_x = bottom_right.x - 1.0;

    for y in top_left.y as usize..bottom_right.y as usize {
        add_horizontal_slice(&mut slice_matrix, y, start_x, end_x);
    }

    slice_matrix
}

fn circle_slice_matrix(width: u32, height: u32, center: Vec3d, radius: f64) -> SliceMatrix {
    let mut slice_matrix = SliceMatrix::new(blank_image(width, height));
    let start_y = (center.y - radius).floor().max(0.0) as usize;
    let end_y = (center.y + radius).ceil().min(height as f64) as usize;
    let max_x = width.saturating_sub(1) as f64;

    for y in start_y..end_y {
        let dy = y as f64 - center.y;
        let x_offset = (radius * radius - dy * dy).max(0.0).sqrt();
        let start_x = (center.x - x_offset).ceil().max(0.0);
        let end_x = (center.x + x_offset).floor().min(max_x);
        if start_x <= end_x {
            add_horizontal_slice(&mut slice_matrix, y, start_x, end_x);
        }
    }

    slice_matrix
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
        mosaic.contains_point(CoordinatedPoint::new(global_coordinate_system(), position))
    })
}

fn reference_object_from_slice_matrices(
    id: &str,
    slice_matrices: Vec<SliceMatrix>,
) -> ReferenceObject {
    ReferenceObject::new(id.to_string(), deduce_mosaics(slice_matrices))
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

fn trace_cpp_square_reference_object(mode: ReferenceBuildMode) -> ReferenceObject {
    match mode {
        ReferenceBuildMode::FromImage => {
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
        ReferenceBuildMode::FromSliceMatrix => reference_object_from_slice_matrices(
            "square",
            vec![rectangle_slice_matrix(
                50,
                50,
                Vec3d::new(15.0, 15.0, 0.0),
                Vec3d::new(35.0, 35.0, 0.0),
            )],
        ),
    }
}

fn trace_cpp_circle_reference_object(mode: ReferenceBuildMode) -> ReferenceObject {
    match mode {
        ReferenceBuildMode::FromImage => {
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
        ReferenceBuildMode::FromSliceMatrix => reference_object_from_slice_matrices(
            "circle",
            vec![circle_slice_matrix(
                50,
                50,
                Vec3d::new(25.0, 25.0, 0.0),
                25.0,
            )],
        ),
    }
}

fn trace_cpp_rectangle_reference_object(mode: ReferenceBuildMode) -> ReferenceObject {
    match mode {
        ReferenceBuildMode::FromImage => {
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
        ReferenceBuildMode::FromSliceMatrix => reference_object_from_slice_matrices(
            "rectangle",
            vec![rectangle_slice_matrix(
                50,
                50,
                Vec3d::new(15.0, 15.0, 0.0),
                Vec3d::new(25.0, 35.0, 0.0),
            )],
        ),
    }
}

#[allow(dead_code)]
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

fn pair_reference_object(mode: ReferenceBuildMode) -> ReferenceObject {
    match mode {
        ReferenceBuildMode::FromImage => {
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
                    deduce_mosaic_at_position(reference_image, Vec3d::new(60.0, 20.0, 0.0))
                        .unwrap(),
                ],
            )
        }
        ReferenceBuildMode::FromSliceMatrix => reference_object_from_slice_matrices(
            "pair",
            vec![
                rectangle_slice_matrix(
                    80,
                    50,
                    Vec3d::new(10.0, 10.0, 0.0),
                    Vec3d::new(30.0, 30.0, 0.0),
                ),
                rectangle_slice_matrix(
                    80,
                    50,
                    Vec3d::new(50.0, 10.0, 0.0),
                    Vec3d::new(70.0, 30.0, 0.0),
                ),
            ],
        ),
    }
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
    println!(
        "mosaic count: {}",
        reference_object.get_mosaics(usize::MAX).len()
    );

    for (index, mosaic) in reference_object.get_mosaics(usize::MAX).iter().enumerate() {
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

fn print_reference_object_image_similarities(build_mode: ReferenceBuildMode) {
    let shapes_data = object_detection_scene_shapes();
    let scene_markers = scene_shape_markers(&shapes_data);
    let image = create_test_image_with_shapes(&shapes_data, 300, 300);
    let scene_mosaics = deduce_all_mosaics(image);
    let reference_cases = vec![
        (
            "reference_object_methods_return_id_surrounding_box_and_relative_rectangle",
            "Square(10, 10) with Square(20, 20)",
            reference_object_methods_reference_object(),
            TraceParams::new(36, 1e-4),
        ),
        (
            "detect_objects_finds_square_results_from_trace_cpp_scene",
            "Square(20, 20)",
            trace_cpp_square_reference_object(build_mode),
            TraceParams::new(36, 1e-4),
        ),
        (
            "detect_objects_finds_circle_results_from_trace_cpp_scene",
            "Circle(radius=25)",
            trace_cpp_circle_reference_object(build_mode),
            TraceParams::new(36, 1e-4),
        ),
        (
            "detect_objects_finds_rectangle_results_from_trace_cpp_scene",
            "Rectangle(10, 20)",
            trace_cpp_rectangle_reference_object(build_mode),
            TraceParams::new(36, 1e-4),
        ),
        (
            "detect_objects_with_two_reference_mosaics_respects_relative_layout",
            "Square(20, 20) with Square(20, 20)",
            pair_reference_object(build_mode),
            TraceParams::new(24, 1e-4),
        ),
    ];

    println!("=== compare-to-image ===");
    println!("scene mosaic count: {}", scene_mosaics.len());

    for (reference_name, shape_description, reference_object, params) in reference_cases {
        println!("reference: {reference_name}");
        println!("reference shape: {shape_description}");

        let reference_trace = Trace::new_from_mosaics(reference_object.get_mosaics(usize::MAX), params.clone());

        for (mosaic_index, mosaic) in scene_mosaics.iter().enumerate() {
            let mosaic_trace = Trace::new_from_mosaic(mosaic.clone(), params.clone());
            let similarity = reference_trace.compare_with(0.85, &mosaic_trace);
            let bounding_box = mosaic.get_bounding_box().to_global_rectangle();
            let center = mosaic.get_center_of_mass();
            let scene_shape = classify_scene_mosaic(mosaic, &scene_markers).unwrap();
            println!(
                "  scene_mosaic[{mosaic_index}] similarity={similarity:.8} shape={} midpoint=({:.8}, {:.8}, {:.8}) area={:.8} bbox=(({:.8}, {:.8}), ({:.8}, {:.8}))",
                scene_shape.kind.as_str(),
                center.get_x(),
                center.get_y(),
                center.get_z(),
                mosaic.get_area(),
                bounding_box.get_top_left().x,
                bounding_box.get_top_left().y,
                bounding_box.get_bottom_right().x,
                bounding_box.get_bottom_right().y,
            );
        }
    }
}

fn reference_cases(build_mode: ReferenceBuildMode) -> Vec<(&'static str, &'static str, ReferenceObject, TraceParams)> {
    vec![
        (
            "reference_object_methods_return_id_surrounding_box_and_relative_rectangle",
            "Square(10, 10) with Square(20, 20)",
            reference_object_methods_reference_object(),
            TraceParams::new(36, 1e-4),
        ),
        (
            "detect_objects_finds_square_results_from_trace_cpp_scene",
            "Square(20, 20)",
            trace_cpp_square_reference_object(build_mode),
            TraceParams::new(36, 1e-4),
        ),
        (
            "detect_objects_finds_circle_results_from_trace_cpp_scene",
            "Circle(radius=25)",
            trace_cpp_circle_reference_object(build_mode),
            TraceParams::new(36, 1e-4),
        ),
        (
            "detect_objects_finds_rectangle_results_from_trace_cpp_scene",
            "Rectangle(10, 20)",
            trace_cpp_rectangle_reference_object(build_mode),
            TraceParams::new(36, 1e-4),
        ),
        (
            "detect_objects_with_two_reference_mosaics_respects_relative_layout",
            "Square(20, 20) with Square(20, 20)",
            pair_reference_object(build_mode),
            TraceParams::new(24, 1e-4),
        ),
    ]
}

fn print_reference_object_cross_similarities(build_mode: ReferenceBuildMode) {
    let reference_cases = reference_cases(build_mode);
    let comparison_params = TraceParams::new(36, 1e-4);

    println!("=== compare-reference-objects ===");
    println!("reference count: {}", reference_cases.len());

    for (left_index, (left_name, left_shape, left_reference_object, _)) in
        reference_cases.iter().enumerate()
    {
        println!(
            "left_reference[{left_index}]: {left_name} shape={left_shape} mosaic_count={}",
            left_reference_object.get_mosaics(usize::MAX).len(),
        );
        let left_trace = Trace::new_from_mosaics(
            left_reference_object.get_mosaics(usize::MAX),
            comparison_params.clone(),
        );

        for (right_index, (right_name, right_shape, right_reference_object, _)) in
            reference_cases.iter().enumerate()
        {
            let right_trace = Trace::new_from_mosaics(
                right_reference_object.get_mosaics(usize::MAX),
                comparison_params.clone(),
            );
            println!(
                "compare reference[{left_index}]={left_name} ({left_shape}) with reference[{right_index}]={right_name} ({right_shape})",
            );
            let similarity = left_trace.compare_with(0.85, &right_trace);
            println!("  similarity={similarity:.8}");
        }
    }
}

fn main() {
    let args: Vec<String> = env::args().collect();
    let build_mode = if args.iter().any(|arg| arg == "--deduce-from-slice-matrix") {
        ReferenceBuildMode::FromSliceMatrix
    } else {
        ReferenceBuildMode::FromImage
    };
    let compare_to_image = args.iter().any(|arg| arg == "--compare-to-image");
    let compare_reference_objects = args
        .iter()
        .any(|arg| arg == "--compare-reference-objects");

    set_trace_debug(compare_to_image || compare_reference_objects);

    print_reference_object_trace(
        "reference_object_methods_return_id_surrounding_box_and_relative_rectangle",
        "Square(10, 10) with Square(20, 20)",
        reference_object_methods_reference_object(),
        TraceParams::new(36, 1e-4),
    );
    print_reference_object_trace(
        "detect_objects_finds_square_results_from_trace_cpp_scene",
        "Square(20, 20)",
        trace_cpp_square_reference_object(build_mode),
        TraceParams::new(36, 1e-4),
    );
    print_reference_object_trace(
        "detect_objects_finds_circle_results_from_trace_cpp_scene",
        "Circle(radius=25)",
        trace_cpp_circle_reference_object(build_mode),
        TraceParams::new(36, 1e-4),
    );
    print_reference_object_trace(
        "detect_objects_finds_rectangle_results_from_trace_cpp_scene",
        "Rectangle(10, 20)",
        trace_cpp_rectangle_reference_object(build_mode),
        TraceParams::new(36, 1e-4),
    );
    print_reference_object_trace(
        "detect_objects_with_two_reference_mosaics_respects_relative_layout",
        "Square(20, 20) with Square(20, 20)",
        pair_reference_object(build_mode),
        TraceParams::new(24, 1e-4),
    );

    if compare_to_image {
        print_reference_object_image_similarities(build_mode);
    }

    if compare_reference_objects {
        print_reference_object_cross_similarities(build_mode);
    }
}

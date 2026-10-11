use std::hint::black_box;
use std::path::PathBuf;
use std::sync::{Arc, Once, OnceLock};
use std::time::{Duration, Instant};

use criterion::measurement::WallTime;
use criterion::{BenchmarkGroup, Criterion};
use num_traits::ToPrimitive;
use opencv::core::{Mat, Vector as CvVector};
use opencv::imgcodecs;
use opencv::imgproc;
use opencv::prelude::*;
use rs_math3d::Vec3d;
use tokio::runtime::{Builder, Runtime};
use video_rs::{Decoder, Frame};
use video_sentinel::eye::{EyeParams, TileParams, deduce_bucketed_mosaics, deduce_rectangles};
use video_sentinel::math::{
    AnonymizedCoordinateSystem, CoordinateSystem, Rectangle as MathRectangle,
};
use video_sentinel::mosaics::{AnonymizedMosaic, Results, WrappedMosaic, deduce_mosaics};
use video_sentinel::object_detection::{ObjectDetectionParams, ReferenceObject, detect_objects};
use video_sentinel::service::{
    AddObjectToBeDetectedResult, BasicParamsInput, EyeParamsInput, GetRectanglesResult,
    ObjectDetectionParamsInput, Service, TileParamsInput, TraceParamsInput,
};
use video_sentinel::slices::{
    BasicParams, Rectangle, WrappedRgbImage, calculate_slices, find_connected_slices,
};
use video_sentinel::traced_mosaics::TracedRelativeMosaic;
use video_sentinel::traces::{Trace, TraceParams};

static VIDEO_RS_INIT: Once = Once::new();
static EXAMPLE_VIDEO: OnceLock<ExampleVideoData> = OnceLock::new();
pub static TRACE_FIXTURE: OnceLock<TraceFixture> = OnceLock::new();
pub static DETECTION_FIXTURE: OnceLock<DetectionFixture> = OnceLock::new();
static BENCHMARK_RUNTIME: OnceLock<Runtime> = OnceLock::new();

fn i32_from_f64(value: f64) -> i32 {
    value
        .round()
        .to_i32()
        .expect("fixture coordinate should fit within i32")
}

fn u32_from_usize(value: usize) -> u32 {
    value
        .to_u32()
        .expect("fixture dimension should fit within u32")
}

fn u64_from_u128(value: u128) -> u64 {
    value
        .to_u64()
        .expect("benchmark duration should fit within u64 nanoseconds")
}

#[derive(Clone)]
pub struct ExampleFrame {
    pub image: WrappedRgbImage,
    pub jpeg: Arc<Vec<u8>>,
}

#[derive(Clone)]
pub struct ExampleVideoData {
    pub frames: Vec<ExampleFrame>,
}

#[derive(Clone)]
pub struct EyePipelineFixture {
    pub previous_bucketed_mosaics: video_sentinel::bucketed_mosaics::BucketedMosaics,
    pub next_mosaics: Vec<AnonymizedMosaic>,
    pub surrounding_rectangle: Rectangle,
    pub eye_params: EyeParams,
}

#[derive(Clone)]
pub struct TraceFixture {
    pub primary_mosaic: WrappedMosaic,
    pub secondary_mosaic: WrappedMosaic,
    pub primary_trace: Trace,
    pub secondary_trace: Trace,
}

#[derive(Clone)]
pub struct DetectionFixture {
    pub surrounding_rectangle: Rectangle,
    pub object_detection_params: ObjectDetectionParams,
    pub reference_object: ReferenceObject,
    pub bucketed_mosaics: video_sentinel::bucketed_mosaics::BucketedMosaics,
    pub query_mosaic: TracedRelativeMosaic,
}

#[derive(Clone)]
pub struct ColoredTestRectangle {
    pub top_left: Vec3d,
    pub bottom_right: Vec3d,
    pub color: &'static str,
    pub rotation_angle_degrees: f64,
}

#[derive(Clone)]
pub struct ColoredTestCircle {
    pub center: Vec3d,
    pub radius: f64,
    pub color: &'static str,
}

#[derive(Default, Clone)]
pub struct ShapesData {
    pub rectangles: Vec<ColoredTestRectangle>,
    pub circles: Vec<ColoredTestCircle>,
}

#[must_use]
pub fn criterion_config() -> Criterion {
    Criterion::default()
        .sample_size(10)
        .warm_up_time(Duration::from_millis(250))
        .measurement_time(Duration::from_secs(1))
        .without_plots()
}

pub fn benchmark_runtime() -> &'static Runtime {
    BENCHMARK_RUNTIME.get_or_init(|| {
        Builder::new_multi_thread()
            .worker_threads(4)
            .enable_all()
            .build()
            .expect("benchmark tokio runtime should build")
    })
}

pub fn bench_with_setup<S, O, Setup, Routine>(
    group: &mut BenchmarkGroup<'_, WallTime>,
    name: &'static str,
    budget: Duration,
    mut setup: Setup,
    mut routine: Routine,
) where
    Setup: FnMut() -> S,
    Routine: FnMut(S) -> O,
{
    group.bench_function(name, |b| {
        b.iter_custom(|iters| {
            let mut total = Duration::ZERO;
            for _ in 0..iters {
                let state = setup();
                let start = Instant::now();
                black_box(routine(state));
                total += start.elapsed();
            }
            enforce_budget(name, total, iters, budget);
            total
        });
    });
}

fn enforce_budget(name: &str, total: Duration, iters: u64, budget: Duration) {
    if !should_enforce_budgets() || iters == 0 {
        return;
    }
    let avg_nanos = total.as_nanos() / u128::from(iters);
    let budget_nanos = budget.as_nanos();
    assert!(
        avg_nanos <= budget_nanos,
        "benchmark `{name}` exceeded budget: avg {:?} > budget {:?}",
        Duration::from_nanos(u64_from_u128(avg_nanos)),
        budget,
    );
}

#[must_use]
pub fn should_enforce_budgets() -> bool {
    std::env::var_os("BENCH_ENFORCE_BUDGETS").is_some() || std::env::var_os("CI").is_some()
}

pub fn example_video() -> &'static ExampleVideoData {
    EXAMPLE_VIDEO.get_or_init(load_example_video)
}

pub fn load_example_video() -> ExampleVideoData {
    VIDEO_RS_INIT.call_once(|| {
        video_rs::init().expect("video-rs should initialize for benchmarks");
    });

    let video_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("video")
        .join("BillardTakeoff.mp4");
    let mut decoder = Decoder::new(video_path.as_path()).expect("example video should open");
    let frames = decoder
        .decode_iter()
        .take(3)
        .map(|frame_result| {
            let (_timestamp, frame) = frame_result.expect("example frame should decode");
            let image = wrapped_rgb_image_from_frame(&frame);
            ExampleFrame {
                jpeg: Arc::new(jpeg_bytes_from_image(&image)),
                image,
            }
        })
        .collect();
    ExampleVideoData { frames }
}

#[must_use]
pub fn ordinary_service() -> Service {
    let mut service = Service::new();
    assert!(matches!(
        service.create_ordinary_session("ordinary", basic_params_input(), Results::Absolute),
        video_sentinel::service::CreateOrdinarySessionResult::Success
    ));
    service
}

#[must_use]
pub fn eye_service() -> Service {
    let mut service = Service::new();
    assert!(matches!(
        service.create_eye_session(
            "eye",
            basic_params_input(),
            eye_params_input(),
            Results::Relative
        ),
        video_sentinel::service::CreateEyeSessionResult::Success
    ));
    service
}

#[must_use]
pub fn object_service() -> Service {
    let frames = &example_video().frames;
    let reference_rectangle = first_rectangle_from_ordinary(&frames[0].image);

    let mut service = Service::new();
    assert!(matches!(
        service.create_object_session(
            "object",
            basic_params_input(),
            object_detection_params_input(),
            Results::Absolute
        ),
        video_sentinel::service::CreateObjectSessionResult::Success
    ));
    assert!(matches!(
        service.add_object_to_be_detected_as_image(
            "object",
            "billard-reference",
            &frames[0].image,
            reference_rectangle,
        ),
        AddObjectToBeDetectedResult::Success
    ));
    service
}

#[must_use]
pub fn first_rectangle_from_ordinary(image: &WrappedRgbImage) -> Rectangle {
    let mut service = ordinary_service();
    match benchmark_runtime().block_on(service.get_rectangles("ordinary", image.clone())) {
        GetRectanglesResult::Success(rectangles) => {
            rectangles
                .into_iter()
                .next()
                .expect("ordinary BillardTakeoff frame should yield at least one mosaic")
                .bounding_box
        }
        _ => panic!("ordinary benchmark setup should return rectangles"),
    }
}

#[must_use]
pub fn basic_params() -> BasicParams {
    BasicParams::new(false, 15)
}

#[must_use]
pub fn trace_params() -> TraceParams {
    TraceParams::new(12, 0.2)
}

#[must_use]
pub fn eye_params() -> EyeParams {
    EyeParams::new(TileParams::new(0.5, 0.5), 0.25, trace_params(), 0.9)
}

#[must_use]
pub fn object_detection_params() -> ObjectDetectionParams {
    ObjectDetectionParams::new(TileParams::new(0.5, 0.5), 0.25, trace_params(), 0.9)
}

#[must_use]
pub fn basic_params_input() -> BasicParamsInput {
    BasicParamsInput {
        do_grayscale: false,
        gradient_threshold: 15,
    }
}

#[must_use]
pub fn eye_params_input() -> EyeParamsInput {
    EyeParamsInput {
        tile_params: TileParamsInput {
            tile_x: 0.5,
            tile_y: 0.5,
        },
        bucket_delta: 0.25,
        trace_params: TraceParamsInput {
            num_skeleton: 12,
            close_slice_threshold: 0.2,
        },
        target_similarity: 0.9,
    }
}

#[must_use]
pub fn object_detection_params_input() -> ObjectDetectionParamsInput {
    ObjectDetectionParamsInput {
        tile_params: TileParamsInput {
            tile_x: 0.5,
            tile_y: 0.5,
        },
        bucket_delta: 0.25,
        trace_params: TraceParamsInput {
            num_skeleton: 12,
            close_slice_threshold: 0.2,
        },
        target_similarity: 0.9,
    }
}

#[must_use]
pub fn surrounding_rectangle(image: &WrappedRgbImage) -> Rectangle {
    Rectangle::new(
        Vec3d::new(0.0, 0.0, 0.0),
        Vec3d::new(f64::from(image.width()), f64::from(image.height()), 0.0),
    )
}

#[must_use]
pub fn ordinary_mosaics(image: &WrappedRgbImage, results: Results) -> Vec<AnonymizedMosaic> {
    let rectangle = surrounding_rectangle(image);
    let slices = calculate_slices(image, rectangle, basic_params());
    let connected_slices = find_connected_slices(&mut slices.clone());
    deduce_mosaics(
        connected_slices,
        &MathRectangle::new(rectangle.get_top_left(), rectangle.get_bottom_right()),
    )
    .into_iter()
    .map(|wrapped_relative_mosaic| {
        AnonymizedMosaic::new(
            results,
            wrapped_relative_mosaic.get_mosaic(),
            Some(wrapped_relative_mosaic),
        )
    })
    .collect()
}

#[must_use]
pub fn eye_pipeline_fixture() -> EyePipelineFixture {
    let frames = &example_video().frames;
    let current_image = frames[2].image.clone();
    let previous_image = frames[1].image.clone();
    let surrounding_rectangle = surrounding_rectangle(&current_image);
    let current_mosaics = ordinary_mosaics(&current_image, Results::Relative);
    let previous_mosaics = ordinary_mosaics(&previous_image, Results::Relative);
    let eye_params = eye_params();
    let previous_bucketed_mosaics = deduce_bucketed_mosaics(
        previous_mosaics,
        surrounding_rectangle,
        eye_params.tile_params,
        eye_params.bucket_delta,
        eye_params.trace_params,
    );
    EyePipelineFixture {
        previous_bucketed_mosaics,
        next_mosaics: current_mosaics,
        surrounding_rectangle,
        eye_params,
    }
}

pub fn trace_fixture() -> &'static TraceFixture {
    TRACE_FIXTURE.get_or_init(|| {
        let reference_image = create_test_image_with_shapes(
            &ShapesData {
                rectangles: vec![ColoredTestRectangle {
                    top_left: Vec3d::new(20.0, 20.0, 0.0),
                    bottom_right: Vec3d::new(60.0, 60.0, 0.0),
                    color: "white",
                    rotation_angle_degrees: 0.0,
                }],
                circles: vec![ColoredTestCircle {
                    center: Vec3d::new(90.0, 90.0, 0.0),
                    radius: 10.0,
                    color: "blue",
                }],
            },
            160,
            160,
        );
        let comparison_image = create_test_image_with_shapes(
            &ShapesData {
                rectangles: vec![ColoredTestRectangle {
                    top_left: Vec3d::new(24.0, 18.0, 0.0),
                    bottom_right: Vec3d::new(62.0, 58.0, 0.0),
                    color: "white",
                    rotation_angle_degrees: 8.0,
                }],
                circles: vec![ColoredTestCircle {
                    center: Vec3d::new(110.0, 35.0, 0.0),
                    radius: 12.0,
                    color: "green",
                }],
            },
            160,
            160,
        );
        let primary_mosaic =
            deduce_mosaic_at_position(&reference_image, Vec3d::new(40.0, 40.0, 0.0))
                .expect("trace fixture primary mosaic should exist")
                .get_relative_mosaic()
                .get_mosaic();
        let secondary_mosaic =
            deduce_mosaic_at_position(&comparison_image, Vec3d::new(42.0, 38.0, 0.0))
                .expect("trace fixture secondary mosaic should exist")
                .get_relative_mosaic()
                .get_mosaic();
        let params = trace_params();
        TraceFixture {
            primary_trace: Trace::new_from_mosaic(&primary_mosaic, params),
            secondary_trace: Trace::new_from_mosaic(&secondary_mosaic, params),
            primary_mosaic,
            secondary_mosaic,
        }
    })
}

pub fn detection_fixture() -> &'static DetectionFixture {
    DETECTION_FIXTURE.get_or_init(|| {
        let reference_image = create_test_image_with_shapes(
            &ShapesData {
                rectangles: vec![
                    ColoredTestRectangle {
                        top_left: Vec3d::new(10.0, 10.0, 0.0),
                        bottom_right: Vec3d::new(32.0, 34.0, 0.0),
                        color: "white",
                        rotation_angle_degrees: 0.0,
                    },
                    ColoredTestRectangle {
                        top_left: Vec3d::new(42.0, 14.0, 0.0),
                        bottom_right: Vec3d::new(52.0, 24.0, 0.0),
                        color: "white",
                        rotation_angle_degrees: 0.0,
                    },
                ],
                circles: Vec::new(),
            },
            80,
            60,
        );
        let scene_image = create_test_image_with_shapes(
            &ShapesData {
                rectangles: vec![
                    ColoredTestRectangle {
                        top_left: Vec3d::new(100.0, 90.0, 0.0),
                        bottom_right: Vec3d::new(122.0, 114.0, 0.0),
                        color: "white",
                        rotation_angle_degrees: 0.0,
                    },
                    ColoredTestRectangle {
                        top_left: Vec3d::new(132.0, 94.0, 0.0),
                        bottom_right: Vec3d::new(142.0, 104.0, 0.0),
                        color: "white",
                        rotation_angle_degrees: 0.0,
                    },
                    ColoredTestRectangle {
                        top_left: Vec3d::new(170.0, 30.0, 0.0),
                        bottom_right: Vec3d::new(190.0, 54.0, 0.0),
                        color: "white",
                        rotation_angle_degrees: 22.0,
                    },
                ],
                circles: vec![ColoredTestCircle {
                    center: Vec3d::new(45.0, 145.0, 0.0),
                    radius: 12.0,
                    color: "green",
                }],
            },
            240,
            180,
        );
        let reference_object = reference_object_from_positions(
            &reference_image,
            &[Vec3d::new(20.0, 20.0, 0.0), Vec3d::new(46.0, 18.0, 0.0)],
            "double-rect",
        );
        let scene_mosaics = deduce_all_mosaics(&scene_image);
        let surrounding_rectangle = surrounding_rectangle(&scene_image);
        let object_detection_params = ObjectDetectionParams::new(
            TileParams::new(0.25, 0.5),
            0.2,
            TraceParams::new(12, 0.2),
            0.8,
        );
        let bucketed_mosaics = deduce_bucketed_mosaics(
            scene_mosaics
                .iter()
                .map(|mosaic| {
                    AnonymizedMosaic::new(
                        Results::Absolute,
                        mosaic.get_relative_mosaic().get_mosaic(),
                        Some(mosaic.get_relative_mosaic()),
                    )
                })
                .collect(),
            surrounding_rectangle,
            object_detection_params.tile_params,
            object_detection_params.bucket_delta,
            object_detection_params.trace_params,
        );
        let query_mosaic = scene_mosaics[0].clone();
        DetectionFixture {
            surrounding_rectangle,
            object_detection_params,
            reference_object,
            bucketed_mosaics,
            query_mosaic,
        }
    })
}

#[must_use]
pub fn detect_objects_for_fixture(
    fixture: &DetectionFixture,
) -> Vec<video_sentinel::slices::ColoredRectangle> {
    detect_objects(
        &fixture.reference_object,
        &fixture.bucketed_mosaics,
        fixture.object_detection_params,
        fixture.surrounding_rectangle,
        Results::Absolute,
    )
}

pub async fn deduce_rectangles_for_fixture(
    fixture: EyePipelineFixture,
) -> Result<Vec<video_sentinel::slices::ColoredRectangle>, String> {
    deduce_rectangles(
        fixture.previous_bucketed_mosaics,
        fixture.next_mosaics,
        fixture.eye_params,
        fixture.surrounding_rectangle,
    )
    .await
}

fn wrapped_rgb_image_from_frame(frame: &Frame) -> WrappedRgbImage {
    let (height, width, channels) = frame.dim();
    assert_eq!(channels, 3, "benchmark video frames should have 3 channels");
    let data = if let Some(slice) = frame.as_slice_memory_order() {
        slice.to_vec()
    } else {
        frame.iter().copied().collect()
    };
    WrappedRgbImage::from_rgb_data(u32_from_usize(width), u32_from_usize(height), &data)
        .expect("decoded frame should convert to WrappedRgbImage")
}

fn jpeg_bytes_from_image(image: &WrappedRgbImage) -> Vec<u8> {
    let rgb = image
        .get_mat()
        .try_clone()
        .expect("benchmark image should clone for encoding");
    let mut bgr = Mat::default();
    imgproc::cvt_color(&rgb, &mut bgr, imgproc::COLOR_RGB2BGR, 0)
        .expect("benchmark image should convert to BGR");
    let mut encoded = CvVector::<u8>::new();
    imgcodecs::imencode(".jpg", &bgr, &mut encoded, &CvVector::<i32>::new())
        .expect("benchmark image should encode as JPEG");
    encoded.to_vec()
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

fn deduce_all_mosaics(image: &WrappedRgbImage) -> Vec<TracedRelativeMosaic> {
    let trace_params = trace_params();
    let rectangle = surrounding_rectangle(image);
    let math_rectangle = MathRectangle::new(rectangle.get_top_left(), rectangle.get_bottom_right());
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
            &video_sentinel::math::CoordinatedPoint::new(
                global_coordinate_system.clone(),
                position,
            ),
        )
    })
}

fn reference_object_from_positions(
    image: &WrappedRgbImage,
    positions: &[Vec3d],
    id: &str,
) -> ReferenceObject {
    let mosaics = positions
        .iter()
        .map(|position| {
            deduce_mosaic_at_position(image, *position)
                .expect("benchmark reference object mosaic should exist")
        })
        .collect();
    ReferenceObject::new(id.to_string(), mosaics)
}

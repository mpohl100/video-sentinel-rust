use rs_math3d::Vec3d;
use vs_core::mosaics::Results;
use vs_core::slices::{Rectangle, WrappedRgbImage};
use vs_service::service::{
    AddColorFilterResult, AddObjectToBeDetectedResult, BasicParamsInput, EnrichedMosaic,
    GetRectanglesResult, ObjectDetectionParamsInput, RgbColor, Service, TileParamsInput,
    TraceParamsInput,
};

fn rgb(red: u8, green: u8, blue: u8) -> RgbColor {
    RgbColor { red, green, blue }
}

fn new_service() -> Service {
    let mut service = Service::new();
    service.create_object_session(
        "object",
        BasicParamsInput {
            do_grayscale: false,
            gradient_threshold: 15,
        },
        ObjectDetectionParamsInput {
            tile_params: TileParamsInput {
                tile_x: 0.2,
                tile_y: 0.2,
            },
            bucket_delta: 0.5,
            trace_params: TraceParamsInput { num_skeleton: 36 },
            target_similarity: 0.8,
        },
        Results::Absolute,
    );
    service
}

fn add_filter(service: &mut Service, color: RgbColor) {
    assert_eq!(
        service.add_color_filter("object", color, rgb(5, 5, 5)),
        AddColorFilterResult::Success
    );
}

fn shapes() -> WrappedRgbImage {
    let image = WrappedRgbImage::new_with_color(100, 50, [0, 0, 0]);
    image.fill_polygon(&[(10, 10), (30, 10), (30, 30), (10, 30)], [255, 0, 0]);
    image.fill_polygon(&[(60, 10), (80, 10), (80, 30), (60, 30)], [0, 255, 0]);
    image
}

fn full_rectangle() -> Rectangle {
    Rectangle::new(Vec3d::new(0.0, 0.0, 0.0), Vec3d::new(100.0, 50.0, 0.0))
}

fn reference_colors(service: &Service) -> Vec<Vec3d> {
    service
        .get_object_session("object")
        .unwrap()
        .objects_to_be_detected[0]
        .get_mosaics(usize::MAX)
        .iter()
        .map(|mosaic| {
            mosaic
                .get_relative_mosaic()
                .get_mosaic()
                .get_average_color()
        })
        .collect()
}

async fn rectangles(service: &Service, image: WrappedRgbImage) -> Vec<EnrichedMosaic> {
    match service.get_rectangles("object", image).await {
        GetRectanglesResult::Success(mosaics) => mosaics,
        GetRectanglesResult::SessionNotFound => panic!("missing session"),
        GetRectanglesResult::ComputationFailed(error) => panic!("detection failed: {error}"),
    }
}

#[test]
fn reference_filters_select_alternatives_and_reject_empty_references() {
    let image = shapes();
    let mut unfiltered = new_service();
    assert!(matches!(
        unfiltered.add_object_to_be_detected_as_image("object", "all", &image, full_rectangle()),
        AddObjectToBeDetectedResult::Success
    ));
    let unfiltered_colors = reference_colors(&unfiltered);
    assert!(unfiltered_colors.iter().any(|color| color.x > 250.0));
    assert!(unfiltered_colors.iter().any(|color| color.y > 250.0));

    let mut service = new_service();
    add_filter(&mut service, rgb(255, 0, 0));
    assert!(matches!(
        service.add_object_to_be_detected_as_image("object", "red", &image, full_rectangle()),
        AddObjectToBeDetectedResult::Success
    ));
    let colors = reference_colors(&service);
    assert_eq!(colors.len(), 1);
    assert!(colors[0].x > 250.0 && colors[0].y < 5.0 && colors[0].z < 5.0);

    add_filter(&mut service, rgb(0, 255, 0));
    assert!(matches!(
        service.add_object_to_be_detected_as_image("object", "both", &image, full_rectangle()),
        AddObjectToBeDetectedResult::Success
    ));
    let refs = service
        .get_object_session("object")
        .unwrap()
        .objects_to_be_detected;
    let colors = refs[1].get_mosaics(usize::MAX);
    assert_eq!(colors.len(), 2);
    assert_eq!(refs[0].get_mosaics(usize::MAX).len(), 1);

    let mut no_match = new_service();
    add_filter(&mut no_match, rgb(0, 0, 255));
    assert!(matches!(
        no_match.add_object_to_be_detected_as_image("object", "missing", &image, full_rectangle()),
        AddObjectToBeDetectedResult::NoReferenceMosaicsDetected
    ));
    assert!(
        no_match
            .get_object_session("object")
            .unwrap()
            .objects_to_be_detected
            .is_empty()
    );
}

#[tokio::test]
async fn every_mosaic_in_a_multi_mosaic_candidate_must_match() {
    let mut service = new_service();
    service.update_object_detection_params(
        "object",
        ObjectDetectionParamsInput {
            tile_params: TileParamsInput {
                tile_x: 0.25,
                tile_y: 0.25,
            },
            bucket_delta: 0.5,
            trace_params: TraceParamsInput { num_skeleton: 24 },
            target_similarity: 0.8,
        },
    );
    add_filter(&mut service, rgb(255, 0, 0));
    add_filter(&mut service, rgb(0, 255, 0));
    let reference = WrappedRgbImage::new_with_color(80, 50, [0, 0, 0]);
    reference.fill_polygon(&[(10, 10), (30, 10), (30, 30), (10, 30)], [255, 0, 0]);
    reference.fill_polygon(&[(50, 10), (70, 10), (70, 30), (50, 30)], [0, 255, 0]);
    assert!(matches!(
        service.add_object_to_be_detected_as_image(
            "object",
            "pair",
            &reference,
            Rectangle::new(Vec3d::new(0.0, 0.0, 0.0), Vec3d::new(80.0, 50.0, 0.0))
        ),
        AddObjectToBeDetectedResult::Success
    ));
    let scene = |second_color| {
        let image = WrappedRgbImage::new_with_color(180, 100, [0, 0, 0]);
        for (x, y, size, color) in [
            (10, 10, 20, [255, 0, 0]),
            (50, 10, 20, second_color),
            (100, 10, 20, [255, 0, 0]),
            (140, 10, 20, second_color),
            (100, 50, 16, [255, 0, 0]),
        ] {
            image.fill_polygon(
                &[(x, y), (x + size, y), (x + size, y + size), (x, y + size)],
                color,
            );
        }
        image
    };
    let matches = rectangles(&service, scene([0, 255, 0])).await;
    assert!(
        !matches.is_empty(),
        "the two matching shapes should form a candidate"
    );
    assert!(rectangles(&service, scene([0, 0, 255])).await.is_empty());
    add_filter(&mut service, rgb(0, 0, 255));
    assert!(!rectangles(&service, scene([0, 0, 255])).await.is_empty());
}

#[tokio::test]
async fn object_candidates_are_filtered_before_detection() {
    let image = shapes();
    let mut service = new_service();
    add_filter(&mut service, rgb(255, 0, 0));
    assert!(matches!(
        service.add_object_to_be_detected_as_image("object", "square", &image, full_rectangle()),
        AddObjectToBeDetectedResult::Success
    ));
    let red = rectangles(&service, image.clone()).await;
    assert_eq!(red.len(), 1);
    assert_eq!(red[0].average_color, rgb(255, 0, 0));

    add_filter(&mut service, rgb(0, 255, 0));
    let both = rectangles(&service, image).await;
    assert_eq!(both.len(), 2);
    assert!(
        both.iter()
            .any(|mosaic| mosaic.average_color == rgb(255, 0, 0))
    );
    assert!(
        both.iter()
            .any(|mosaic| mosaic.average_color == rgb(0, 255, 0))
    );
}

#[tokio::test]
async fn object_color_filters_use_rgb_with_grayscale_segmentation_and_relative_results() {
    let mut service = Service::new();
    service.create_object_session(
        "object",
        BasicParamsInput {
            do_grayscale: true,
            gradient_threshold: 15,
        },
        ObjectDetectionParamsInput {
            tile_params: TileParamsInput {
                tile_x: 0.2,
                tile_y: 0.2,
            },
            bucket_delta: 0.5,
            trace_params: TraceParamsInput { num_skeleton: 36 },
            target_similarity: 0.8,
        },
        Results::Relative,
    );
    add_filter(&mut service, rgb(255, 0, 0));
    let image = shapes();
    assert!(matches!(
        service.add_object_to_be_detected_as_image("object", "red", &image, full_rectangle()),
        AddObjectToBeDetectedResult::Success
    ));
    let detected = rectangles(&service, image).await;
    assert_eq!(detected.len(), 1);
    assert_eq!(detected[0].average_color, rgb(255, 0, 0));
}

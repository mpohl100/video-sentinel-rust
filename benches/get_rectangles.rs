mod common;

use std::cell::Cell;
use std::time::Duration;

use criterion::{criterion_group, criterion_main};
use video_sentinel::service::GetRectanglesResult;
use video_sentinel::slices::WrappedRgbImage;

use common::{
    bench_with_setup, benchmark_runtime, criterion_config, example_video, eye_service,
    object_service, ordinary_service,
};

fn get_rectangles_benchmarks(c: &mut criterion::Criterion) {
    let frames = example_video().frames.clone();
    let mut group = c.benchmark_group("get_rectangles");

    let decode_index = Cell::new(0usize);
    bench_with_setup(
        &mut group,
        "decode_jpeg_frame",
        Duration::from_millis(40),
        || {
            let index = decode_index.get();
            decode_index.set((index + 1) % frames.len());
            frames[index].jpeg.as_ref().clone()
        },
        |jpeg| {
            WrappedRgbImage::from_jpeg_bytes(&jpeg).expect("JPEG benchmark decode should succeed")
        },
    );

    let ordinary_service = ordinary_service();
    let ordinary_index = Cell::new(0usize);
    bench_with_setup(
        &mut group,
        "service_get_rectangles_ordinary",
        Duration::from_millis(550),
        || {
            let index = ordinary_index.get();
            ordinary_index.set((index + 1) % frames.len());
            frames[index].image.clone()
        },
        |image| match benchmark_runtime().block_on(ordinary_service.get_rectangles(
            "ordinary".to_string(),
            image,
            None,
        )) {
            GetRectanglesResult::Success(rectangles) => rectangles.len(),
            _ => panic!("ordinary benchmark should succeed"),
        },
    );

    let eye_service = eye_service();
    let eye_pairs = [
        (frames[1].image.clone(), frames[0].image.clone()),
        (frames[2].image.clone(), frames[1].image.clone()),
        (frames[0].image.clone(), frames[2].image.clone()),
    ];
    let eye_index = Cell::new(0usize);
    bench_with_setup(
        &mut group,
        "service_get_rectangles_eye",
        Duration::from_secs(5),
        || {
            let index = eye_index.get();
            eye_index.set((index + 1) % eye_pairs.len());
            eye_pairs[index].clone()
        },
        |(image, previous_image)| match benchmark_runtime().block_on(eye_service.get_rectangles(
            "eye".to_string(),
            image,
            Some(previous_image),
        )) {
            GetRectanglesResult::Success(rectangles) => rectangles.len(),
            _ => panic!("eye benchmark should succeed"),
        },
    );

    let object_service = object_service();
    let object_index = Cell::new(0usize);
    bench_with_setup(
        &mut group,
        "service_get_rectangles_object",
        Duration::from_secs(2),
        || {
            let index = object_index.get();
            object_index.set((index + 1) % frames.len());
            frames[index].image.clone()
        },
        |image| match benchmark_runtime().block_on(object_service.get_rectangles(
            "object".to_string(),
            image,
            None,
        )) {
            GetRectanglesResult::Success(rectangles) => rectangles.len(),
            _ => panic!("object benchmark should succeed"),
        },
    );

    group.finish();
}

criterion_group! {
    name = benches;
    config = criterion_config();
    targets = get_rectangles_benchmarks
}
criterion_main!(benches);

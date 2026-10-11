#![warn(clippy::pedantic)]
#![allow(
    clippy::missing_errors_doc,
    clippy::missing_panics_doc,
    clippy::too_many_lines,
    clippy::unused_async_trait_impl
)]

pub mod common;

use std::time::Duration;

use criterion::{criterion_group, criterion_main};
use video_sentinel::eye::deduce_bucketed_mosaics;
use video_sentinel::mosaics::{Results, deduce_mosaics};
use video_sentinel::slices::{WrappedRgbImage, calculate_slices, find_connected_slices};
use video_sentinel::traces::Trace;

use common::{
    basic_params, bench_with_setup, benchmark_runtime, criterion_config,
    deduce_rectangles_for_fixture, detect_objects_for_fixture, detection_fixture, example_video,
    eye_pipeline_fixture, object_detection_params, ordinary_mosaics, surrounding_rectangle,
    trace_fixture,
};

fn pipeline_benchmarks(c: &mut criterion::Criterion) {
    let frames = example_video().frames.clone();
    let frame = frames[0].image.clone();
    let frame_rectangle = surrounding_rectangle(&frame);
    let frame_slices = calculate_slices(&frame, frame_rectangle, basic_params());
    let connected_slices = find_connected_slices(&mut frame_slices.clone());

    let mut group = c.benchmark_group("pipeline");

    bench_with_setup(
        &mut group,
        "slices_calculate_slices",
        Duration::from_millis(400),
        || frame.clone(),
        |image: WrappedRgbImage| {
            calculate_slices(&image, frame_rectangle, basic_params())
                .get_slice_lines()
                .len()
        },
    );

    bench_with_setup(
        &mut group,
        "slices_find_connected_slices",
        Duration::from_millis(80),
        || frame_slices.clone(),
        |mut slices| find_connected_slices(&mut slices).len(),
    );

    bench_with_setup(
        &mut group,
        "mosaics_deduce_mosaics",
        Duration::from_millis(10),
        || connected_slices.clone(),
        |slice_matrices| {
            deduce_mosaics(
                slice_matrices,
                &video_sentinel::math::Rectangle::new(
                    frame_rectangle.get_top_left(),
                    frame_rectangle.get_bottom_right(),
                ),
            )
            .len()
        },
    );

    let trace_fixture = trace_fixture().clone();
    bench_with_setup(
        &mut group,
        "traces_new_from_mosaic",
        Duration::from_millis(2),
        || trace_fixture.primary_mosaic.clone(),
        |mosaic| Trace::new_from_mosaic(&mosaic, common::trace_params()),
    );

    bench_with_setup(
        &mut group,
        "traces_compare_with",
        Duration::from_millis(1),
        || {
            (
                trace_fixture.primary_trace.clone(),
                trace_fixture.secondary_trace.clone(),
            )
        },
        |(left, right)| left.compare_with(0.8, &right),
    );

    let eye_fixture = eye_pipeline_fixture();
    let bucket_inputs = (
        eye_fixture.next_mosaics.clone(),
        eye_fixture.surrounding_rectangle,
        eye_fixture.eye_params,
    );
    bench_with_setup(
        &mut group,
        "bucketed_mosaics_deduce_bucketed_mosaics",
        Duration::from_millis(250),
        || bucket_inputs.clone(),
        |(mosaics, surrounding_rectangle, eye_params)| {
            benchmark_runtime()
                .block_on(deduce_bucketed_mosaics(
                    mosaics,
                    surrounding_rectangle,
                    eye_params.tile_params,
                    eye_params.bucket_delta,
                    eye_params.trace_params,
                ))
                .expect("bucketed mosaic benchmark should succeed")
        },
    );

    let bucket_lookup_fixture = detection_fixture().clone();
    bench_with_setup(
        &mut group,
        "bucketed_mosaics_get_all_similar_mosaics",
        Duration::from_millis(1),
        || {
            (
                bucket_lookup_fixture.bucketed_mosaics.clone(),
                bucket_lookup_fixture.query_mosaic.clone(),
            )
        },
        |(bucketed_mosaics, query_mosaic)| {
            bucketed_mosaics
                .get_all_similar_mosaics(&query_mosaic)
                .len()
        },
    );

    bench_with_setup(
        &mut group,
        "eye_deduce_rectangles",
        Duration::from_secs(4),
        eye_pipeline_fixture,
        |fixture| {
            benchmark_runtime()
                .block_on(deduce_rectangles_for_fixture(fixture))
                .expect("eye rectangle benchmark should succeed")
                .len()
        },
    );

    bench_with_setup(
        &mut group,
        "detections_detect_objects",
        Duration::from_millis(40),
        || detection_fixture().clone(),
        |fixture| detect_objects_for_fixture(&fixture).len(),
    );

    let object_mosaics_inputs = (
        ordinary_mosaics(&frames[2].image, Results::Absolute),
        surrounding_rectangle(&frames[2].image),
        object_detection_params(),
    );
    bench_with_setup(
        &mut group,
        "object_pipeline_build_bucketed_scene",
        Duration::from_millis(300),
        || object_mosaics_inputs.clone(),
        |(mosaics, rectangle, params)| {
            benchmark_runtime()
                .block_on(deduce_bucketed_mosaics(
                    mosaics,
                    rectangle,
                    params.tile_params,
                    params.bucket_delta,
                    params.trace_params,
                ))
                .expect("object bucket benchmark should succeed")
        },
    );

    group.finish();
}

criterion_group! {
    name = benches;
    config = criterion_config();
    targets = pipeline_benchmarks
}
criterion_main!(benches);

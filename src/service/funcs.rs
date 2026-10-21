#![warn(clippy::pedantic)]
#![allow(
    clippy::missing_errors_doc,
    clippy::missing_panics_doc,
    clippy::too_many_lines,
    clippy::unused_async_trait_impl
)]

use crate::bucketed_mosaics::BucketedMosaics;
use crate::eye::deduce_bucketed_mosaics;
use crate::eye::deduce_rectangles;
use crate::math::AnonymizedCoordinateSystem;
use crate::math::CoordinateSystem;
use crate::mosaics::AnonymizedMosaic;
use crate::mosaics::Results;
use crate::mosaics::deduce_mosaics;
use crate::object_detection::detect_objects;
use crate::slices;
use crate::slices::BasicParams;
use crate::slices::Color;
use crate::slices::Rectangle;
use crate::slices::WrappedRgbImage;
use crate::slices::calculate_slices;
use crate::slices::find_connected_slices;
use num_traits::ToPrimitive;
use rs_math3d::Vec3d;
use std::sync::Arc;
use tokio::task::JoinSet;

use super::{
    Circle, ColoredAnonymizedMosaic, EnrichedMosaic, EyeSession, ObjectSession, OrdinarySession,
    Point, RgbColor, Slice, SliceLine,
};

pub(super) fn usize_from_u32(value: u32) -> usize {
    value
        .to_usize()
        .expect("image dimension should fit within usize")
}

pub(super) fn f64_from_usize(value: usize) -> f64 {
    value
        .to_f64()
        .expect("image dimension should fit within f64")
}

pub(super) fn u8_from_f64(value: f64) -> u8 {
    value
        .round()
        .to_u8()
        .expect("average color channel should fit within u8")
}

pub(super) fn calculate_ordinary_mosaics(
    basic_params: BasicParams,
    results: Results,
    image: &WrappedRgbImage,
) -> Vec<AnonymizedMosaic> {
    let width = usize_from_u32(image.width());
    let height = usize_from_u32(image.height());
    let rectangle = Rectangle::new(
        Vec3d::new(0.0, 0.0, 0.0),
        Vec3d::new(f64_from_usize(width), f64_from_usize(height), 0.0),
    );
    let slices = calculate_slices(image, rectangle, basic_params);
    let connected_slices = find_connected_slices(&mut slices.clone());
    deduce_mosaics(
        connected_slices.clone(),
        &crate::math::Rectangle::new(rectangle.get_top_left(), rectangle.get_bottom_right()),
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

pub(super) fn deduce_enriched_mosaic(
    anonymized_mosaic: &AnonymizedMosaic,
    color: Color,
    label: Option<String>,
    results: Results,
) -> EnrichedMosaic {
    let mosaic = anonymized_mosaic.get_mosaic();
    let slice_matrix = mosaic.get_slice_matrix();
    let global_coordinate_system = AnonymizedCoordinateSystem::Direct(CoordinateSystem::new(
        Vec3d::new(0.0, 0.0, 0.0),
        Vec3d::new(1.0, 0.0, 0.0),
        Vec3d::new(0.0, 1.0, 0.0),
    ));
    let slice_matrix_output = slice_matrix
        .get_slice_lines()
        .into_iter()
        .map(|slice_line| SliceLine {
            line_number: slice_line.get_line_number(),
            slices: slice_line
                .get_slices()
                .iter()
                .map(|slice| Slice {
                    start: Point {
                        x: slice
                            .get_start()
                            .convert_to(global_coordinate_system.clone())
                            .get_x(),
                        y: slice
                            .get_start()
                            .convert_to(global_coordinate_system.clone())
                            .get_y(),
                    },
                    end: Point {
                        x: slice
                            .get_end()
                            .convert_to(global_coordinate_system.clone())
                            .get_x(),
                        y: slice
                            .get_end()
                            .convert_to(global_coordinate_system.clone())
                            .get_y(),
                    },
                })
                .collect(),
        })
        .collect();
    let bounding_box = anonymized_mosaic.get_bounding_box();
    let bounding_circle = anonymized_mosaic.get_bounding_circle();
    let area = anonymized_mosaic.get_area();
    let global_bounding_box = bounding_box.to_global_rectangle();
    let global_bounding_circle_center = bounding_circle
        .get_center()
        .convert_to(global_coordinate_system.clone())
        .get_local_point();
    let global_center_of_mass = anonymized_mosaic
        .get_center_of_mass()
        .convert_to(global_coordinate_system.clone());
    EnrichedMosaic {
        bounding_box: slices::Rectangle::new_from_math_rectangle(&global_bounding_box),
        bounding_circle: Circle {
            center: global_bounding_circle_center,
            radius: bounding_circle.get_radius(),
        },
        color,
        label,
        area,
        center_of_mass: global_center_of_mass.get_local_point(),
        slice_matrix: slice_matrix_output,
        average_color: RgbColor {
            red: u8_from_f64(mosaic.get_average_color().x),
            green: u8_from_f64(mosaic.get_average_color().y),
            blue: u8_from_f64(mosaic.get_average_color().z),
        },
        results,
    }
}

pub(super) fn calculate_ordinary(
    ordinary_session: &OrdinarySession,
    image: &WrappedRgbImage,
) -> Vec<ColoredAnonymizedMosaic> {
    let mosaics = calculate_ordinary_mosaics(
        ordinary_session.basic_params,
        ordinary_session.results,
        image,
    );
    mosaics
        .into_iter()
        .map(|mosaic| ColoredAnonymizedMosaic::new(Color::Green, None, mosaic.clone()))
        .collect()
}

pub(super) async fn calculate_eye(
    eye_session: &EyeSession,
    image: WrappedRgbImage,
) -> Result<(Vec<ColoredAnonymizedMosaic>, BucketedMosaics), String> {
    let image_width = f64::from(image.width());
    let image_height = f64::from(image.height());
    let surrounding_rectangle = Rectangle::new(
        Vec3d::new(0.0, 0.0, 0.0),
        Vec3d::new(image_width, image_height, 0.0),
    );
    let current_mosaics =
        calculate_ordinary_mosaics(eye_session.basic_params, eye_session.results, &image);
    let current_bucketed_mosaics = deduce_bucketed_mosaics(
        current_mosaics.clone(),
        surrounding_rectangle,
        eye_session.eye_params.tile_params,
        eye_session.eye_params.bucket_delta,
        eye_session.eye_params.trace_params,
    )
    .await?;
    let Some(previous_bucketed_mosaics) = eye_session.cached_previous_mosaics.clone() else {
        return Ok((
            current_mosaics
                .into_iter()
                .map(|mosaic| ColoredAnonymizedMosaic::new(Color::Blue, None, mosaic))
                .collect(),
            current_bucketed_mosaics,
        ));
    };
    let rectangles = deduce_rectangles(
        previous_bucketed_mosaics,
        current_mosaics,
        eye_session.eye_params,
        surrounding_rectangle,
    )
    .await?;
    Ok((
        rectangles
            .into_iter()
            .map(|colored_rectangle| {
                ColoredAnonymizedMosaic::new(
                    colored_rectangle.get_color(),
                    None,
                    colored_rectangle.get_mosaics()[0].clone(),
                )
            })
            .collect(),
        current_bucketed_mosaics,
    ))
}

pub(super) async fn calculate_object(
    object_session: &ObjectSession,
    image: WrappedRgbImage,
) -> Result<Vec<ColoredAnonymizedMosaic>, String> {
    let image_width = f64::from(image.width());
    let image_height = f64::from(image.height());
    let surrounding_rectangle = Rectangle::new(
        Vec3d::new(0.0, 0.0, 0.0),
        Vec3d::new(image_width, image_height, 0.0),
    );
    let current_mosaics =
        calculate_ordinary_mosaics(object_session.basic_params, object_session.results, &image);
    let bucketed_mosaics = Arc::new(
        deduce_bucketed_mosaics(
            current_mosaics.clone(),
            surrounding_rectangle,
            object_session.object_detection_params.tile_params,
            object_session.object_detection_params.bucket_delta,
            object_session.object_detection_params.trace_params,
        )
        .await?,
    );
    let object_detection_params = Arc::new(object_session.object_detection_params);
    let reference_objects = object_session.objects_to_be_detected.clone();
    let mut join_set = JoinSet::new();

    for (index, reference_object) in reference_objects.into_iter().enumerate() {
        let bucketed_mosaics = Arc::clone(&bucketed_mosaics);
        let object_detection_params = Arc::clone(&object_detection_params);
        let results = object_session.results;
        let object_id = reference_object.get_id();
        join_set.spawn_blocking(move || {
            (
                index,
                object_id,
                detect_objects(
                    &reference_object,
                    bucketed_mosaics.as_ref(),
                    *object_detection_params.as_ref(),
                    surrounding_rectangle,
                    results,
                ),
            )
        });
    }

    let mut indexed_rectangles = Vec::new();
    while let Some(joined_result) = join_set.join_next().await {
        let indexed_group = joined_result
            .map_err(|error| format!("failed to join object detection task: {error}"))?;
        indexed_rectangles.push(indexed_group);
    }
    indexed_rectangles.sort_by_key(|(index, _, _)| *index);

    let rectangles: Vec<_> = indexed_rectangles
        .into_iter()
        .flat_map(|(_, object_id, rectangles)| {
            rectangles
                .into_iter()
                .map(move |colored_rectangle| (object_id.clone(), colored_rectangle))
        })
        .collect();
    Ok(rectangles
        .into_iter()
        .map(|(object_id, colored_rectangle)| {
            ColoredAnonymizedMosaic::new(
                colored_rectangle.get_color(),
                Some(object_id),
                colored_rectangle.get_mosaics()[0].clone(),
            )
        })
        .collect())
}

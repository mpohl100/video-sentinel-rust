#![warn(clippy::pedantic)]
#![allow(
    clippy::missing_errors_doc,
    clippy::missing_panics_doc,
    clippy::too_many_lines,
    clippy::unused_async_trait_impl
)]

use crate::bucketed_mosaics::BucketedMosaics;
use crate::mosaics::AnonymizedMosaic;
use crate::mosaics::ColoredRectangle;
use crate::mosaics::Results;
use crate::slices::Color;
use crate::slices::{Rectangle, WrappedRelativeRectangle};
use crate::traces::Trace;
#[cfg(test)]
use num_traits::ToPrimitive;
use rs_math3d::Vec3d;

use super::{ObjectDetectionParams, ReferenceObject};

#[cfg(test)]
pub(super) fn i32_from_f64(value: f64) -> i32 {
    value
        .round()
        .to_i32()
        .expect("fixture coordinate should fit within i32")
}

#[must_use]
pub fn detect_objects(
    reference_object: &ReferenceObject,
    bucketed_mosaics: &BucketedMosaics,
    object_detection_params: ObjectDetectionParams,
    surrounding_rectangle: Rectangle,
    results: Results,
) -> Vec<ColoredRectangle> {
    let first_reference_mosaic = reference_object.get_mosaics(1);
    if first_reference_mosaic.is_empty() {
        return Vec::new();
    }
    let traced_biggest_mosaic = first_reference_mosaic[0].clone();
    let biggest_trace = traced_biggest_mosaic.get_trace();
    let biggest_candidates = bucketed_mosaics.get_all_similar_mosaics(&traced_biggest_mosaic);
    let mut candidates = biggest_candidates
        .into_iter()
        .filter(|candidate| {
            let candidate_trace = candidate.get_trace();
            candidate_trace.compare_with(object_detection_params.target_similarity, biggest_trace)
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
        let traced_current_mosaic = current_reference_object.get_mosaics(i + 1)[i].clone();
        let current_trace = traced_current_mosaic.get_trace();
        for candidate in candidates {
            let absolute_rectangle = combine_boxes(
                candidate
                    .get_mosaics(usize::MAX)
                    .iter()
                    .map(|mosaic| {
                        Rectangle::new_from_math_rectangle(
                            &mosaic
                                .get_relative_mosaic()
                                .get_bounding_box()
                                .to_global_rectangle(),
                        )
                    })
                    .collect(),
            );
            let suspected_region = relative_rectangle.multiply_with_rectangle(absolute_rectangle);
            let inverted_suspected_region =
                inverted_relative_rectangle.multiply_with_rectangle(absolute_rectangle);
            let combined_regions = combine_boxes(vec![
                suspected_region,
                absolute_rectangle,
                inverted_suspected_region,
            ]);
            let relative_combined_region = WrappedRelativeRectangle::new_from_rectangles(
                combined_regions,
                surrounding_rectangle,
            );
            let next_mosaic_candidates = bucketed_mosaics.get_similar_mosaics_from_rectangle(
                &traced_current_mosaic.clone(),
                &relative_combined_region,
            );
            let real_candidates: Vec<_> = next_mosaic_candidates
                .into_iter()
                .filter(|next_mosaic_candidate| {
                    let next_candidate_trace = next_mosaic_candidate.get_trace();
                    next_candidate_trace
                        .compare_with(object_detection_params.target_similarity, current_trace)
                        >= object_detection_params.target_similarity
                })
                .collect();

            for real_candidate in real_candidates {
                let mut candidate_mosaics = candidate.get_mosaics(i + 1);
                candidate_mosaics.push(real_candidate);
                let current_candidate_reference_object =
                    ReferenceObject::new("dummy_id".to_string(), candidate_mosaics);
                let current_candidate_mosaics: Vec<_> = current_candidate_reference_object
                    .get_mosaics(usize::MAX)
                    .into_iter()
                    .map(|mosaic| mosaic.get_relative_mosaic().get_mosaic())
                    .collect();
                let current_candidate_reference_object_trace = Trace::new_from_mosaics(
                    &current_candidate_mosaics,
                    object_detection_params.trace_params,
                );
                if current_candidate_reference_object_trace.compare_with(
                    object_detection_params.target_similarity,
                    &{
                        let current_reference_mosaics: Vec<_> = current_reference_object
                            .get_mosaics(usize::MAX)
                            .into_iter()
                            .map(|mosaic| mosaic.get_relative_mosaic().get_mosaic())
                            .collect();
                        Trace::new_from_mosaics(
                            &current_reference_mosaics,
                            object_detection_params.trace_params,
                        )
                    },
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
                        Rectangle::new_from_math_rectangle(&match results {
                            Results::Absolute => mosaic
                                .get_relative_mosaic()
                                .get_mosaic()
                                .get_bounding_box()
                                .to_global_rectangle(),
                            Results::Relative => mosaic
                                .get_relative_mosaic()
                                .get_bounding_box()
                                .to_global_rectangle(),
                        })
                    })
                    .collect(),
            );
            ColoredRectangle::new(
                bounding_box,
                Color::Green,
                candidate
                    .get_mosaics(usize::MAX)
                    .into_iter()
                    .map(|mosaic| {
                        AnonymizedMosaic::new(
                            results,
                            mosaic.get_relative_mosaic().get_mosaic(),
                            Some(mosaic.get_relative_mosaic()),
                        )
                    })
                    .collect(),
            )
        })
        .collect()
}

pub(super) fn combine_boxes(boxes: Vec<Rectangle>) -> Rectangle {
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

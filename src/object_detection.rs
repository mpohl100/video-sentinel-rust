use core::panic;

use rs_math3d::Vec3d;

use crate::bucketed_mosaics::BucketedMosaics;
use crate::eye::TileParams;
use crate::mosaics::WrappedMosaic;
use crate::slices::Color;
use crate::slices::RelativeRectangle;
use crate::slices::{ColoredRectangle, Rectangle};
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
        self.mosaics[..until_index].to_vec()
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
) -> Vec<ColoredRectangle> {
    let biggest_mosaic = reference_object.get_mosaics(1)[0].clone();
    let biggest_trace = Trace::new_from_mosaic(
        biggest_mosaic.clone(),
        object_detection_params.trace_params.clone(),
    );
    let biggest_candidates = bucketed_mosaics.get_all_similar_mosaics(&biggest_mosaic);
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
        let current_reference_object = ReferenceObject::new("dummy_id".to_string(), reference_object.get_mosaics(i + 1));
        let relative_rectangle = current_reference_object.get_relative_rectangle_to_smallest();
        let inverted_relative_rectangle = relative_rectangle.invert();
        let mut new_candidate_reference_objects = Vec::new();
        let current_mosaic = current_reference_object.get_mosaics(i + 1)[i].clone();
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
            let next_mosaic_candidates = bucketed_mosaics
                .get_similar_mosaics_from_rectangle(&current_mosaic.clone(), combined_regions);
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
                let current_candidate_reference_object = ReferenceObject::new("dummy_id".to_string(), candidate_mosaics);
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
            ColoredRectangle::new(bounding_box, Color::Green, candidate.get_mosaics(usize::MAX))
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

#![warn(clippy::pedantic)]
#![allow(
    clippy::missing_errors_doc,
    clippy::missing_panics_doc,
    clippy::too_many_lines,
    clippy::unused_async_trait_impl
)]

use crate::bucketed_mosaics::BucketedMosaics;
use crate::math::Rectangle as MathRectangle;
use crate::mosaics::AnonymizedMosaic;
use crate::mosaics::WrappedRelativeMosaic;
use crate::slices::Color;
use crate::mosaics::ColoredRectangle;
use crate::slices::Rectangle;
use crate::traced_mosaics::TracedRelativeMosaic;
use crate::traces::TraceParams;
#[cfg(test)]
use num_traits::ToPrimitive;
use std::sync::Arc;
use tokio::task::JoinSet;

use super::{EyeParams, TileParams};

#[cfg(test)]
pub(super) fn f64_from_usize(value: usize) -> f64 {
    value.to_f64().expect("fixture index should fit within f64")
}

pub async fn deduce_bucketed_mosaics(
    mosaics: Vec<AnonymizedMosaic>,
    surrounding_rectangle: Rectangle,
    tile_params: TileParams,
    bucket_delta: f64,
    trace_params: TraceParams,
) -> Result<BucketedMosaics, String> {
    let mut bucketed_mosaics = BucketedMosaics::new(tile_params, bucket_delta);
    let absolute_rectangle = MathRectangle::new(
        surrounding_rectangle.get_top_left(),
        surrounding_rectangle.get_bottom_right(),
    );
    let mut join_set = JoinSet::new();

    for (index, mosaic) in mosaics.into_iter().enumerate() {
        let absolute_rectangle = absolute_rectangle.clone();
        join_set.spawn_blocking(move || {
            (
                index,
                TracedRelativeMosaic::new(
                    WrappedRelativeMosaic::new(mosaic.get_mosaic(), absolute_rectangle),
                    trace_params,
                ),
            )
        });
    }

    let mut traced_mosaics = Vec::new();
    while let Some(joined_result) = join_set.join_next().await {
        let indexed_traced_mosaic = joined_result
            .map_err(|error| format!("failed to join bucketed mosaic task: {error}"))?;
        traced_mosaics.push(indexed_traced_mosaic);
    }
    traced_mosaics.sort_by_key(|(index, _)| *index);

    for (_, traced_mosaic) in traced_mosaics {
        bucketed_mosaics.add_mosaic(&traced_mosaic);
    }

    Ok(bucketed_mosaics)
}

pub async fn deduce_rectangles(
    previous_bucketed_mosaics: BucketedMosaics,
    next_mosaics: Vec<AnonymizedMosaic>,
    eye_params: EyeParams,
    surrounding_rectangle: Rectangle,
) -> Result<Vec<ColoredRectangle>, String> {
    let absolute_rectangle = MathRectangle::new(
        surrounding_rectangle.get_top_left(),
        surrounding_rectangle.get_bottom_right(),
    );
    let previous_bucketed_mosaics = Arc::new(previous_bucketed_mosaics);
    let eye_params = Arc::new(eye_params);
    let mut join_set = JoinSet::new();

    for (index, next_mosaic) in next_mosaics.into_iter().enumerate() {
        let previous_bucketed_mosaics = Arc::clone(&previous_bucketed_mosaics);
        let eye_params = Arc::clone(&eye_params);
        let absolute_rectangle = absolute_rectangle.clone();
        join_set.spawn_blocking(move || {
            (
                index,
                deduce_rectangle_for_next_mosaic(
                    previous_bucketed_mosaics.as_ref(),
                    next_mosaic,
                    eye_params.as_ref(),
                    absolute_rectangle,
                ),
            )
        });
    }

    let mut indexed_results = Vec::new();
    while let Some(joined_result) = join_set.join_next().await {
        let indexed_rectangle =
            joined_result.map_err(|error| format!("failed to join eye mosaic task: {error}"))?;
        indexed_results.push(indexed_rectangle);
    }
    indexed_results.sort_by_key(|(index, _)| *index);
    Ok(indexed_results
        .into_iter()
        .map(|(_, rectangle)| rectangle)
        .collect())
}

pub(super) fn deduce_rectangle_for_next_mosaic(
    previous_bucketed_mosaics: &BucketedMosaics,
    next_mosaic: AnonymizedMosaic,
    eye_params: &EyeParams,
    absolute_rectangle: MathRectangle,
) -> ColoredRectangle {
    let wrapped_next_mosaic =
        WrappedRelativeMosaic::new(next_mosaic.get_mosaic(), absolute_rectangle);
    let traced_next_mosaic =
        TracedRelativeMosaic::new(wrapped_next_mosaic.clone(), eye_params.trace_params);
    let potentially_similar_mosaics =
        previous_bucketed_mosaics.get_potentially_similar_mosaics(&traced_next_mosaic);
    let mut current_color = Color::Red;
    for previous_mosaic in potentially_similar_mosaics {
        if are_mosaics_similar(
            &previous_mosaic,
            &traced_next_mosaic,
            eye_params.target_similarity,
        ) {
            let color = deduce_color(
                Rectangle::new_from_math_rectangle(
                    &previous_mosaic
                        .get_relative_mosaic()
                        .get_bounding_box()
                        .to_global_rectangle(),
                ),
                Rectangle::new_from_math_rectangle(
                    &traced_next_mosaic
                        .get_relative_mosaic()
                        .get_bounding_box()
                        .to_global_rectangle(),
                ),
            );
            if current_color != Color::Blue {
                current_color = color;
            }
        }
    }
    ColoredRectangle::new(
        Rectangle::new_from_math_rectangle(&next_mosaic.get_bounding_box().to_global_rectangle()),
        current_color,
        vec![next_mosaic],
    )
}

pub(super) fn deduce_color(
    previous_bounding_box: Rectangle,
    next_bounding_box: Rectangle,
) -> Color {
    let _overlaps = previous_bounding_box.overlaps(&next_bounding_box);
    Color::Blue
}

pub(super) fn are_mosaics_similar(
    mosaic1: &TracedRelativeMosaic,
    mosaic2: &TracedRelativeMosaic,
    target_similarity: f64,
) -> bool {
    let result = mosaic1
        .get_trace()
        .compare_with(target_similarity, mosaic2.get_trace());
    result >= target_similarity
}

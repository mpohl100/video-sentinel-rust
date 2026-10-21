#![warn(clippy::pedantic)]
#![allow(
    clippy::missing_errors_doc,
    clippy::missing_panics_doc,
    clippy::too_many_lines,
    clippy::unused_async_trait_impl
)]

use crate::service::EnrichedMosaic;

use super::{CircleGql, ColorGql, RectangleGql, ResultsGql, RgbColorGql, SliceLineGql, Vec3Gql};

#[derive(async_graphql::SimpleObject, Clone)]
pub struct EnrichedMosaicGql {
    bounding_box: RectangleGql,
    bounding_circle: CircleGql,
    color: ColorGql,
    label: Option<String>,
    area: f64,
    center_of_mass: Vec3Gql,
    slice_matrix: Vec<SliceLineGql>,
    average_color: RgbColorGql,
    results: ResultsGql,
}

impl From<EnrichedMosaic> for EnrichedMosaicGql {
    fn from(value: EnrichedMosaic) -> Self {
        Self {
            bounding_box: value.bounding_box.into(),
            bounding_circle: value.bounding_circle.into(),
            color: value.color.into(),
            label: value.label,
            area: value.area,
            center_of_mass: value.center_of_mass.into(),
            slice_matrix: value.slice_matrix.into_iter().map(Into::into).collect(),
            average_color: value.average_color.into(),
            results: value.results.into(),
        }
    }
}

#![warn(clippy::pedantic)]
#![allow(
    clippy::missing_errors_doc,
    clippy::missing_panics_doc,
    clippy::too_many_lines,
    clippy::unused_async_trait_impl
)]

use crate::object_detection::ReferenceObject;

use super::RectangleGql;

#[derive(async_graphql::SimpleObject, Clone)]
pub struct ReferenceObjectGql {
    object_id: String,
    surrounding_bounding_box: RectangleGql,
}

impl From<ReferenceObject> for ReferenceObjectGql {
    fn from(value: ReferenceObject) -> Self {
        Self {
            object_id: value.get_id(),
            surrounding_bounding_box: value.get_surrounding_bounding_box().into(),
        }
    }
}

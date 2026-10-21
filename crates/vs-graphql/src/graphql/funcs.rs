#![warn(clippy::pedantic)]
#![allow(
    clippy::missing_errors_doc,
    clippy::missing_panics_doc,
    clippy::too_many_lines,
    clippy::unused_async_trait_impl
)]

use crate::slices::WrappedRgbImage;
use async_graphql::{Context, EmptySubscription, Error, Result, Schema, Upload};
use std::io::Read;

use super::{GraphqlAppState, MutationRoot, QueryRoot};

pub type VideoSentinelSchema = Schema<QueryRoot, MutationRoot, EmptySubscription>;

#[must_use]
pub fn create_schema() -> VideoSentinelSchema {
    Schema::build(QueryRoot, MutationRoot, EmptySubscription)
        .data(GraphqlAppState::default())
        .finish()
}

pub(super) fn app_state(ctx: &Context<'_>) -> Result<GraphqlAppState> {
    ctx.data::<GraphqlAppState>()
        .cloned()
        .map_err(|_| Error::new("GraphQL app state not available"))
}

pub(super) fn read_upload_bytes(ctx: &Context<'_>, upload: Upload) -> Result<Vec<u8>> {
    let upload_value = upload.value(ctx)?;
    let mut content = upload_value.content;
    let mut bytes = Vec::new();
    content
        .read_to_end(&mut bytes)
        .map_err(|error| Error::new(format!("Failed to read uploaded file: {error}")))?;
    Ok(bytes)
}

pub(super) fn wrapped_rgb_image_from_jpeg(bytes: &[u8]) -> Result<WrappedRgbImage> {
    WrappedRgbImage::from_jpeg_bytes(bytes)
        .map_err(|error| Error::new(format!("Failed to decode JPEG upload: {error}")))
}

pub(super) fn message_error(message: &str) -> Error {
    Error::new(message)
}

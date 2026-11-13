#![warn(clippy::pedantic)]
#![allow(
    clippy::missing_errors_doc,
    clippy::missing_panics_doc,
    clippy::too_many_lines,
    clippy::unused_async_trait_impl
)]

use crate::service::{
    AddColorFilterResult, AddObjectToBeDetectedResult, BucketDeltaUpdateResult,
    CreateEyeSessionResult, CreateObjectSessionResult, CreateOrdinarySessionResult,
    CreateThreeDTriangleSessionResult, DeleteReferenceObjectResult, DeleteSessionResult,
    EyeParamsUpdateResult, GetRectanglesResult, ObjectDetectionParamsUpdateResult,
    TargetSimilarityUpdateResult, TileParamsUpdateResult, TraceParamsUpdateResult,
    UpdateBasicParamsResult,
};
use crate::service::{
    ProcessThreeDTriangleFrameResult, ThreeDTriangleCalibration, ThreeDTriangleFrame,
};
use async_graphql::{Context, Object, Result, Upload};

use super::{
    BasicParamsInputGql, CameraIntrinsicsInputGql, DepthEncodingGql, EnrichedMosaicGql,
    EyeParamsInputGql, ObjectDetectionParamsInputGql, RectangleInputGql, ResultsGql,
    RgbColorInputGql, RigidTransformInputGql, TileParamsInputGql, TraceParamsInputGql, TriangleGql,
    app_state, message_error, read_upload_bytes, wrapped_rgb_image_from_jpeg,
};

pub struct MutationRoot;

#[Object]
impl MutationRoot {
    async fn create_three_d_triangle_session(
        &self,
        ctx: &Context<'_>,
        session_id: String,
        depth_camera: CameraIntrinsicsInputGql,
        rgb_camera: CameraIntrinsicsInputGql,
        depth_to_rgb: RigidTransformInputGql,
    ) -> Result<bool> {
        let service = app_state(ctx)?.service;
        let mut service = service.lock().await;
        match service.create_three_d_triangle_session(
            &session_id,
            Some(ThreeDTriangleCalibration {
                depth_camera: depth_camera.into(),
                rgb_camera: rgb_camera.into(),
                depth_to_rgb: depth_to_rgb.into(),
            }),
        ) {
            CreateThreeDTriangleSessionResult::Success => Ok(true),
            CreateThreeDTriangleSessionResult::SessionAlreadyExists => {
                Err(message_error("Session already exists"))
            }
            CreateThreeDTriangleSessionResult::InvalidInput(error) => Err(message_error(&error)),
        }
    }

    #[allow(clippy::too_many_arguments)]
    async fn get_three_d_triangles(
        &self,
        ctx: &Context<'_>,
        session_id: String,
        rgb: Upload,
        depth: Upload,
        depth_width: u32,
        depth_height: u32,
        depth_encoding: DepthEncodingGql,
        #[graphql(default = false)] depth_is_bigendian: bool,
    ) -> Result<Vec<TriangleGql>> {
        let rgb_bytes = read_upload_bytes(ctx, rgb)?;
        let depth_bytes = read_upload_bytes(ctx, depth)?;
        let service = app_state(ctx)?.service;
        let request = service
            .lock()
            .await
            .get_triangles_request(&session_id, None);
        let Some(request) = request else {
            return Err(message_error(
                "Session not found or calibration unavailable",
            ));
        };
        match request
            .get_triangles_with(move || {
                let rgb = wrapped_rgb_image_from_jpeg(&rgb_bytes).map_err(|error| error.message)?;
                let depth = super::depth_upload::depth_frame_from_bytes(
                    depth_width,
                    depth_height,
                    depth_encoding,
                    depth_is_bigendian,
                    &depth_bytes,
                )
                .map_err(|error| error.message)?;
                Ok(ThreeDTriangleFrame { rgb, depth })
            })
            .await
        {
            ProcessThreeDTriangleFrameResult::Success(triangles) => {
                Ok(triangles.into_iter().map(Into::into).collect())
            }
            ProcessThreeDTriangleFrameResult::SessionNotFound => {
                Err(message_error("Session not found"))
            }
            ProcessThreeDTriangleFrameResult::InvalidInput(error) => Err(message_error(&error)),
        }
    }

    async fn get_rectangles(
        &self,
        ctx: &Context<'_>,
        session_id: String,
        image: Upload,
    ) -> Result<Vec<EnrichedMosaicGql>> {
        let image_bytes = read_upload_bytes(ctx, image)?;
        let service = app_state(ctx)?.service;
        let request = service.lock().await.get_rectangles_request(&session_id);
        let Some(request) = request else {
            return Err(message_error("Session not found"));
        };

        match request
            .get_rectangles_with(move || {
                wrapped_rgb_image_from_jpeg(&image_bytes).map_err(|error| error.message)
            })
            .await
        {
            GetRectanglesResult::Success(mosaics) => {
                Ok(mosaics.into_iter().map(Into::into).collect())
            }

            GetRectanglesResult::SessionNotFound => Err(message_error("Session not found")),
            GetRectanglesResult::ComputationFailed(error) => Err(message_error(&error)),
        }
    }

    async fn create_ordinary_session(
        &self,
        ctx: &Context<'_>,
        session_id: String,
        basic_params: BasicParamsInputGql,
        results: ResultsGql,
    ) -> Result<bool> {
        let service = app_state(ctx)?.service;
        let mut service = service.lock().await;
        match service.create_ordinary_session(&session_id, basic_params.into(), results.into()) {
            CreateOrdinarySessionResult::Success => Ok(true),
            CreateOrdinarySessionResult::SessionAlreadyExists => {
                Err(message_error("Session already exists"))
            }
        }
    }

    async fn create_eye_session(
        &self,
        ctx: &Context<'_>,
        session_id: String,
        basic_params: BasicParamsInputGql,
        eye_params: EyeParamsInputGql,
        results: ResultsGql,
    ) -> Result<bool> {
        let service = app_state(ctx)?.service;
        let mut service = service.lock().await;
        match service.create_eye_session(
            &session_id,
            basic_params.into(),
            eye_params.into(),
            results.into(),
        ) {
            CreateEyeSessionResult::Success => Ok(true),
            CreateEyeSessionResult::SessionAlreadyExists => {
                Err(message_error("Session already exists"))
            }
        }
    }

    async fn add_color_filter(
        &self,
        ctx: &Context<'_>,
        session_id: String,
        color: RgbColorInputGql,
        threshold_for_color: RgbColorInputGql,
    ) -> Result<bool> {
        let service = app_state(ctx)?.service;
        let mut service = service.lock().await;
        match service.add_color_filter(&session_id, color.into(), threshold_for_color.into()) {
            AddColorFilterResult::Success => Ok(true),
            AddColorFilterResult::SessionNotFound => Err(message_error("Session not found")),
            AddColorFilterResult::SessionTypeDoesNotSupportColorFilters => {
                Err(message_error("Session type does not support color filters"))
            }
        }
    }

    async fn create_object_session(
        &self,
        ctx: &Context<'_>,
        session_id: String,
        basic_params: BasicParamsInputGql,
        object_detection_params: ObjectDetectionParamsInputGql,
        results: ResultsGql,
    ) -> Result<bool> {
        let service = app_state(ctx)?.service;
        let mut service = service.lock().await;
        match service.create_object_session(
            &session_id,
            basic_params.into(),
            object_detection_params.into(),
            results.into(),
        ) {
            CreateObjectSessionResult::Success => Ok(true),
            CreateObjectSessionResult::SessionAlreadyExists => {
                Err(message_error("Session already exists"))
            }
        }
    }

    async fn update_basic_params(
        &self,
        ctx: &Context<'_>,
        session_id: String,
        basic_params: BasicParamsInputGql,
    ) -> Result<bool> {
        let service = app_state(ctx)?.service;
        let mut service = service.lock().await;
        match service.update_basic_params(&session_id, basic_params.into()) {
            UpdateBasicParamsResult::Success => Ok(true),
            UpdateBasicParamsResult::SessionNotFound => Err(message_error("Session not found")),
        }
    }

    async fn update_tile_params(
        &self,
        ctx: &Context<'_>,
        session_id: String,
        tile_params: TileParamsInputGql,
    ) -> Result<bool> {
        let service = app_state(ctx)?.service;
        let mut service = service.lock().await;
        match service.update_tile_params(&session_id, tile_params.into()) {
            TileParamsUpdateResult::Success => Ok(true),
            TileParamsUpdateResult::SessionNotFound => Err(message_error("Session not found")),
            TileParamsUpdateResult::SessionTypeDoesNotSupportTileParams => {
                Err(message_error("Session type does not support tile params"))
            }
        }
    }

    async fn update_trace_params(
        &self,
        ctx: &Context<'_>,
        session_id: String,
        trace_params: TraceParamsInputGql,
    ) -> Result<bool> {
        let service = app_state(ctx)?.service;
        let mut service = service.lock().await;
        match service.update_trace_params(&session_id, trace_params.into()) {
            TraceParamsUpdateResult::Success => Ok(true),
            TraceParamsUpdateResult::SessionNotFound => Err(message_error("Session not found")),
            TraceParamsUpdateResult::SessionTypeDoesNotSupportTraceParams => {
                Err(message_error("Session type does not support trace params"))
            }
        }
    }

    async fn update_bucket_delta(
        &self,
        ctx: &Context<'_>,
        session_id: String,
        bucket_delta: f64,
    ) -> Result<bool> {
        let service = app_state(ctx)?.service;
        let mut service = service.lock().await;
        match service.update_bucket_delta(&session_id, bucket_delta) {
            BucketDeltaUpdateResult::Success => Ok(true),
            BucketDeltaUpdateResult::SessionNotFound => Err(message_error("Session not found")),
            BucketDeltaUpdateResult::SessionTypeDoesNotSupportBucketDelta => {
                Err(message_error("Session type does not support bucket delta"))
            }
        }
    }

    async fn update_target_similarity(
        &self,
        ctx: &Context<'_>,
        session_id: String,
        target_similarity: f64,
    ) -> Result<bool> {
        let service = app_state(ctx)?.service;
        let mut service = service.lock().await;
        match service.update_target_similarity(&session_id, target_similarity) {
            TargetSimilarityUpdateResult::Success => Ok(true),
            TargetSimilarityUpdateResult::SessionNotFound => {
                Err(message_error("Session not found"))
            }
            TargetSimilarityUpdateResult::SessionTypeDoesNotSupportTargetSimilarity => Err(
                message_error("Session type does not support target similarity"),
            ),
        }
    }

    async fn update_eye_params(
        &self,
        ctx: &Context<'_>,
        session_id: String,
        eye_params: EyeParamsInputGql,
    ) -> Result<bool> {
        let service = app_state(ctx)?.service;
        let mut service = service.lock().await;
        match service.update_eye_params(&session_id, eye_params.into()) {
            EyeParamsUpdateResult::Success => Ok(true),
            EyeParamsUpdateResult::SessionNotFound => Err(message_error("Session not found")),
            EyeParamsUpdateResult::SessionTypeDoesNotSupportEyeParams => {
                Err(message_error("Session type does not support eye params"))
            }
        }
    }

    async fn update_object_detection_params(
        &self,
        ctx: &Context<'_>,
        session_id: String,
        object_detection_params: ObjectDetectionParamsInputGql,
    ) -> Result<bool> {
        let service = app_state(ctx)?.service;
        let mut service = service.lock().await;
        match service.update_object_detection_params(&session_id, object_detection_params.into()) {
            ObjectDetectionParamsUpdateResult::Success => Ok(true),
            ObjectDetectionParamsUpdateResult::SessionNotFound => {
                Err(message_error("Session not found"))
            }
            ObjectDetectionParamsUpdateResult::SessionTypeDoesNotSupportObjectDetectionParams => {
                Err(message_error(
                    "Session type does not support object detection params",
                ))
            }
        }
    }

    async fn add_object_to_be_detected_as_image(
        &self,
        ctx: &Context<'_>,
        session_id: String,
        object_id: String,
        image: Upload,
        surrounding_rectangle: RectangleInputGql,
    ) -> Result<bool> {
        let image_bytes = read_upload_bytes(ctx, image)?;
        let wrapped_image = wrapped_rgb_image_from_jpeg(&image_bytes)?;

        let service = app_state(ctx)?.service;
        let mut service = service.lock().await;
        match service.add_object_to_be_detected_as_image(
            &session_id,
            &object_id,
            &wrapped_image,
            surrounding_rectangle.into(),
        ) {
            AddObjectToBeDetectedResult::Success => Ok(true),
            AddObjectToBeDetectedResult::SessionNotFound => Err(message_error("Session not found")),
            AddObjectToBeDetectedResult::NoReferenceMosaicsDetected => {
                Err(message_error("No reference mosaics detected"))
            }
            AddObjectToBeDetectedResult::SessionTypeDoesNotSupportAddingObjectToBeDetected => Err(
                message_error("Session type does not support adding object to be detected"),
            ),
        }
    }

    async fn add_object_to_be_detected_as_ascii_art(
        &self,
        ctx: &Context<'_>,
        session_id: String,
        object_id: String,
        ascii_art: String,
    ) -> Result<bool> {
        let service = app_state(ctx)?.service;
        let mut service = service.lock().await;
        match service.add_object_to_be_detected_as_ascii_art(&session_id, &object_id, &ascii_art) {
            AddObjectToBeDetectedResult::Success => Ok(true),
            AddObjectToBeDetectedResult::SessionNotFound => Err(message_error("Session not found")),
            AddObjectToBeDetectedResult::NoReferenceMosaicsDetected => {
                Err(message_error("No reference mosaics detected"))
            }
            AddObjectToBeDetectedResult::SessionTypeDoesNotSupportAddingObjectToBeDetected => Err(
                message_error("Session type does not support adding object to be detected"),
            ),
        }
    }

    async fn delete_session(&self, ctx: &Context<'_>, session_id: String) -> Result<bool> {
        let service = app_state(ctx)?.service;
        let mut service = service.lock().await;
        match service.delete_session(&session_id) {
            DeleteSessionResult::Success => Ok(true),
            DeleteSessionResult::SessionNotFound => Err(message_error("Session not found")),
        }
    }

    async fn delete_reference_object(
        &self,
        ctx: &Context<'_>,
        session_id: String,
        object_id: String,
    ) -> Result<bool> {
        let service = app_state(ctx)?.service;
        let mut service = service.lock().await;
        match service.delete_reference_object(&session_id, &object_id) {
            DeleteReferenceObjectResult::Success => Ok(true),
            DeleteReferenceObjectResult::SessionNotFound => Err(message_error("Session not found")),
            DeleteReferenceObjectResult::SessionTypeDoesNotSupportDeletingReferenceObject => Err(
                message_error("Session type does not support deleting reference object"),
            ),
            DeleteReferenceObjectResult::ReferenceObjectNotFound => {
                Err(message_error("Reference object not found"))
            }
        }
    }
}

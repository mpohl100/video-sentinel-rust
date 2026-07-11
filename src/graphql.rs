use std::io::Read;
use std::sync::Arc;

use async_graphql::{Context, EmptySubscription, Enum, Error, InputObject, Object, Result, Schema, Upload};
use rs_math3d::Vec3d;
use tokio::sync::Mutex;

use crate::object_detection::ReferenceObject;
use crate::service::{
    AddObjectToBeDetectedResult, BasicParamsInput, BucketDeltaUpdateResult, Circle,
    CreateEyeSessionResult, CreateObjectSessionResult, CreateOrdinarySessionResult,
    DeleteReferenceObjectResult, DeleteSessionResult, EnrichedMosaic, EyeParamsInput,
    EyeParamsUpdateResult, GetRectanglesResult, ObjectDetectionParamsInput,
    ObjectDetectionParamsUpdateResult, ObjectSession, Results, Service, SliceLine,
    TargetSimilarityUpdateResult, TileParamsInput, TileParamsUpdateResult, TraceParamsInput,
    TraceParamsUpdateResult, UpdateBasicParamsResult,
};
use crate::slices::{Color, Rectangle, WrappedRgbImage};

#[derive(Clone)]
pub struct GraphqlAppState {
    pub service: Arc<Mutex<Service>>,
}

impl Default for GraphqlAppState {
    fn default() -> Self {
        Self {
            service: Arc::new(Mutex::new(Service::new())),
        }
    }
}

pub type VideoSentinelSchema = Schema<QueryRoot, MutationRoot, EmptySubscription>;

pub fn create_schema() -> VideoSentinelSchema {
    Schema::build(QueryRoot, MutationRoot, EmptySubscription)
        .data(GraphqlAppState::default())
        .finish()
}

pub struct QueryRoot;
pub struct MutationRoot;

#[derive(Enum, Copy, Clone, Eq, PartialEq)]
pub enum ResultsGql {
    Absolute,
    Relative,
}

impl From<Results> for ResultsGql {
    fn from(value: Results) -> Self {
        match value {
            Results::Absolute => Self::Absolute,
            Results::Relative => Self::Relative,
        }
    }
}

impl From<ResultsGql> for Results {
    fn from(value: ResultsGql) -> Self {
        match value {
            ResultsGql::Absolute => Self::Absolute,
            ResultsGql::Relative => Self::Relative,
        }
    }
}

#[derive(Enum, Copy, Clone, Eq, PartialEq)]
pub enum ColorGql {
    Red,
    Green,
    Blue,
}

impl From<Color> for ColorGql {
    fn from(value: Color) -> Self {
        match value {
            Color::Red => Self::Red,
            Color::Green => Self::Green,
            Color::Blue => Self::Blue,
        }
    }
}

#[derive(async_graphql::SimpleObject, Clone)]
pub struct Vec3Gql {
    x: f64,
    y: f64,
    z: f64,
}

impl From<Vec3d> for Vec3Gql {
    fn from(value: Vec3d) -> Self {
        Self {
            x: value.x,
            y: value.y,
            z: value.z,
        }
    }
}

#[derive(async_graphql::SimpleObject, Clone)]
pub struct PointGql {
    x: f64,
    y: f64,
}

impl From<crate::service::Point> for PointGql {
    fn from(value: crate::service::Point) -> Self {
        Self {
            x: value.x,
            y: value.y,
        }
    }
}

#[derive(async_graphql::SimpleObject, Clone)]
pub struct SliceGql {
    start: PointGql,
    end: PointGql,
}

impl From<crate::service::Slice> for SliceGql {
    fn from(value: crate::service::Slice) -> Self {
        Self {
            start: value.start.into(),
            end: value.end.into(),
        }
    }
}

#[derive(async_graphql::SimpleObject, Clone)]
pub struct SliceLineGql {
    slices: Vec<SliceGql>,
    line_number: usize,
}

impl From<SliceLine> for SliceLineGql {
    fn from(value: SliceLine) -> Self {
        Self {
            slices: value.slices.into_iter().map(Into::into).collect(),
            line_number: value.line_number,
        }
    }
}

#[derive(async_graphql::SimpleObject, Clone)]
pub struct RgbColorGql {
    red: u8,
    green: u8,
    blue: u8,
}

impl From<crate::service::RgbColor> for RgbColorGql {
    fn from(value: crate::service::RgbColor) -> Self {
        Self {
            red: value.red,
            green: value.green,
            blue: value.blue,
        }
    }
}

#[derive(async_graphql::SimpleObject, Clone)]
pub struct RectangleGql {
    top_left: Vec3Gql,
    bottom_right: Vec3Gql,
}

impl From<Rectangle> for RectangleGql {
    fn from(value: Rectangle) -> Self {
        Self {
            top_left: value.get_top_left().into(),
            bottom_right: value.get_bottom_right().into(),
        }
    }
}

#[derive(async_graphql::SimpleObject, Clone)]
pub struct CircleGql {
    center: Vec3Gql,
    radius: f64,
}

impl From<Circle> for CircleGql {
    fn from(value: Circle) -> Self {
        Self {
            center: value.center.into(),
            radius: value.radius,
        }
    }
}

#[derive(async_graphql::SimpleObject, Clone)]
pub struct EnrichedMosaicGql {
    bounding_box: RectangleGql,
    bounding_circle: CircleGql,
    color: ColorGql,
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
            area: value.area,
            center_of_mass: value.center_of_mass.into(),
            slice_matrix: value.slice_matrix.into_iter().map(Into::into).collect(),
            average_color: value.average_color.into(),
            results: value.results.into(),
        }
    }
}

#[derive(async_graphql::SimpleObject, Clone)]
pub struct BasicParamsGql {
    do_grayscale: bool,
    gradient_threshold: u8,
}

impl From<crate::slices::BasicParams> for BasicParamsGql {
    fn from(value: crate::slices::BasicParams) -> Self {
        Self {
            do_grayscale: value.do_grayscale(),
            gradient_threshold: value.gradient_threshold(),
        }
    }
}

#[derive(async_graphql::SimpleObject, Clone)]
pub struct TileParamsGql {
    tile_x: f64,
    tile_y: f64,
}

impl From<crate::eye::TileParams> for TileParamsGql {
    fn from(value: crate::eye::TileParams) -> Self {
        Self {
            tile_x: value.relative_tile_x(),
            tile_y: value.relative_tile_y(),
        }
    }
}

#[derive(async_graphql::SimpleObject, Clone)]
pub struct TraceParamsGql {
    num_skeleton: usize,
    close_slice_threshold: f64,
}

impl From<crate::traces::TraceParams> for TraceParamsGql {
    fn from(value: crate::traces::TraceParams) -> Self {
        Self {
            num_skeleton: value.num_skeleton(),
            close_slice_threshold: value.close_slice_threshold(),
        }
    }
}

#[derive(async_graphql::SimpleObject, Clone)]
pub struct EyeParamsGql {
    tile_params: TileParamsGql,
    bucket_delta: f64,
    trace_params: TraceParamsGql,
    target_similarity: f64,
}

impl From<crate::eye::EyeParams> for EyeParamsGql {
    fn from(value: crate::eye::EyeParams) -> Self {
        Self {
            tile_params: value.tile_params.into(),
            bucket_delta: value.bucket_delta,
            trace_params: value.trace_params.into(),
            target_similarity: value.target_similarity,
        }
    }
}

#[derive(async_graphql::SimpleObject, Clone)]
pub struct ObjectDetectionParamsGql {
    tile_params: TileParamsGql,
    bucket_delta: f64,
    trace_params: TraceParamsGql,
    target_similarity: f64,
}

impl From<crate::object_detection::ObjectDetectionParams> for ObjectDetectionParamsGql {
    fn from(value: crate::object_detection::ObjectDetectionParams) -> Self {
        Self {
            tile_params: value.tile_params.into(),
            bucket_delta: value.bucket_delta,
            trace_params: value.trace_params.into(),
            target_similarity: value.target_similarity,
        }
    }
}

#[derive(async_graphql::SimpleObject, Clone)]
pub struct OrdinarySessionGql {
    basic_params: BasicParamsGql,
    results: ResultsGql,
}

impl From<crate::service::OrdinarySession> for OrdinarySessionGql {
    fn from(value: crate::service::OrdinarySession) -> Self {
        Self {
            basic_params: value.basic_params.into(),
            results: value.results.into(),
        }
    }
}

#[derive(async_graphql::SimpleObject, Clone)]
pub struct EyeSessionGql {
    basic_params: BasicParamsGql,
    eye_params: EyeParamsGql,
    results: ResultsGql,
}

impl From<crate::service::EyeSession> for EyeSessionGql {
    fn from(value: crate::service::EyeSession) -> Self {
        Self {
            basic_params: value.basic_params.into(),
            eye_params: value.eye_params.into(),
            results: value.results.into(),
        }
    }
}

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

#[derive(async_graphql::SimpleObject, Clone)]
pub struct ObjectSessionGql {
    basic_params: BasicParamsGql,
    object_detection_params: ObjectDetectionParamsGql,
    objects_to_be_detected: Vec<ReferenceObjectGql>,
    results: ResultsGql,
}

impl From<ObjectSession> for ObjectSessionGql {
    fn from(value: ObjectSession) -> Self {
        Self {
            basic_params: value.basic_params.into(),
            object_detection_params: value.object_detection_params.into(),
            objects_to_be_detected: value
                .objects_to_be_detected
                .into_iter()
                .map(Into::into)
                .collect(),
            results: value.results.into(),
        }
    }
}

#[derive(InputObject, Clone)]
pub struct BasicParamsInputGql {
    do_grayscale: bool,
    gradient_threshold: u8,
}

impl From<BasicParamsInputGql> for BasicParamsInput {
    fn from(value: BasicParamsInputGql) -> Self {
        Self {
            do_grayscale: value.do_grayscale,
            gradient_threshold: value.gradient_threshold,
        }
    }
}

#[derive(InputObject, Clone)]
pub struct TileParamsInputGql {
    tile_x: f64,
    tile_y: f64,
}

impl From<TileParamsInputGql> for TileParamsInput {
    fn from(value: TileParamsInputGql) -> Self {
        Self {
            tile_x: value.tile_x,
            tile_y: value.tile_y,
        }
    }
}

#[derive(InputObject, Clone)]
pub struct TraceParamsInputGql {
    num_skeleton: usize,
    close_slice_threshold: f64,
}

impl From<TraceParamsInputGql> for TraceParamsInput {
    fn from(value: TraceParamsInputGql) -> Self {
        Self {
            num_skeleton: value.num_skeleton,
            close_slice_threshold: value.close_slice_threshold,
        }
    }
}

#[derive(InputObject, Clone)]
pub struct EyeParamsInputGql {
    tile_params: TileParamsInputGql,
    bucket_delta: f64,
    trace_params: TraceParamsInputGql,
    target_similarity: f64,
}

impl From<EyeParamsInputGql> for EyeParamsInput {
    fn from(value: EyeParamsInputGql) -> Self {
        Self {
            tile_params: value.tile_params.into(),
            bucket_delta: value.bucket_delta,
            trace_params: value.trace_params.into(),
            target_similarity: value.target_similarity,
        }
    }
}

#[derive(InputObject, Clone)]
pub struct ObjectDetectionParamsInputGql {
    tile_params: TileParamsInputGql,
    bucket_delta: f64,
    trace_params: TraceParamsInputGql,
    target_similarity: f64,
}

impl From<ObjectDetectionParamsInputGql> for ObjectDetectionParamsInput {
    fn from(value: ObjectDetectionParamsInputGql) -> Self {
        Self {
            tile_params: value.tile_params.into(),
            bucket_delta: value.bucket_delta,
            trace_params: value.trace_params.into(),
            target_similarity: value.target_similarity,
        }
    }
}

#[derive(InputObject, Clone)]
pub struct Vec3InputGql {
    x: f64,
    y: f64,
    z: Option<f64>,
}

#[derive(InputObject, Clone)]
pub struct RectangleInputGql {
    top_left: Vec3InputGql,
    bottom_right: Vec3InputGql,
}

impl From<RectangleInputGql> for Rectangle {
    fn from(value: RectangleInputGql) -> Self {
        Self::new(
            Vec3d::new(value.top_left.x, value.top_left.y, value.top_left.z.unwrap_or(0.0)),
            Vec3d::new(
                value.bottom_right.x,
                value.bottom_right.y,
                value.bottom_right.z.unwrap_or(0.0),
            ),
        )
    }
}

fn app_state(ctx: &Context<'_>) -> Result<GraphqlAppState> {
    ctx.data::<GraphqlAppState>()
        .cloned()
        .map_err(|_| Error::new("GraphQL app state not available"))
}

fn read_upload_bytes(ctx: &Context<'_>, upload: Upload, require_jpeg: bool) -> Result<Vec<u8>> {
    let upload_value = upload.value(ctx)?;
    if require_jpeg
        && upload_value
            .content_type
            .as_deref()
            .is_some_and(|value| !value.eq_ignore_ascii_case("image/jpeg"))
    {
        return Err(Error::new("Uploaded file must have content type image/jpeg"));
    }

    let mut content = upload_value.content;
    let mut bytes = Vec::new();
    content
        .read_to_end(&mut bytes)
        .map_err(|error| Error::new(format!("Failed to read uploaded file: {error}")))?;
    Ok(bytes)
}

fn wrapped_rgb_image_from_jpeg(bytes: &[u8]) -> Result<WrappedRgbImage> {
    let rgb_image = image::load_from_memory_with_format(bytes, image::ImageFormat::Jpeg)
        .map_err(|error| Error::new(format!("Failed to decode JPEG upload: {error}")))?
        .to_rgb8();
    Ok(WrappedRgbImage::new(rgb_image))
}

fn enum_error(message: &str) -> Error {
    Error::new(message)
}

#[Object]
impl QueryRoot {
    async fn get_ordinary_session(
        &self,
        ctx: &Context<'_>,
        session_id: String,
    ) -> Result<Option<OrdinarySessionGql>> {
        let service = app_state(ctx)?.service;
        let service = service.lock().await;
        Ok(service.get_ordinary_session(&session_id).map(Into::into))
    }

    async fn get_eye_session(
        &self,
        ctx: &Context<'_>,
        session_id: String,
    ) -> Result<Option<EyeSessionGql>> {
        let service = app_state(ctx)?.service;
        let service = service.lock().await;
        Ok(service.get_eye_session(&session_id).map(Into::into))
    }

    async fn get_object_session(
        &self,
        ctx: &Context<'_>,
        session_id: String,
    ) -> Result<Option<ObjectSessionGql>> {
        let service = app_state(ctx)?.service;
        let service = service.lock().await;
        Ok(service.get_object_session(&session_id).map(Into::into))
    }

    async fn get_basic_params(
        &self,
        ctx: &Context<'_>,
        session_id: String,
    ) -> Result<Option<BasicParamsGql>> {
        let service = app_state(ctx)?.service;
        let service = service.lock().await;
        Ok(service.get_basic_params(&session_id).map(Into::into))
    }

    async fn get_tile_params(
        &self,
        ctx: &Context<'_>,
        session_id: String,
    ) -> Result<Option<TileParamsGql>> {
        let service = app_state(ctx)?.service;
        let service = service.lock().await;
        Ok(service.get_tile_params(&session_id).map(Into::into))
    }

    async fn get_trace_params(
        &self,
        ctx: &Context<'_>,
        session_id: String,
    ) -> Result<Option<TraceParamsGql>> {
        let service = app_state(ctx)?.service;
        let service = service.lock().await;
        Ok(service.get_trace_params(&session_id).map(Into::into))
    }

    async fn get_bucket_delta(
        &self,
        ctx: &Context<'_>,
        session_id: String,
    ) -> Result<Option<f64>> {
        let service = app_state(ctx)?.service;
        let service = service.lock().await;
        Ok(service.get_bucket_delta(&session_id))
    }

    async fn get_target_similarity(
        &self,
        ctx: &Context<'_>,
        session_id: String,
    ) -> Result<Option<f64>> {
        let service = app_state(ctx)?.service;
        let service = service.lock().await;
        Ok(service.get_target_similarity(&session_id))
    }

    async fn get_rectangles(
        &self,
        ctx: &Context<'_>,
        session_id: String,
        image: Upload,
        previous_image: Option<Upload>,
    ) -> Result<Vec<EnrichedMosaicGql>> {
        let image_bytes = read_upload_bytes(ctx, image, true)?;
        let image = wrapped_rgb_image_from_jpeg(&image_bytes)?;
        let previous_image = previous_image
            .map(|upload| {
                let bytes = read_upload_bytes(ctx, upload, true)?;
                wrapped_rgb_image_from_jpeg(&bytes)
            })
            .transpose()?;

        let service = app_state(ctx)?.service;
        let service = service.lock().await;

        match service.get_rectangles(session_id, image, previous_image) {
            GetRectanglesResult::Success(mosaics) => Ok(mosaics.into_iter().map(Into::into).collect()),
            GetRectanglesResult::SessionNotFound => {
                Err(enum_error("Session not found"))
            }
            GetRectanglesResult::PreviousImageRequiredForEyeSession => {
                Err(enum_error("Previous image required for eye session"))
            }
        }
    }
}

#[Object]
impl MutationRoot {
    async fn create_ordinary_session(
        &self,
        ctx: &Context<'_>,
        session_id: String,
        basic_params: BasicParamsInputGql,
        results: ResultsGql,
    ) -> Result<bool> {
        let service = app_state(ctx)?.service;
        let mut service = service.lock().await;
        match service.create_ordinary_session(session_id, basic_params.into(), results.into()) {
            CreateOrdinarySessionResult::Success => Ok(true),
            CreateOrdinarySessionResult::SessionAlreadyExists => {
                Err(enum_error("Session already exists"))
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
            session_id,
            basic_params.into(),
            eye_params.into(),
            results.into(),
        ) {
            CreateEyeSessionResult::Success => Ok(true),
            CreateEyeSessionResult::SessionAlreadyExists => {
                Err(enum_error("Session already exists"))
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
            session_id,
            basic_params.into(),
            object_detection_params.into(),
            results.into(),
        ) {
            CreateObjectSessionResult::Success => Ok(true),
            CreateObjectSessionResult::SessionAlreadyExists => {
                Err(enum_error("Session already exists"))
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
        match service.update_basic_params(session_id, basic_params.into()) {
            UpdateBasicParamsResult::Success => Ok(true),
            UpdateBasicParamsResult::SessionNotFound => Err(enum_error("Session not found")),
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
        match service.update_tile_params(session_id, tile_params.into()) {
            TileParamsUpdateResult::Success => Ok(true),
            TileParamsUpdateResult::SessionNotFound => Err(enum_error("Session not found")),
            TileParamsUpdateResult::SessionTypeDoesNotSupportTileParams => {
                Err(enum_error("Session type does not support tile params"))
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
        match service.update_trace_params(session_id, trace_params.into()) {
            TraceParamsUpdateResult::Success => Ok(true),
            TraceParamsUpdateResult::SessionNotFound => Err(enum_error("Session not found")),
            TraceParamsUpdateResult::SessionTypeDoesNotSupportTraceParams => {
                Err(enum_error("Session type does not support trace params"))
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
        match service.update_bucket_delta(session_id, bucket_delta) {
            BucketDeltaUpdateResult::Success => Ok(true),
            BucketDeltaUpdateResult::SessionNotFound => Err(enum_error("Session not found")),
            BucketDeltaUpdateResult::SessionTypeDoesNotSupportBucketDelta => {
                Err(enum_error("Session type does not support bucket delta"))
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
        match service.update_target_similarity(session_id, target_similarity) {
            TargetSimilarityUpdateResult::Success => Ok(true),
            TargetSimilarityUpdateResult::SessionNotFound => Err(enum_error("Session not found")),
            TargetSimilarityUpdateResult::SessionTypeDoesNotSupportTargetSimilarity => {
                Err(enum_error("Session type does not support target similarity"))
            }
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
        match service.update_eye_params(session_id, eye_params.into()) {
            EyeParamsUpdateResult::Success => Ok(true),
            EyeParamsUpdateResult::SessionNotFound => Err(enum_error("Session not found")),
            EyeParamsUpdateResult::SessionTypeDoesNotSupportEyeParams => {
                Err(enum_error("Session type does not support eye params"))
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
        match service.update_object_detection_params(session_id, object_detection_params.into()) {
            ObjectDetectionParamsUpdateResult::Success => Ok(true),
            ObjectDetectionParamsUpdateResult::SessionNotFound => {
                Err(enum_error("Session not found"))
            }
            ObjectDetectionParamsUpdateResult::SessionTypeDoesNotSupportObjectDetectionParams => {
                Err(enum_error(
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
        let image_bytes = read_upload_bytes(ctx, image, true)?;
        let wrapped_image = wrapped_rgb_image_from_jpeg(&image_bytes)?;

        let service = app_state(ctx)?.service;
        let mut service = service.lock().await;
        match service.add_object_to_be_detected_as_image(
            session_id,
            object_id,
            wrapped_image,
            surrounding_rectangle.into(),
        ) {
            AddObjectToBeDetectedResult::Success => Ok(true),
            AddObjectToBeDetectedResult::SessionNotFound => Err(enum_error("Session not found")),
            AddObjectToBeDetectedResult::SessionTypeDoesNotSupportAddingObjectToBeDetected => {
                Err(enum_error(
                    "Session type does not support adding object to be detected",
                ))
            }
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
        match service.add_object_to_be_detected_as_ascii_art(session_id, object_id, ascii_art) {
            AddObjectToBeDetectedResult::Success => Ok(true),
            AddObjectToBeDetectedResult::SessionNotFound => Err(enum_error("Session not found")),
            AddObjectToBeDetectedResult::SessionTypeDoesNotSupportAddingObjectToBeDetected => {
                Err(enum_error(
                    "Session type does not support adding object to be detected",
                ))
            }
        }
    }

    async fn delete_session(&self, ctx: &Context<'_>, session_id: String) -> Result<bool> {
        let service = app_state(ctx)?.service;
        let mut service = service.lock().await;
        match service.delete_session(&session_id) {
            DeleteSessionResult::Success => Ok(true),
            DeleteSessionResult::SessionNotFound => Err(enum_error("Session not found")),
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
        match service.delete_reference_object(&session_id, object_id) {
            DeleteReferenceObjectResult::Success => Ok(true),
            DeleteReferenceObjectResult::SessionNotFound => Err(enum_error("Session not found")),
            DeleteReferenceObjectResult::SessionTypeDoesNotSupportDeletingReferenceObject => {
                Err(enum_error(
                    "Session type does not support deleting reference object",
                ))
            }
            DeleteReferenceObjectResult::ReferenceObjectNotFound => {
                Err(enum_error("Reference object not found"))
            }
        }
    }
}

#![warn(clippy::pedantic)]
#![allow(
    clippy::missing_errors_doc,
    clippy::missing_panics_doc,
    clippy::too_many_lines,
    clippy::unused_async_trait_impl
)]

use crate::eye::EyeParams;
use crate::eye::TileParams;
use crate::mosaics::Results;
use crate::mosaics::WrappedRelativeMosaic;
use crate::object_detection::ObjectDetectionParams;
use crate::object_detection::ReferenceObject;
use crate::slices::BasicParams;
use crate::slices::Rectangle;
use crate::slices::WrappedRgbImage;
use crate::traced_mosaics::TracedRelativeMosaic;
use crate::traces::TraceParams;
use rs_math3d::Vec3d;
use std::collections::BTreeMap;

use super::{
    AddColorFilterResult, AddObjectToBeDetectedResult, BasicParamsInput, BucketDeltaUpdateResult,
    ColorFilter, CreateEyeSessionResult, CreateObjectSessionResult, CreateOrdinarySessionResult,
    CreateThreeDTriangleSessionResult, DeleteReferenceObjectResult, DeleteSessionResult,
    EyeParamsInput, EyeParamsUpdateResult, EyeSession, GetRectanglesRequest, GetRectanglesResult,
    GetTrianglesRequest, ObjectDetectionParamsInput, ObjectDetectionParamsUpdateResult,
    ObjectSession, OrdinarySession, RgbColor, Session, TargetSimilarityUpdateResult,
    ThreeDTriangleCalibration, TileParamsInput, TileParamsUpdateResult, TraceParamsInput,
    TraceParamsUpdateResult, UpdateBasicParamsResult, calculate_ordinary_mosaics,
};

struct ThreeDTriangleSession {
    calibration: Option<ThreeDTriangleCalibration>,
    calculation: std::sync::Arc<super::get_triangles_request::TriangleCalculation>,
}

pub struct Service {
    sessions: BTreeMap<String, Session>,
    three_d_triangle_sessions: BTreeMap<String, ThreeDTriangleSession>,
}

impl Default for Service {
    fn default() -> Self {
        Self::new()
    }
}

impl Service {
    #[must_use]
    pub fn new() -> Self {
        Service {
            sessions: BTreeMap::new(),
            three_d_triangle_sessions: BTreeMap::new(),
        }
    }

    // mutations

    pub fn create_three_d_triangle_session(
        &mut self,
        session_id: &str,
        calibration: Option<ThreeDTriangleCalibration>,
    ) -> CreateThreeDTriangleSessionResult {
        if self.sessions.contains_key(session_id)
            || self.three_d_triangle_sessions.contains_key(session_id)
        {
            return CreateThreeDTriangleSessionResult::SessionAlreadyExists;
        }
        if let Some(calibration) = &calibration
            && let Err(error) = super::three_d_triangle::validate_calibration(calibration)
        {
            return CreateThreeDTriangleSessionResult::InvalidInput(error);
        }
        self.three_d_triangle_sessions.insert(
            session_id.to_owned(),
            ThreeDTriangleSession {
                calibration,
                calculation: std::sync::Arc::new(
                    super::get_triangles_request::TriangleCalculation::new(),
                ),
            },
        );
        CreateThreeDTriangleSessionResult::Success
    }

    #[must_use]
    pub fn get_triangles_request(
        &self,
        session_id: &str,
        calibration_override: Option<ThreeDTriangleCalibration>,
    ) -> Option<GetTrianglesRequest> {
        let session = self.three_d_triangle_sessions.get(session_id)?;
        let calibration = calibration_override.or_else(|| session.calibration.clone())?;
        Some(GetTrianglesRequest::new(
            std::sync::Arc::clone(&session.calculation),
            calibration,
        ))
    }

    pub fn create_ordinary_session(
        &mut self,
        session_id: &str,
        basic_params_input: BasicParamsInput,
        results: Results,
    ) -> CreateOrdinarySessionResult {
        let basic_params = BasicParams::new(
            basic_params_input.do_grayscale,
            basic_params_input.gradient_threshold,
        );
        if self.session_id_exists(session_id) {
            return CreateOrdinarySessionResult::SessionAlreadyExists;
        }
        self.sessions.insert(
            session_id.to_owned(),
            Session::Ordinary(OrdinarySession {
                basic_params,
                results,
                color_filters: Vec::new(),
                calculation: std::sync::Arc::new(
                    super::get_rectangles_request::SessionCalculation::new(),
                ),
            }),
        );
        CreateOrdinarySessionResult::Success
    }

    /// Adds an inclusive per-channel RGB tolerance filter to an ordinary session.
    pub fn add_color_filter(
        &mut self,
        session_id: &str,
        color: RgbColor,
        threshold_for_color: RgbColor,
    ) -> AddColorFilterResult {
        match self.sessions.get_mut(session_id) {
            Some(Session::Ordinary(session)) => {
                session.color_filters.push(ColorFilter {
                    color,
                    threshold_for_color,
                });
                AddColorFilterResult::Success
            }
            Some(_) => AddColorFilterResult::SessionTypeDoesNotSupportColorFilters,
            None if self.three_d_triangle_sessions.contains_key(session_id) => {
                AddColorFilterResult::SessionTypeDoesNotSupportColorFilters
            }
            None => AddColorFilterResult::SessionNotFound,
        }
    }

    pub fn create_eye_session(
        &mut self,
        session_id: &str,
        basic_params_input: BasicParamsInput,
        eye_params_input: EyeParamsInput,
        results: Results,
    ) -> CreateEyeSessionResult {
        let basic_params = BasicParams::new(
            basic_params_input.do_grayscale,
            basic_params_input.gradient_threshold,
        );
        let tile_params = TileParams::new(
            eye_params_input.tile_params.tile_x,
            eye_params_input.tile_params.tile_y,
        );
        let trace_params = TraceParams::new(eye_params_input.trace_params.num_skeleton);
        let eye_params = EyeParams::new(
            tile_params,
            eye_params_input.bucket_delta,
            trace_params,
            eye_params_input.target_similarity,
        );
        if self.session_id_exists(session_id) {
            return CreateEyeSessionResult::SessionAlreadyExists;
        }
        self.sessions.insert(
            session_id.to_owned(),
            Session::Eye(EyeSession {
                basic_params,
                eye_params,
                cached_previous_mosaics: None,
                results,
                cache_state: std::sync::Arc::new(std::sync::Mutex::new((0, None))),
                cache_generation: 0,
                calculation: std::sync::Arc::new(
                    super::get_rectangles_request::SessionCalculation::new(),
                ),
            }),
        );
        CreateEyeSessionResult::Success
    }

    pub fn create_object_session(
        &mut self,
        session_id: &str,
        basic_params_input: BasicParamsInput,
        object_detection_params_input: ObjectDetectionParamsInput,
        results: Results,
    ) -> CreateObjectSessionResult {
        if self.session_id_exists(session_id) {
            return CreateObjectSessionResult::SessionAlreadyExists;
        }
        let basic_params = BasicParams::new(
            basic_params_input.do_grayscale,
            basic_params_input.gradient_threshold,
        );
        let tile_params = TileParams::new(
            object_detection_params_input.tile_params.tile_x,
            object_detection_params_input.tile_params.tile_y,
        );
        let trace_params =
            TraceParams::new(object_detection_params_input.trace_params.num_skeleton);
        let object_detection_params = ObjectDetectionParams::new(
            tile_params,
            object_detection_params_input.bucket_delta,
            trace_params,
            object_detection_params_input.target_similarity,
        );
        self.sessions.insert(
            session_id.to_owned(),
            Session::Object(ObjectSession {
                basic_params,
                object_detection_params,
                objects_to_be_detected: vec![],
                results,
                calculation: std::sync::Arc::new(
                    super::get_rectangles_request::SessionCalculation::new(),
                ),
            }),
        );
        CreateObjectSessionResult::Success
    }

    pub fn update_basic_params(
        &mut self,
        session_id: &str,
        basic_params_input: BasicParamsInput,
    ) -> UpdateBasicParamsResult {
        if let Some(session) = self.sessions.get_mut(session_id) {
            match session {
                Session::Ordinary(ordinary_session) => {
                    ordinary_session.basic_params = BasicParams::new(
                        basic_params_input.do_grayscale,
                        basic_params_input.gradient_threshold,
                    );
                }
                Session::Eye(eye_session) => {
                    eye_session.basic_params = BasicParams::new(
                        basic_params_input.do_grayscale,
                        basic_params_input.gradient_threshold,
                    );
                    reset_eye_cache(eye_session);
                }
                Session::Object(object_session) => {
                    object_session.basic_params = BasicParams::new(
                        basic_params_input.do_grayscale,
                        basic_params_input.gradient_threshold,
                    );
                }
            }
            UpdateBasicParamsResult::Success
        } else {
            UpdateBasicParamsResult::SessionNotFound
        }
    }

    pub fn update_tile_params(
        &mut self,
        session_id: &str,
        tile_params_input: TileParamsInput,
    ) -> TileParamsUpdateResult {
        if let Some(session) = self.sessions.get_mut(session_id) {
            match session {
                Session::Eye(eye_session) => {
                    eye_session.eye_params.tile_params =
                        TileParams::new(tile_params_input.tile_x, tile_params_input.tile_y);
                    reset_eye_cache(eye_session);
                }
                Session::Object(object_session) => {
                    object_session.object_detection_params.tile_params =
                        TileParams::new(tile_params_input.tile_x, tile_params_input.tile_y);
                }
                Session::Ordinary(_) => {
                    return TileParamsUpdateResult::SessionTypeDoesNotSupportTileParams;
                }
            }
            TileParamsUpdateResult::Success
        } else {
            TileParamsUpdateResult::SessionNotFound
        }
    }

    pub fn update_trace_params(
        &mut self,
        session_id: &str,
        trace_params_input: TraceParamsInput,
    ) -> TraceParamsUpdateResult {
        if let Some(session) = self.sessions.get_mut(session_id) {
            match session {
                Session::Eye(eye_session) => {
                    eye_session.eye_params.trace_params =
                        TraceParams::new(trace_params_input.num_skeleton);
                    reset_eye_cache(eye_session);
                }
                Session::Object(object_session) => {
                    object_session.object_detection_params.trace_params =
                        TraceParams::new(trace_params_input.num_skeleton);
                }
                Session::Ordinary(_) => {
                    return TraceParamsUpdateResult::SessionTypeDoesNotSupportTraceParams;
                }
            }
            TraceParamsUpdateResult::Success
        } else {
            TraceParamsUpdateResult::SessionNotFound
        }
    }

    pub fn update_bucket_delta(
        &mut self,
        session_id: &str,
        bucket_delta: f64,
    ) -> BucketDeltaUpdateResult {
        if let Some(session) = self.sessions.get_mut(session_id) {
            match session {
                Session::Eye(eye_session) => {
                    eye_session.eye_params.bucket_delta = bucket_delta;
                    reset_eye_cache(eye_session);
                }
                Session::Object(object_session) => {
                    object_session.object_detection_params.bucket_delta = bucket_delta;
                }
                Session::Ordinary(_) => {
                    return BucketDeltaUpdateResult::SessionTypeDoesNotSupportBucketDelta;
                }
            }
            BucketDeltaUpdateResult::Success
        } else {
            BucketDeltaUpdateResult::SessionNotFound
        }
    }

    pub fn update_target_similarity(
        &mut self,
        session_id: &str,
        target_similarity: f64,
    ) -> TargetSimilarityUpdateResult {
        if let Some(session) = self.sessions.get_mut(session_id) {
            match session {
                Session::Eye(eye_session) => {
                    eye_session.eye_params.target_similarity = target_similarity;
                    reset_eye_cache(eye_session);
                }
                Session::Object(object_session) => {
                    object_session.object_detection_params.target_similarity = target_similarity;
                }
                Session::Ordinary(_) => {
                    return TargetSimilarityUpdateResult::SessionTypeDoesNotSupportTargetSimilarity;
                }
            }
            TargetSimilarityUpdateResult::Success
        } else {
            TargetSimilarityUpdateResult::SessionNotFound
        }
    }

    pub fn update_eye_params(
        &mut self,
        session_id: &str,
        eye_params_input: EyeParamsInput,
    ) -> EyeParamsUpdateResult {
        if let Some(session) = self.sessions.get_mut(session_id) {
            match session {
                Session::Eye(eye_session) => {
                    let tile_params = TileParams::new(
                        eye_params_input.tile_params.tile_x,
                        eye_params_input.tile_params.tile_y,
                    );
                    let trace_params = TraceParams::new(eye_params_input.trace_params.num_skeleton);
                    eye_session.eye_params = EyeParams::new(
                        tile_params,
                        eye_params_input.bucket_delta,
                        trace_params,
                        eye_params_input.target_similarity,
                    );
                    reset_eye_cache(eye_session);
                }
                _ => return EyeParamsUpdateResult::SessionTypeDoesNotSupportEyeParams,
            }
            EyeParamsUpdateResult::Success
        } else {
            EyeParamsUpdateResult::SessionNotFound
        }
    }

    pub fn update_object_detection_params(
        &mut self,
        session_id: &str,
        object_detection_params_input: ObjectDetectionParamsInput,
    ) -> ObjectDetectionParamsUpdateResult {
        if let Some(session) = self.sessions.get_mut(session_id) {
            match session {
                Session::Object(object_session) => {
                    let tile_params = TileParams::new(
                        object_detection_params_input.tile_params.tile_x,
                        object_detection_params_input.tile_params.tile_y,
                    );
                    let trace_params =
                        TraceParams::new(object_detection_params_input.trace_params.num_skeleton);
                    object_session.object_detection_params = ObjectDetectionParams::new(
                        tile_params,
                        object_detection_params_input.bucket_delta,
                        trace_params,
                        object_detection_params_input.target_similarity,
                    );
                }
                _ => return ObjectDetectionParamsUpdateResult::SessionTypeDoesNotSupportObjectDetectionParams,
            }
            ObjectDetectionParamsUpdateResult::Success
        } else {
            ObjectDetectionParamsUpdateResult::SessionNotFound
        }
    }

    pub fn add_object_to_be_detected_as_image(
        &mut self,
        session_id: &str,
        object_id: &str,
        image: &WrappedRgbImage,
        surrounding_rectangle: Rectangle,
    ) -> AddObjectToBeDetectedResult {
        if let Some(session) = self.sessions.get_mut(session_id) {
            match session {
                Session::Object(object_session) => {
                    let math_rectangle = crate::math::Rectangle::new(
                        surrounding_rectangle.get_top_left(),
                        surrounding_rectangle.get_bottom_right(),
                    );
                    let mosaics = calculate_ordinary_mosaics(
                        object_session.basic_params,
                        Results::Absolute,
                        image,
                    );
                    let reference_mosaics = mosaics
                        .into_iter()
                        .filter(|mosaic| {
                            let coordinated_bounding_box = mosaic.get_bounding_box();
                            let global_bounding_box = coordinated_bounding_box.to_global_rectangle();
                            let bounding_box =
                                Rectangle::new_from_math_rectangle(&global_bounding_box);
                            bounding_box.overlaps(&surrounding_rectangle)
                        })
                        .map(|mosaic| TracedRelativeMosaic::new(
                            WrappedRelativeMosaic::new(mosaic.get_mosaic(), math_rectangle.clone()),
                            object_session.object_detection_params.trace_params,
                        ))
                        .collect::<Vec<_>>();
                    if reference_mosaics.is_empty() {
                        return AddObjectToBeDetectedResult::NoReferenceMosaicsDetected;
                    }
                    object_session
                        .objects_to_be_detected
                        .push(ReferenceObject::new(object_id.to_owned(), reference_mosaics));
                }
                _ => return AddObjectToBeDetectedResult::SessionTypeDoesNotSupportAddingObjectToBeDetected,
            }
            AddObjectToBeDetectedResult::Success
        } else {
            AddObjectToBeDetectedResult::SessionNotFound
        }
    }

    pub fn add_object_to_be_detected_as_ascii_art(
        &mut self,
        session_id: &str,
        object_id: &str,
        ascii_art: &str,
    ) -> AddObjectToBeDetectedResult {
        if let Some(session) = self.sessions.get_mut(session_id) {
            match session {
                Session::Object(_object_session) => {
                    let image = WrappedRgbImage::new_from_ascii_art(ascii_art);
                    let surrounding_rectangle = Rectangle::new(
                        Vec3d::new(0.0, 0.0, 0.0),
                        Vec3d::new(f64::from(image.width()), f64::from(image.height()), 0.0),
                    );
                    self.add_object_to_be_detected_as_image(
                        session_id,
                        object_id,
                        &image,
                        surrounding_rectangle,
                    )
                }
                _ => AddObjectToBeDetectedResult::SessionTypeDoesNotSupportAddingObjectToBeDetected,
            }
        } else {
            AddObjectToBeDetectedResult::SessionNotFound
        }
    }

    pub fn delete_session(&mut self, session_id: &str) -> DeleteSessionResult {
        let removed_regular = self.sessions.remove(session_id).is_some();
        let removed_triangle = self.three_d_triangle_sessions.remove(session_id).is_some();
        if removed_regular || removed_triangle {
            DeleteSessionResult::Success
        } else {
            DeleteSessionResult::SessionNotFound
        }
    }

    pub fn delete_reference_object(
        &mut self,
        session_id: &str,
        object_id: &str,
    ) -> DeleteReferenceObjectResult {
        if let Some(session) = self.sessions.get_mut(session_id) {
            match session {
                Session::Object(object_session) => {
                    let initial_len = object_session.objects_to_be_detected.len();
                    object_session
                        .objects_to_be_detected
                        .retain(|object| object.get_id() != object_id);
                    if object_session.objects_to_be_detected.len() == initial_len {
                        return DeleteReferenceObjectResult::ReferenceObjectNotFound;
                    }
                }
                _ => return DeleteReferenceObjectResult::SessionTypeDoesNotSupportDeletingReferenceObject,
            }
            DeleteReferenceObjectResult::Success
        } else {
            DeleteReferenceObjectResult::SessionNotFound
        }
    }

    // queries
    #[must_use]
    pub fn get_ordinary_session(&self, session_id: &str) -> Option<OrdinarySession> {
        if let Some(Session::Ordinary(ordinary_session)) = self.sessions.get(session_id) {
            Some(ordinary_session.clone())
        } else {
            None
        }
    }

    #[must_use]
    pub fn get_eye_session(&self, session_id: &str) -> Option<EyeSession> {
        if let Some(Session::Eye(eye_session)) = self.sessions.get(session_id) {
            let mut eye_session = eye_session.clone();
            refresh_eye_cache(&mut eye_session);
            Some(eye_session)
        } else {
            None
        }
    }

    #[must_use]
    pub fn get_object_session(&self, session_id: &str) -> Option<ObjectSession> {
        if let Some(Session::Object(object_session)) = self.sessions.get(session_id) {
            Some(object_session.clone())
        } else {
            None
        }
    }

    #[must_use]
    pub fn get_basic_params(&self, session_id: &str) -> Option<BasicParams> {
        match self.sessions.get(session_id) {
            Some(Session::Ordinary(ordinary_session)) => Some(ordinary_session.basic_params),
            Some(Session::Eye(eye_session)) => Some(eye_session.basic_params),
            Some(Session::Object(object_session)) => Some(object_session.basic_params),
            None => None,
        }
    }

    #[must_use]
    pub fn get_tile_params(&self, session_id: &str) -> Option<TileParams> {
        match self.sessions.get(session_id) {
            Some(Session::Eye(eye_session)) => Some(eye_session.eye_params.tile_params),
            Some(Session::Object(object_session)) => {
                Some(object_session.object_detection_params.tile_params)
            }
            _ => None,
        }
    }

    #[must_use]
    pub fn get_trace_params(&self, session_id: &str) -> Option<TraceParams> {
        match self.sessions.get(session_id) {
            Some(Session::Eye(eye_session)) => Some(eye_session.eye_params.trace_params),
            Some(Session::Object(object_session)) => {
                Some(object_session.object_detection_params.trace_params)
            }
            _ => None,
        }
    }

    #[must_use]
    pub fn get_bucket_delta(&self, session_id: &str) -> Option<f64> {
        match self.sessions.get(session_id) {
            Some(Session::Eye(eye_session)) => Some(eye_session.eye_params.bucket_delta),
            Some(Session::Object(object_session)) => {
                Some(object_session.object_detection_params.bucket_delta)
            }
            _ => None,
        }
    }

    #[must_use]
    pub fn get_target_similarity(&self, session_id: &str) -> Option<f64> {
        match self.sessions.get(session_id) {
            Some(Session::Eye(eye_session)) => Some(eye_session.eye_params.target_similarity),
            Some(Session::Object(object_session)) => {
                Some(object_session.object_detection_params.target_similarity)
            }
            _ => None,
        }
    }

    #[must_use]
    pub fn get_session(&self, session_id: &str) -> Option<Session> {
        let mut session = self.sessions.get(session_id)?.clone();
        if let Session::Eye(eye_session) = &mut session {
            refresh_eye_cache(eye_session);
        }
        Some(session)
    }

    fn session_id_exists(&self, session_id: &str) -> bool {
        self.sessions.contains_key(session_id)
            || self.three_d_triangle_sessions.contains_key(session_id)
    }

    #[must_use]
    pub fn get_rectangles_request(&self, session_id: &str) -> Option<GetRectanglesRequest> {
        let session = self.get_session(session_id)?;
        Some(GetRectanglesRequest::new(session))
    }

    pub async fn get_rectangles(
        &self,
        session_id: &str,
        image: WrappedRgbImage,
    ) -> GetRectanglesResult {
        let Some(request) = self.get_rectangles_request(session_id) else {
            return GetRectanglesResult::SessionNotFound;
        };
        request.get_rectangles_with(move || Ok(image)).await
    }
}

fn refresh_eye_cache(eye_session: &mut super::EyeSession) {
    let cache = eye_session.cache_state.lock().unwrap();
    if cache.0 == eye_session.cache_generation {
        eye_session.cached_previous_mosaics = cache.1.clone();
    } else {
        eye_session.cached_previous_mosaics = None;
    }
}

fn reset_eye_cache(eye_session: &mut super::EyeSession) {
    let mut cache = eye_session.cache_state.lock().unwrap();
    cache.0 = cache.0.wrapping_add(1);
    cache.1 = None;
    eye_session.cached_previous_mosaics = None;
    eye_session.cache_generation = cache.0;
}

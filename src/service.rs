use crate::eye::EyeParams;
use crate::eye::TileParams;
use crate::eye::deduce_bucketed_mosaics;
use crate::eye::deduce_rectangles;
use crate::math::WrappedCoordinateSystem;
use crate::mosaics::WrappedMosaic;
use crate::mosaics::WrappedRelativeMosaic;
use crate::mosaics::deduce_mosaics;
use crate::object_detection::ObjectDetectionParams;
use crate::object_detection::ReferenceObject;
use crate::object_detection::detect_objects;
use crate::slices;
use crate::slices::BasicParams;
use crate::slices::Color;
use crate::slices::Rectangle;
use crate::slices::WrappedRgbImage;
use crate::slices::calculate_slices;
use crate::slices::find_connected_slices;
use crate::traces::TraceParams;

use rs_math3d::Vec3d;
use std::collections::BTreeMap;

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
/// Controls whether enriched outputs use source mosaic coordinates (`Absolute`)
/// or normalized coordinates derived from `WrappedRelativeMosaic` (`Relative`).
pub enum Results {
    Absolute,
    Relative,
}

#[derive(Clone)]
pub struct BasicParamsInput {
    pub do_grayscale: bool,
    pub gradient_threshold: u8,
}

#[derive(Clone)]
pub struct TileParamsInput {
    pub tile_x: f64,
    pub tile_y: f64,
}

#[derive(Clone)]
pub struct TraceParamsInput {
    pub num_skeleton: usize,
    pub close_slice_threshold: f64,
}

#[derive(Clone)]
pub struct EyeParamsInput {
    pub tile_params: TileParamsInput,
    pub bucket_delta: f64,
    pub trace_params: TraceParamsInput,
    pub target_similarity: f64,
}

#[derive(Clone)]
pub struct ObjectDetectionParamsInput {
    pub tile_params: TileParamsInput,
    pub bucket_delta: f64,
    pub trace_params: TraceParamsInput,
    pub target_similarity: f64,
}

#[derive(Clone)]
pub struct OrdinarySession {
    pub basic_params: BasicParams,
    pub results: Results,
}

#[derive(Clone)]
pub struct EyeSession {
    pub basic_params: BasicParams,
    pub eye_params: EyeParams,
    pub results: Results,
}

#[derive(Clone)]
pub struct ObjectSession {
    pub basic_params: BasicParams,
    pub object_detection_params: ObjectDetectionParams,
    pub objects_to_be_detected: Vec<ReferenceObject>,
    pub results: Results,
}

#[derive(Clone)]
pub struct Point {
    pub x: f64,
    pub y: f64,
}

#[derive(Clone)]
pub struct Slice {
    pub start: Point,
    pub end: Point,
}

#[derive(Clone)]
pub struct SliceLine {
    pub slices: Vec<Slice>,
    pub line_number: usize,
}

#[derive(Clone)]
pub struct RgbColor {
    pub red: u8,
    pub green: u8,
    pub blue: u8,
}

#[derive(Clone)]
/// Bounding circle returned with each `EnrichedMosaic`.
/// The center is exported in the global coordinate system representation.
/// In `Results::Relative`, the values remain normalized (0..1) before export.
pub struct Circle {
    pub center: Vec3d,
    pub radius: f64,
}

#[derive(Clone)]
pub struct EnrichedMosaic {
    pub bounding_box: Rectangle,
    pub bounding_circle: Circle,
    pub color: Color,
    pub area: f64,
    pub center_of_mass: Vec3d,
    pub slice_matrix: Vec<SliceLine>,
    pub average_color: RgbColor,
    pub results: Results,
}

#[derive(Clone)]
pub enum Session {
    Ordinary(OrdinarySession),
    Eye(EyeSession),
    Object(ObjectSession),
}

pub enum CreateOrdinarySessionResult {
    Success,
    SessionAlreadyExists,
}

pub enum CreateEyeSessionResult {
    Success,
    SessionAlreadyExists,
}

pub enum CreateObjectSessionResult {
    Success,
    SessionAlreadyExists,
}

pub enum UpdateBasicParamsResult {
    Success,
    SessionNotFound,
}

pub enum TileParamsUpdateResult {
    Success,
    SessionNotFound,
    SessionTypeDoesNotSupportTileParams,
}

pub enum TraceParamsUpdateResult {
    Success,
    SessionNotFound,
    SessionTypeDoesNotSupportTraceParams,
}

pub enum BucketDeltaUpdateResult {
    Success,
    SessionNotFound,
    SessionTypeDoesNotSupportBucketDelta,
}

pub enum TargetSimilarityUpdateResult {
    Success,
    SessionNotFound,
    SessionTypeDoesNotSupportTargetSimilarity,
}

pub enum EyeParamsUpdateResult {
    Success,
    SessionNotFound,
    SessionTypeDoesNotSupportEyeParams,
}

pub enum ObjectDetectionParamsUpdateResult {
    Success,
    SessionNotFound,
    SessionTypeDoesNotSupportObjectDetectionParams,
}

pub enum AddObjectToBeDetectedResult {
    Success,
    SessionNotFound,
    SessionTypeDoesNotSupportAddingObjectToBeDetected,
}

pub enum DeleteSessionResult {
    Success,
    SessionNotFound,
}

pub enum DeleteReferenceObjectResult {
    Success,
    SessionNotFound,
    SessionTypeDoesNotSupportDeletingReferenceObject,
    ReferenceObjectNotFound,
}

pub enum GetRectanglesResult {
    Success(Vec<EnrichedMosaic>),
    SessionNotFound,
    PreviousImageRequiredForEyeSession,
}

pub struct Service {
    sessions: BTreeMap<String, Session>,
}

impl Default for Service {
    fn default() -> Self {
        Self::new()
    }
}

impl Service {
    pub fn new() -> Self {
        Service {
            sessions: BTreeMap::new(),
        }
    }

    // mutations

    pub fn create_ordinary_session(
        &mut self,
        session_id: String,
        basic_params_input: BasicParamsInput,
        results: Results,
    ) -> CreateOrdinarySessionResult {
        let basic_params = BasicParams::new(
            basic_params_input.do_grayscale,
            basic_params_input.gradient_threshold,
        );
        if self.sessions.contains_key(&session_id) {
            return CreateOrdinarySessionResult::SessionAlreadyExists;
        }
        self.sessions.insert(
            session_id,
            Session::Ordinary(OrdinarySession {
                basic_params,
                results,
            }),
        );
        CreateOrdinarySessionResult::Success
    }

    pub fn create_eye_session(
        &mut self,
        session_id: String,
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
        let trace_params = TraceParams::new(
            eye_params_input.trace_params.num_skeleton,
            eye_params_input.trace_params.close_slice_threshold,
        );
        let eye_params = EyeParams::new(
            tile_params,
            eye_params_input.bucket_delta,
            trace_params,
            eye_params_input.target_similarity,
        );
        if self.sessions.contains_key(&session_id) {
            return CreateEyeSessionResult::SessionAlreadyExists;
        }
        self.sessions.insert(
            session_id,
            Session::Eye(EyeSession {
                basic_params,
                eye_params,
                results,
            }),
        );
        CreateEyeSessionResult::Success
    }

    pub fn create_object_session(
        &mut self,
        session_id: String,
        basic_params_input: BasicParamsInput,
        object_detection_params_input: ObjectDetectionParamsInput,
        results: Results,
    ) -> CreateObjectSessionResult {
        if self.sessions.contains_key(&session_id) {
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
        let trace_params = TraceParams::new(
            object_detection_params_input.trace_params.num_skeleton,
            object_detection_params_input
                .trace_params
                .close_slice_threshold,
        );
        let object_detection_params = ObjectDetectionParams::new(
            tile_params,
            object_detection_params_input.bucket_delta,
            trace_params,
            object_detection_params_input.target_similarity,
        );
        self.sessions.insert(
            session_id,
            Session::Object(ObjectSession {
                basic_params,
                object_detection_params,
                objects_to_be_detected: vec![],
                results,
            }),
        );
        CreateObjectSessionResult::Success
    }

    pub fn update_basic_params(
        &mut self,
        session_id: String,
        basic_params_input: BasicParamsInput,
    ) -> UpdateBasicParamsResult {
        if let Some(session) = self.sessions.get_mut(&session_id) {
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
        session_id: String,
        tile_params_input: TileParamsInput,
    ) -> TileParamsUpdateResult {
        if let Some(session) = self.sessions.get_mut(&session_id) {
            match session {
                Session::Eye(eye_session) => {
                    eye_session.eye_params.tile_params =
                        TileParams::new(tile_params_input.tile_x, tile_params_input.tile_y);
                }
                Session::Object(object_session) => {
                    object_session.object_detection_params.tile_params =
                        TileParams::new(tile_params_input.tile_x, tile_params_input.tile_y);
                }
                _ => return TileParamsUpdateResult::SessionTypeDoesNotSupportTileParams,
            }
            TileParamsUpdateResult::Success
        } else {
            TileParamsUpdateResult::SessionNotFound
        }
    }

    pub fn update_trace_params(
        &mut self,
        session_id: String,
        trace_params_input: TraceParamsInput,
    ) -> TraceParamsUpdateResult {
        if let Some(session) = self.sessions.get_mut(&session_id) {
            match session {
                Session::Eye(eye_session) => {
                    eye_session.eye_params.trace_params = TraceParams::new(
                        trace_params_input.num_skeleton,
                        trace_params_input.close_slice_threshold,
                    );
                }
                Session::Object(object_session) => {
                    object_session.object_detection_params.trace_params = TraceParams::new(
                        trace_params_input.num_skeleton,
                        trace_params_input.close_slice_threshold,
                    );
                }
                _ => return TraceParamsUpdateResult::SessionTypeDoesNotSupportTraceParams,
            }
            TraceParamsUpdateResult::Success
        } else {
            TraceParamsUpdateResult::SessionNotFound
        }
    }

    pub fn update_bucket_delta(
        &mut self,
        session_id: String,
        bucket_delta: f64,
    ) -> BucketDeltaUpdateResult {
        if let Some(session) = self.sessions.get_mut(&session_id) {
            match session {
                Session::Eye(eye_session) => {
                    eye_session.eye_params.bucket_delta = bucket_delta;
                }
                Session::Object(object_session) => {
                    object_session.object_detection_params.bucket_delta = bucket_delta;
                }
                _ => return BucketDeltaUpdateResult::SessionTypeDoesNotSupportBucketDelta,
            }
            BucketDeltaUpdateResult::Success
        } else {
            BucketDeltaUpdateResult::SessionNotFound
        }
    }

    pub fn update_target_similarity(
        &mut self,
        session_id: String,
        target_similarity: f64,
    ) -> TargetSimilarityUpdateResult {
        if let Some(session) = self.sessions.get_mut(&session_id) {
            match session {
                Session::Eye(eye_session) => {
                    eye_session.eye_params.target_similarity = target_similarity;
                }
                Session::Object(object_session) => {
                    object_session.object_detection_params.target_similarity = target_similarity;
                }
                _ => {
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
        session_id: String,
        eye_params_input: EyeParamsInput,
    ) -> EyeParamsUpdateResult {
        if let Some(session) = self.sessions.get_mut(&session_id) {
            match session {
                Session::Eye(eye_session) => {
                    let tile_params = TileParams::new(
                        eye_params_input.tile_params.tile_x,
                        eye_params_input.tile_params.tile_y,
                    );
                    let trace_params = TraceParams::new(
                        eye_params_input.trace_params.num_skeleton,
                        eye_params_input.trace_params.close_slice_threshold,
                    );
                    eye_session.eye_params = EyeParams::new(
                        tile_params,
                        eye_params_input.bucket_delta,
                        trace_params,
                        eye_params_input.target_similarity,
                    );
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
        session_id: String,
        object_detection_params_input: ObjectDetectionParamsInput,
    ) -> ObjectDetectionParamsUpdateResult {
        if let Some(session) = self.sessions.get_mut(&session_id) {
            match session {
                Session::Object(object_session) => {
                    let tile_params = TileParams::new(
                        object_detection_params_input.tile_params.tile_x,
                        object_detection_params_input.tile_params.tile_y,
                    );
                    let trace_params = TraceParams::new(
                        object_detection_params_input.trace_params.num_skeleton,
                        object_detection_params_input
                            .trace_params
                            .close_slice_threshold,
                    );
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
        session_id: String,
        object_id: String,
        image: WrappedRgbImage,
        surrounding_rectangle: Rectangle,
    ) -> AddObjectToBeDetectedResult {
        if let Some(session) = self.sessions.get_mut(&session_id) {
            match session {
                Session::Object(object_session) => {
                    let mosaics =
                        calculate_ordinary_mosaics(object_session.basic_params.clone(), image);
                    let reference_mosaics = mosaics
                        .into_iter()
                        .filter(|mosaic| {
                            let coordinated_bounding_box = mosaic.get_bounding_box();
                            let global_bounding_box = coordinated_bounding_box.to_global_rectangle();
                            let bounding_box =
                                Rectangle::new_from_math_rectangle(global_bounding_box);
                            bounding_box.overlaps(&surrounding_rectangle)
                        })
                        .collect();
                    object_session
                        .objects_to_be_detected
                        .push(ReferenceObject::new(object_id, reference_mosaics));
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
        session_id: String,
        object_id: String,
        ascii_art: String,
    ) -> AddObjectToBeDetectedResult {
        if let Some(session) = self.sessions.get_mut(&session_id) {
            match session {
                Session::Object(_object_session) => {
                    let image = WrappedRgbImage::new_from_ascii_art(ascii_art.as_str());
                    let image_guard = image.image.lock().unwrap();
                    let width = image_guard.width() as f64;
                    let height = image_guard.height() as f64;
                    drop(image_guard);
                    let surrounding_rectangle = Rectangle::new(
                        Vec3d::new(0.0, 0.0, 0.0),
                        Vec3d::new(width, height, 0.0),
                    );
                    self.add_object_to_be_detected_as_image(
                        session_id,
                        object_id,
                        image,
                        surrounding_rectangle,
                    );
                }
                _ => return AddObjectToBeDetectedResult::SessionTypeDoesNotSupportAddingObjectToBeDetected,
            }
            AddObjectToBeDetectedResult::Success
        } else {
            AddObjectToBeDetectedResult::SessionNotFound
        }
    }

    pub fn delete_session(&mut self, session_id: &String) -> DeleteSessionResult {
        if self.sessions.remove(session_id).is_some() {
            DeleteSessionResult::Success
        } else {
            DeleteSessionResult::SessionNotFound
        }
    }

    pub fn delete_reference_object(
        &mut self,
        session_id: &String,
        object_id: String,
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
    pub fn get_ordinary_session(&self, session_id: &String) -> Option<OrdinarySession> {
        if let Some(Session::Ordinary(ordinary_session)) = self.sessions.get(session_id) {
            Some(ordinary_session.clone())
        } else {
            None
        }
    }

    pub fn get_eye_session(&self, session_id: &String) -> Option<EyeSession> {
        if let Some(Session::Eye(eye_session)) = self.sessions.get(session_id) {
            Some(eye_session.clone())
        } else {
            None
        }
    }

    pub fn get_object_session(&self, session_id: &String) -> Option<ObjectSession> {
        if let Some(Session::Object(object_session)) = self.sessions.get(session_id) {
            Some(object_session.clone())
        } else {
            None
        }
    }

    pub fn get_basic_params(&self, session_id: &String) -> Option<BasicParams> {
        match self.sessions.get(session_id) {
            Some(Session::Ordinary(ordinary_session)) => {
                Some(ordinary_session.basic_params.clone())
            }
            Some(Session::Eye(eye_session)) => Some(eye_session.basic_params.clone()),
            Some(Session::Object(object_session)) => Some(object_session.basic_params.clone()),
            None => None,
        }
    }

    pub fn get_tile_params(&self, session_id: &String) -> Option<TileParams> {
        match self.sessions.get(session_id) {
            Some(Session::Eye(eye_session)) => Some(eye_session.eye_params.tile_params.clone()),
            Some(Session::Object(object_session)) => {
                Some(object_session.object_detection_params.tile_params.clone())
            }
            _ => None,
        }
    }

    pub fn get_trace_params(&self, session_id: &String) -> Option<TraceParams> {
        match self.sessions.get(session_id) {
            Some(Session::Eye(eye_session)) => Some(eye_session.eye_params.trace_params.clone()),
            Some(Session::Object(object_session)) => {
                Some(object_session.object_detection_params.trace_params.clone())
            }
            _ => None,
        }
    }

    pub fn get_bucket_delta(&self, session_id: &String) -> Option<f64> {
        match self.sessions.get(session_id) {
            Some(Session::Eye(eye_session)) => Some(eye_session.eye_params.bucket_delta),
            Some(Session::Object(object_session)) => {
                Some(object_session.object_detection_params.bucket_delta)
            }
            _ => None,
        }
    }

    pub fn get_target_similarity(&self, session_id: &String) -> Option<f64> {
        match self.sessions.get(session_id) {
            Some(Session::Eye(eye_session)) => Some(eye_session.eye_params.target_similarity),
            Some(Session::Object(object_session)) => {
                Some(object_session.object_detection_params.target_similarity)
            }
            _ => None,
        }
    }

    pub fn get_rectangles(
        &self,
        session_id: String,
        image: WrappedRgbImage,
        previous_image: Option<WrappedRgbImage>,
    ) -> GetRectanglesResult {
        let (results, mosaics) = match self.sessions.get(&session_id) {
            Some(Session::Eye(eye_session)) => match previous_image {
                Some(previous_image) => (
                    eye_session.results,
                    calculate_eye(eye_session, image, previous_image),
                ),
                None => return GetRectanglesResult::PreviousImageRequiredForEyeSession,
            },
            Some(Session::Object(object_session)) => (
                object_session.results,
                calculate_object(object_session, image),
            ),
            Some(Session::Ordinary(ordinary_session)) => (
                ordinary_session.results,
                calculate_ordinary(ordinary_session, image),
            ),
            None => return GetRectanglesResult::SessionNotFound,
        };
        GetRectanglesResult::Success(
            mosaics
                .into_iter()
                .map(|colored_relative_mosaic| {
                    deduce_enriched_mosaic(
                        colored_relative_mosaic.mosaic,
                        colored_relative_mosaic.color,
                        results,
                    )
                })
                .collect(),
        )
    }
}

fn calculate_ordinary_mosaics(
    basic_params: BasicParams,
    image: WrappedRgbImage,
) -> Vec<WrappedMosaic> {
    let width = image.image.lock().unwrap().width() as usize;
    let height = image.image.lock().unwrap().height() as usize;
    let rectangle = Rectangle::new(
        Vec3d::new(0.0, 0.0, 0.0),
        Vec3d::new(width as f64, height as f64, 0.0),
    );
    let slices = calculate_slices(image.clone(), rectangle, basic_params);
    let connected_slices = find_connected_slices(&mut slices.clone());
    deduce_mosaics(connected_slices.clone())
}

/// Internal pairing of a color and a wrapped relative mosaic before enrichment.
struct ColoredRelativeMosaic {
    color: Color,
    mosaic: WrappedRelativeMosaic,
}

fn deduce_enriched_mosaic(
    wrapped_relative_mosaic: WrappedRelativeMosaic,
    color: Color,
    results: Results,
) -> EnrichedMosaic {
    let mosaic = wrapped_relative_mosaic.get_mosaic();
    let slice_matrix = mosaic.get_slice_matrix();
    let global_coordinate_system = WrappedCoordinateSystem::new(
        Vec3d::new(0.0, 0.0, 0.0),
        Vec3d::new(1.0, 0.0, 0.0),
        Vec3d::new(0.0, 1.0, 0.0),
    );
    let slice_matrix_output = slice_matrix
        .get_slice_lines()
        .into_iter()
        .map(|slice_line| SliceLine {
            line_number: slice_line.get_line_number(),
            slices: slice_line
                .get_slices()
                .into_iter()
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
    let (bounding_box, bounding_circle, coordinated_center_of_mass, area) = match results {
        Results::Absolute => (
            mosaic.get_bounding_box(),
            mosaic.get_bounding_circle(),
            mosaic.get_center_of_mass(),
            mosaic.get_area(),
        ),
        Results::Relative => (
            wrapped_relative_mosaic.get_bounding_box(),
            wrapped_relative_mosaic.get_bounding_circle(),
            wrapped_relative_mosaic.get_center_of_mass(),
            wrapped_relative_mosaic.get_area(),
        ),
    };
    let global_bounding_box = bounding_box.to_global_rectangle();
    let global_bounding_circle_center = bounding_circle
        .get_center()
        .convert_to(global_coordinate_system.clone())
        .get_local_point();
    let global_center_of_mass =
        coordinated_center_of_mass.convert_to(global_coordinate_system.clone());
    EnrichedMosaic {
        bounding_box: slices::Rectangle::new_from_math_rectangle(global_bounding_box),
        bounding_circle: Circle {
            center: global_bounding_circle_center,
            radius: bounding_circle.get_radius(),
        },
        color,
        area,
        center_of_mass: global_center_of_mass.get_local_point(),
        slice_matrix: slice_matrix_output,
        average_color: RgbColor {
            red: mosaic.get_average_color().x as u8,
            green: mosaic.get_average_color().y as u8,
            blue: mosaic.get_average_color().z as u8,
        },
        results,
    }
}

fn calculate_ordinary(
    ordinary_session: &OrdinarySession,
    image: WrappedRgbImage,
) -> Vec<ColoredRelativeMosaic> {
    let image_guard = image.image.lock().unwrap();
    let image_width = image_guard.width() as f64;
    let image_height = image_guard.height() as f64;
    drop(image_guard);
    let surrounding_rectangle = crate::math::Rectangle::new(
        Vec3d::new(0.0, 0.0, 0.0),
        Vec3d::new(image_width, image_height, 0.0),
    );
    let mosaics = calculate_ordinary_mosaics(ordinary_session.basic_params.clone(), image);
    mosaics
        .into_iter()
        .map(|mosaic| ColoredRelativeMosaic {
            color: Color::Green,
            mosaic: WrappedRelativeMosaic::new(mosaic, surrounding_rectangle.clone()),
        })
        .collect()
}

fn calculate_eye(
    eye_session: &EyeSession,
    image: WrappedRgbImage,
    previous_image: WrappedRgbImage,
) -> Vec<ColoredRelativeMosaic> {
    let image_width = image.image.lock().unwrap().width() as f64;
    let image_height = image.image.lock().unwrap().height() as f64;
    let surrounding_rectangle = Rectangle::new(
        Vec3d::new(0.0, 0.0, 0.0),
        Vec3d::new(image_width, image_height, 0.0),
    );
    let current_mosaics = calculate_ordinary_mosaics(eye_session.basic_params.clone(), image);
    let previous_mosaics =
        calculate_ordinary_mosaics(eye_session.basic_params.clone(), previous_image);
    let previous_bucketed_mosaics = deduce_bucketed_mosaics(
        previous_mosaics.clone(),
        surrounding_rectangle.clone(),
        eye_session.eye_params.tile_params.clone(),
        eye_session.eye_params.bucket_delta,
    );
    let rectangles = deduce_rectangles(
        previous_bucketed_mosaics,
        current_mosaics,
        eye_session.eye_params.clone(),
        surrounding_rectangle.clone(),
    );
    let surrounding_math_rectangle = crate::math::Rectangle::new(
        surrounding_rectangle.get_top_left(),
        surrounding_rectangle.get_bottom_right(),
    );
    rectangles
        .into_iter()
        .map(|colored_rectangle| ColoredRelativeMosaic {
            color: colored_rectangle.get_color(),
            mosaic: WrappedRelativeMosaic::new(
                colored_rectangle.get_mosaics()[0].clone(),
                surrounding_math_rectangle.clone(),
            ),
        })
        .collect()
}

fn calculate_object(
    object_session: &ObjectSession,
    image: WrappedRgbImage,
) -> Vec<ColoredRelativeMosaic> {
    let image_width = image.image.lock().unwrap().width() as f64;
    let image_height = image.image.lock().unwrap().height() as f64;
    let surrounding_rectangle = Rectangle::new(
        Vec3d::new(0.0, 0.0, 0.0),
        Vec3d::new(image_width, image_height, 0.0),
    );
    let current_mosaics = calculate_ordinary_mosaics(object_session.basic_params.clone(), image);
    let bucketed_mosaics = deduce_bucketed_mosaics(
        current_mosaics.clone(),
        surrounding_rectangle.clone(),
        object_session.object_detection_params.tile_params.clone(),
        object_session.object_detection_params.bucket_delta,
    );
    let mut rectangles = Vec::new();
    for reference_object in object_session.objects_to_be_detected.clone().into_iter() {
        rectangles.extend(detect_objects(
            reference_object.clone(),
            &bucketed_mosaics,
            object_session.object_detection_params.clone(),
            surrounding_rectangle.clone(),
        ));
    }
    let surrounding_math_rectangle = crate::math::Rectangle::new(
        surrounding_rectangle.get_top_left(),
        surrounding_rectangle.get_bottom_right(),
    );
    rectangles
        .into_iter()
        .map(|colored_rectangle| ColoredRelativeMosaic {
            color: colored_rectangle.get_color(),
            mosaic: WrappedRelativeMosaic::new(
                colored_rectangle.get_mosaics()[0].clone(),
                surrounding_math_rectangle.clone(),
            ),
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::{ImageBuffer, Rgb};

    const EPSILON: f64 = 1e-8;

    fn assert_float_eq(actual: f64, expected: f64) {
        assert!(
            (actual - expected).abs() <= EPSILON,
            "expected {expected}, got {actual}"
        );
    }

    fn basic_params_input() -> BasicParamsInput {
        BasicParamsInput {
            do_grayscale: false,
            gradient_threshold: 15,
        }
    }

    fn tile_params_input() -> TileParamsInput {
        TileParamsInput {
            tile_x: 0.5,
            tile_y: 0.5,
        }
    }

    fn trace_params_input() -> TraceParamsInput {
        TraceParamsInput {
            num_skeleton: 12,
            close_slice_threshold: 0.2,
        }
    }

    fn eye_params_input() -> EyeParamsInput {
        EyeParamsInput {
            tile_params: tile_params_input(),
            bucket_delta: 0.25,
            trace_params: trace_params_input(),
            target_similarity: 0.9,
        }
    }

    fn object_detection_params_input() -> ObjectDetectionParamsInput {
        ObjectDetectionParamsInput {
            tile_params: tile_params_input(),
            bucket_delta: 0.25,
            trace_params: trace_params_input(),
            target_similarity: 0.9,
        }
    }

    fn solid_image(color: [u8; 3]) -> WrappedRgbImage {
        WrappedRgbImage::new(ImageBuffer::from_pixel(4, 4, Rgb(color)))
    }

    fn larger_solid_image(color: [u8; 3]) -> WrappedRgbImage {
        WrappedRgbImage::new(ImageBuffer::from_pixel(16, 16, Rgb(color)))
    }

    fn object_ascii_art() -> String {
        ["##", "##"].join("\n")
    }

    fn tiny_surrounding_rectangle() -> Rectangle {
        Rectangle::new(Vec3d::new(0.0, 0.0, 0.0), Vec3d::new(4.0, 4.0, 0.0))
    }

    #[test]
    fn service_creates_sessions_and_exposes_typed_getters() {
        let mut service = Service::new();

        assert!(matches!(
            service.create_ordinary_session("ordinary".to_string(), basic_params_input(), Results::Absolute),
            CreateOrdinarySessionResult::Success
        ));
        assert!(matches!(
            service.create_eye_session("eye".to_string(), basic_params_input(), eye_params_input(), Results::Relative),
            CreateEyeSessionResult::Success
        ));
        assert!(matches!(
            service.create_object_session(
                "object".to_string(),
                basic_params_input(),
                object_detection_params_input(),
                Results::Absolute,
            ),
            CreateObjectSessionResult::Success
        ));

        assert!(service.get_ordinary_session(&"ordinary".to_string()).is_some());
        assert!(service.get_eye_session(&"eye".to_string()).is_some());
        assert!(service.get_object_session(&"object".to_string()).is_some());
        assert!(matches!(
            service.create_ordinary_session("ordinary".to_string(), basic_params_input(), Results::Absolute),
            CreateOrdinarySessionResult::SessionAlreadyExists
        ));
        assert!(matches!(
            service.create_eye_session("eye".to_string(), basic_params_input(), eye_params_input(), Results::Relative),
            CreateEyeSessionResult::SessionAlreadyExists
        ));
        assert!(matches!(
            service.create_object_session(
                "object".to_string(),
                basic_params_input(),
                object_detection_params_input(),
                Results::Absolute,
            ),
            CreateObjectSessionResult::SessionAlreadyExists
        ));
    }

    #[test]
    fn service_updates_supported_sessions_and_rejects_wrong_types() {
        let mut service = Service::new();
        service.create_ordinary_session("ordinary".to_string(), basic_params_input(), Results::Absolute);
        service.create_eye_session("eye".to_string(), basic_params_input(), eye_params_input(), Results::Relative);
        service.create_object_session(
            "object".to_string(),
            basic_params_input(),
            object_detection_params_input(),
            Results::Absolute,
        );

        assert!(matches!(
            service.update_basic_params(
                "ordinary".to_string(),
                BasicParamsInput {
                    do_grayscale: true,
                    gradient_threshold: 7,
                },
            ),
            UpdateBasicParamsResult::Success
        ));
        assert!(service.get_basic_params(&"ordinary".to_string()).unwrap().do_grayscale());
        assert_eq!(service.get_basic_params(&"ordinary".to_string()).unwrap().gradient_threshold(), 7);

        assert!(matches!(
            service.update_tile_params(
                "eye".to_string(),
                TileParamsInput {
                    tile_x: 0.25,
                    tile_y: 0.75,
                },
            ),
            TileParamsUpdateResult::Success
        ));
        assert_float_eq(service.get_tile_params(&"eye".to_string()).unwrap().relative_tile_x(), 0.25);
        assert_float_eq(service.get_tile_params(&"eye".to_string()).unwrap().relative_tile_y(), 0.75);
        assert!(matches!(
            service.update_tile_params("ordinary".to_string(), tile_params_input()),
            TileParamsUpdateResult::SessionTypeDoesNotSupportTileParams
        ));

        assert!(matches!(
            service.update_trace_params(
                "object".to_string(),
                TraceParamsInput {
                    num_skeleton: 24,
                    close_slice_threshold: 0.4,
                },
            ),
            TraceParamsUpdateResult::Success
        ));
        assert_eq!(service.get_trace_params(&"object".to_string()).unwrap().num_skeleton(), 24);
        assert_float_eq(
            service.get_trace_params(&"object".to_string()).unwrap().close_slice_threshold(),
            0.4,
        );
        assert!(matches!(
            service.update_trace_params("ordinary".to_string(), trace_params_input()),
            TraceParamsUpdateResult::SessionTypeDoesNotSupportTraceParams
        ));

        assert!(matches!(service.update_bucket_delta("eye".to_string(), 0.6), BucketDeltaUpdateResult::Success));
        assert_float_eq(service.get_bucket_delta(&"eye".to_string()).unwrap(), 0.6);
        assert!(matches!(
            service.update_bucket_delta("ordinary".to_string(), 0.6),
            BucketDeltaUpdateResult::SessionTypeDoesNotSupportBucketDelta
        ));

        assert!(matches!(
            service.update_target_similarity("object".to_string(), 0.82),
            TargetSimilarityUpdateResult::Success
        ));
        assert_float_eq(service.get_target_similarity(&"object".to_string()).unwrap(), 0.82);
        assert!(matches!(
            service.update_target_similarity("ordinary".to_string(), 0.82),
            TargetSimilarityUpdateResult::SessionTypeDoesNotSupportTargetSimilarity
        ));

        assert!(matches!(
            service.update_eye_params(
                "eye".to_string(),
                EyeParamsInput {
                    tile_params: TileParamsInput {
                        tile_x: 0.2,
                        tile_y: 0.4,
                    },
                    bucket_delta: 0.3,
                    trace_params: TraceParamsInput {
                        num_skeleton: 30,
                        close_slice_threshold: 0.1,
                    },
                    target_similarity: 0.88,
                },
            ),
            EyeParamsUpdateResult::Success
        ));
        assert_float_eq(service.get_bucket_delta(&"eye".to_string()).unwrap(), 0.3);
        assert_eq!(service.get_trace_params(&"eye".to_string()).unwrap().num_skeleton(), 30);
        assert_float_eq(service.get_target_similarity(&"eye".to_string()).unwrap(), 0.88);
        assert!(matches!(
            service.update_eye_params("ordinary".to_string(), eye_params_input()),
            EyeParamsUpdateResult::SessionTypeDoesNotSupportEyeParams
        ));

        assert!(matches!(
            service.update_object_detection_params(
                "object".to_string(),
                ObjectDetectionParamsInput {
                    tile_params: TileParamsInput {
                        tile_x: 0.4,
                        tile_y: 0.4,
                    },
                    bucket_delta: 0.5,
                    trace_params: TraceParamsInput {
                        num_skeleton: 20,
                        close_slice_threshold: 0.25,
                    },
                    target_similarity: 0.91,
                },
            ),
            ObjectDetectionParamsUpdateResult::Success
        ));
        assert_float_eq(service.get_tile_params(&"object".to_string()).unwrap().relative_tile_x(), 0.4);
        assert_float_eq(service.get_bucket_delta(&"object".to_string()).unwrap(), 0.5);
        assert_eq!(service.get_trace_params(&"object".to_string()).unwrap().num_skeleton(), 20);
        assert_float_eq(service.get_target_similarity(&"object".to_string()).unwrap(), 0.91);
        assert!(matches!(
            service.update_object_detection_params("eye".to_string(), object_detection_params_input()),
            ObjectDetectionParamsUpdateResult::SessionTypeDoesNotSupportObjectDetectionParams
        ));
    }

    #[test]
    fn service_adds_and_deletes_reference_objects_and_sessions() {
        let mut service = Service::new();
        service.create_object_session(
            "object".to_string(),
            basic_params_input(),
            object_detection_params_input(),
            Results::Absolute,
        );
        service.create_ordinary_session("ordinary".to_string(), basic_params_input(), Results::Absolute);

        assert!(matches!(
            service.add_object_to_be_detected_as_image(
                "object".to_string(),
                "ref-0".to_string(),
                solid_image([0, 0, 0]),
                tiny_surrounding_rectangle(),
            ),
            AddObjectToBeDetectedResult::Success
        ));

        assert!(matches!(
            service.add_object_to_be_detected_as_ascii_art(
                "object".to_string(),
                "ref-1".to_string(),
                object_ascii_art(),
            ),
            AddObjectToBeDetectedResult::Success
        ));
        assert_eq!(
            service
                .get_object_session(&"object".to_string())
                .unwrap()
                .objects_to_be_detected
                .len(),
            2
        );

        assert!(matches!(
            service.add_object_to_be_detected_as_ascii_art(
                "ordinary".to_string(),
                "ref-2".to_string(),
                object_ascii_art(),
            ),
            AddObjectToBeDetectedResult::SessionTypeDoesNotSupportAddingObjectToBeDetected
        ));
        assert!(matches!(
            service.delete_reference_object(&"ordinary".to_string(), "ref-1".to_string()),
            DeleteReferenceObjectResult::SessionTypeDoesNotSupportDeletingReferenceObject
        ));
        assert!(matches!(
            service.delete_reference_object(&"object".to_string(), "missing".to_string()),
            DeleteReferenceObjectResult::ReferenceObjectNotFound
        ));
        assert!(matches!(
            service.delete_reference_object(&"object".to_string(), "ref-0".to_string()),
            DeleteReferenceObjectResult::Success
        ));
        assert!(matches!(
            service.delete_reference_object(&"object".to_string(), "ref-1".to_string()),
            DeleteReferenceObjectResult::Success
        ));
        assert_eq!(
            service
                .get_object_session(&"object".to_string())
                .unwrap()
                .objects_to_be_detected
                .len(),
            0
        );
        assert!(matches!(service.delete_session(&"object".to_string()), DeleteSessionResult::Success));
        assert!(matches!(service.delete_session(&"object".to_string()), DeleteSessionResult::SessionNotFound));
    }

    #[test]
    fn service_rectangles_and_enrichment_helpers_return_expected_shapes() {
        let mut service = Service::new();
        service.create_ordinary_session("ordinary".to_string(), basic_params_input(), Results::Absolute);
        service.create_eye_session("eye".to_string(), basic_params_input(), eye_params_input(), Results::Relative);

        assert!(matches!(
            service.get_rectangles("missing".to_string(), solid_image([255, 255, 255]), None),
            GetRectanglesResult::SessionNotFound
        ));
        assert!(matches!(
            service.get_rectangles("eye".to_string(), solid_image([255, 255, 255]), None),
            GetRectanglesResult::PreviousImageRequiredForEyeSession
        ));

        let mosaics = calculate_ordinary_mosaics(
            BasicParams::new(false, 15),
            larger_solid_image([255, 255, 255]),
        );
        assert!(!mosaics.is_empty());

        let wrapped_relative_mosaic = WrappedRelativeMosaic::new(
            mosaics[0].clone(),
            crate::math::Rectangle::new(Vec3d::new(0.0, 0.0, 0.0), Vec3d::new(16.0, 16.0, 0.0)),
        );
        let enriched_absolute = deduce_enriched_mosaic(
            wrapped_relative_mosaic.clone(),
            Color::Green,
            Results::Absolute,
        );
        let enriched_relative = deduce_enriched_mosaic(
            wrapped_relative_mosaic,
            Color::Blue,
            Results::Relative,
        );

        assert!(matches!(
            service.get_rectangles(
                "ordinary".to_string(),
                larger_solid_image([255, 255, 255]),
                None,
            ),
            GetRectanglesResult::Success(_)
        ));
        assert!(enriched_absolute.color == Color::Green);
        assert!(enriched_relative.color == Color::Blue);
        assert!(enriched_absolute.area > 0.0);
        assert!(enriched_relative.area > 0.0);
        assert!(!enriched_absolute.slice_matrix.is_empty());
        assert_eq!(enriched_absolute.average_color.red, 255);
        assert_eq!(enriched_absolute.average_color.green, 255);
        assert_eq!(enriched_absolute.average_color.blue, 255);
        assert!(enriched_relative.center_of_mass.x >= 0.0);
        assert!(enriched_relative.center_of_mass.x <= 1.0);
        assert!(enriched_relative.center_of_mass.y >= 0.0);
        assert!(enriched_relative.center_of_mass.y <= 1.0);
    }
}

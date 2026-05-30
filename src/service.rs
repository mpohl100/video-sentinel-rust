use crate::eye::EyeParams;
use crate::eye::ImageDecompositionParams;
use crate::math::WrappedCoordinateSystem;
use crate::object_detection::ObjectDetectionParams;
use crate::slices::BasicParams;
use crate::traces::TraceParams;
use crate::slices::Rectangle;
use crate::slices::calculate_slices;
use crate::slices::find_connected_slices;
use crate::slices::WrappedRgbImage;
use crate::slices::Color;
use crate::mosaics::deduce_mosaics;

use rs_math3d::Vec3d;
use std::collections::BTreeMap;

#[derive(Clone)]
pub struct BasicParamsInput {
    pub do_grayscale: bool,
    pub gradient_threshold: u8,
}

#[derive(Clone)]
pub struct ImageDecompositionParamsInput {
    pub width: usize,
    pub height: usize,
    pub slice_width: usize,
    pub slice_height: usize,
}

#[derive(Clone)]
pub struct TraceParamsInput {
    pub num_skeleton: usize,
    pub close_slice_threshold: f64,
}

#[derive(Clone)]
pub struct EyeParamsInput {
    pub image_decomposition_params: ImageDecompositionParamsInput,
    pub bucket_delta: f64,
    pub trace_params: TraceParamsInput,
    pub target_similarity: f64,
}

#[derive(Clone)]
pub struct ObjectDetectionParamsInput {
    pub image_decomposition_params: ImageDecompositionParamsInput,
    pub bucket_delta: f64,
    pub trace_params: TraceParamsInput,
    pub target_similarity: f64,
}

#[derive(Clone)]
pub struct OrdinarySession {
    pub basic_params: BasicParams,
}

#[derive(Clone)]
pub struct EyeSession {
    pub basic_params: BasicParams,
    pub eye_params: EyeParams,
}

#[derive(Clone)]
pub struct ObjectSession {
    pub basic_params: BasicParams,
    pub object_detection_params: ObjectDetectionParams,
}

#[derive(Clone)]
pub struct Point{
    pub x: f64,
    pub y: f64,
}

#[derive(Clone)]
pub struct Slice {
    pub start: Point,
    pub end: Point,
}

#[derive(Clone)]
pub struct SliceLine{
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
pub struct EnrichedMosaic {
    pub bounding_box: Rectangle,
    pub color: Color,
    pub area: f64,
    pub center_of_mass: Vec3d,
    pub slice_matrix: Vec<SliceLine>,
    pub average_color: RgbColor,
}

#[derive(Clone)]
pub enum Session {
    Ordinary(OrdinarySession),
    Eye(EyeSession),
    Object(ObjectSession),
}

pub struct Service {
    sessions: BTreeMap<String, Session>,
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
    ) {
        let basic_params = BasicParams::new(
            basic_params_input.do_grayscale,
            basic_params_input.gradient_threshold,
        );
        self.sessions.insert(
            session_id,
            Session::Ordinary(OrdinarySession { basic_params }),
        );
    }

    pub fn create_eye_session(
        &mut self,
        session_id: String,
        basic_params_input: BasicParamsInput,
        eye_params_input: EyeParamsInput,
    ) {
        let basic_params = BasicParams::new(
            basic_params_input.do_grayscale,
            basic_params_input.gradient_threshold,
        );
        let image_decomposition_params = ImageDecompositionParams::new(
            eye_params_input.image_decomposition_params.width,
            eye_params_input.image_decomposition_params.height,
            eye_params_input.image_decomposition_params.slice_width,
            eye_params_input.image_decomposition_params.slice_height,
        );
        let trace_params = TraceParams::new(
            eye_params_input.trace_params.num_skeleton,
            eye_params_input.trace_params.close_slice_threshold,
        );
        let eye_params = EyeParams::new(
            image_decomposition_params,
            eye_params_input.bucket_delta,
            trace_params,
            eye_params_input.target_similarity,
        );
        self.sessions.insert(
            session_id,
            Session::Eye(EyeSession {
                basic_params,
                eye_params,
            }),
        );
    }

    pub fn create_object_session(
        &mut self,
        session_id: String,
        basic_params_input: BasicParamsInput,
        object_detection_params_input: ObjectDetectionParamsInput,
    ) {
        let basic_params = BasicParams::new(
            basic_params_input.do_grayscale,
            basic_params_input.gradient_threshold,
        );
        let image_decomposition_params = ImageDecompositionParams::new(
            object_detection_params_input
                .image_decomposition_params
                .width,
            object_detection_params_input
                .image_decomposition_params
                .height,
            object_detection_params_input
                .image_decomposition_params
                .slice_width,
            object_detection_params_input
                .image_decomposition_params
                .slice_height,
        );
        let trace_params = TraceParams::new(
            object_detection_params_input.trace_params.num_skeleton,
            object_detection_params_input
                .trace_params
                .close_slice_threshold,
        );
        let object_detection_params = ObjectDetectionParams::new(
            image_decomposition_params,
            object_detection_params_input.bucket_delta,
            trace_params,
            object_detection_params_input.target_similarity,
        );
        self.sessions.insert(
            session_id,
            Session::Object(ObjectSession {
                basic_params,
                object_detection_params,
            }),
        );
    }

    pub fn update_basic_params(
        &mut self,
        session_id: String,
        basic_params_input: BasicParamsInput,
    ) {
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
        }
    }

    pub fn update_eye_image_decomposition_params(
        &mut self,
        session_id: String,
        image_decomposition_params_input: ImageDecompositionParamsInput,
    ) {
        if let Some(session) = self.sessions.get_mut(&session_id) {
            match session {
                Session::Eye(eye_session) => {
                    eye_session.eye_params.image_decomposition_params =
                        ImageDecompositionParams::new(
                            image_decomposition_params_input.width,
                            image_decomposition_params_input.height,
                            image_decomposition_params_input.slice_width,
                            image_decomposition_params_input.slice_height,
                        );
                }
                Session::Object(object_session) => {
                    object_session
                        .object_detection_params
                        .image_decomposition_params = ImageDecompositionParams::new(
                        image_decomposition_params_input.width,
                        image_decomposition_params_input.height,
                        image_decomposition_params_input.slice_width,
                        image_decomposition_params_input.slice_height,
                    );
                }
                _ => {}
            }
        }
    }

    pub fn update_trace_params(
        &mut self,
        session_id: String,
        trace_params_input: TraceParamsInput,
    ) {
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
                _ => {}
            }
        }
    }

    pub fn update_bucket_delta(&mut self, session_id: String, bucket_delta: f64) {
        if let Some(session) = self.sessions.get_mut(&session_id) {
            match session {
                Session::Eye(eye_session) => {
                    eye_session.eye_params.bucket_delta = bucket_delta;
                }
                Session::Object(object_session) => {
                    object_session.object_detection_params.bucket_delta = bucket_delta;
                }
                _ => {}
            }
        }
    }

    pub fn update_target_similarity(&mut self, session_id: String, target_similarity: f64) {
        if let Some(session) = self.sessions.get_mut(&session_id) {
            match session {
                Session::Eye(eye_session) => {
                    eye_session.eye_params.target_similarity = target_similarity;
                }
                Session::Object(object_session) => {
                    object_session.object_detection_params.target_similarity = target_similarity;
                }
                _ => {}
            }
        }
    }

    pub fn update_eye_params(&mut self, session_id: String, eye_params_input: EyeParamsInput) {
        if let Some(session) = self.sessions.get_mut(&session_id) {
            match session {
                Session::Eye(eye_session) => {
                    let image_decomposition_params = ImageDecompositionParams::new(
                        eye_params_input.image_decomposition_params.width,
                        eye_params_input.image_decomposition_params.height,
                        eye_params_input.image_decomposition_params.slice_width,
                        eye_params_input.image_decomposition_params.slice_height,
                    );
                    let trace_params = TraceParams::new(
                        eye_params_input.trace_params.num_skeleton,
                        eye_params_input.trace_params.close_slice_threshold,
                    );
                    eye_session.eye_params = EyeParams::new(
                        image_decomposition_params,
                        eye_params_input.bucket_delta,
                        trace_params,
                        eye_params_input.target_similarity,
                    );
                }
                _ => {}
            }
        }
    }

    pub fn update_object_detection_params(
        &mut self,
        session_id: String,
        object_detection_params_input: ObjectDetectionParamsInput,
    ) {
        if let Some(session) = self.sessions.get_mut(&session_id) {
            match session {
                Session::Object(object_session) => {
                    let image_decomposition_params = ImageDecompositionParams::new(
                        object_detection_params_input
                            .image_decomposition_params
                            .width,
                        object_detection_params_input
                            .image_decomposition_params
                            .height,
                        object_detection_params_input
                            .image_decomposition_params
                            .slice_width,
                        object_detection_params_input
                            .image_decomposition_params
                            .slice_height,
                    );
                    let trace_params = TraceParams::new(
                        object_detection_params_input.trace_params.num_skeleton,
                        object_detection_params_input
                            .trace_params
                            .close_slice_threshold,
                    );
                    object_session.object_detection_params = ObjectDetectionParams::new(
                        image_decomposition_params,
                        object_detection_params_input.bucket_delta,
                        trace_params,
                        object_detection_params_input.target_similarity,
                    );
                }
                _ => {}
            }
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

    pub fn get_image_decomposition_params(
        &self,
        session_id: &String,
    ) -> Option<ImageDecompositionParams> {
        match self.sessions.get(session_id) {
            Some(Session::Eye(eye_session)) => {
                Some(eye_session.eye_params.image_decomposition_params.clone())
            }
            Some(Session::Object(object_session)) => Some(
                object_session
                    .object_detection_params
                    .image_decomposition_params
                    .clone(),
            ),
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

    fn get_rectangles(
        &self,
        session_id: String,
        previous_frame: WrappedRgbImage,
        next_frame: WrappedRgbImage,
    ) -> Vec<EnrichedRectangle> {
        match self.sessions.get(&session_id) {
            Some(Session::Eye(eye_session)) => {
                calculate_eye(eye_session, previous_frame, next_frame)
            }
            Some(Session::Object(object_session)) => {
                calculate_object(object_session, previous_frame, next_frame)
            }
            Some(Session::Ordinary(ordinary_session)) => {
                calculate_ordinary(ordinary_session, previous_frame, next_frame)
            }
            None => vec![],
        }
    }
}

fn calculate_ordinary(
    ordinary_session: &OrdinarySession,
    _previous_frame: WrappedRgbImage,
    next_frame: WrappedRgbImage,
) -> Vec<EnrichedMosaic> {
    let width = next_frame.image.lock().unwrap().width() as usize;
    let height = next_frame.image.lock().unwrap().height() as usize;
    let rectangle = Rectangle::new(
        Vec3d::new(0.0, 0.0, 0.0),
        Vec3d::new(width as f64, height as f64, 0.0),
    );
    let mut slices = calculate_slices(next_frame.clone(), rectangle, ordinary_session.basic_params.clone());
    let connected_slices = find_connected_slices(&mut slices);
    let mosaics = deduce_mosaics(connected_slices.clone());
    mosaics
        .into_iter()
        .map(|mosaic| {
            let slice_matrix = mosaic.get_slice_matrix();
            let global_coordinate_system = WrappedCoordinateSystem::new(
                Vec3d::new(0.0, 0.0, 0.0),
                Vec3d::new(1.0, 0.0, 0.0),
                Vec3d::new(0.0, 1.0, 0.0),
            );
            let slice_matrix_output = slice_matrix.get_slice_lines()
                .into_iter()
                .map(|slice_line| SliceLine {
                    line_number: slice_line.get_line_number(),
                    slices: slice_line
                        .get_slices()
                        .into_iter()
                        .map(|slice| Slice {
                            start: Point {
                                x: slice.get_start().convert_to(global_coordinate_system.clone()).get_x(),
                                y: slice.get_start().convert_to(global_coordinate_system.clone()).get_y(),
                            },
                            end: Point {
                                x: slice.get_end().convert_to(global_coordinate_system.clone()).get_x(),
                                y: slice.get_end().convert_to(global_coordinate_system.clone()).get_y(),
                            },
                        })
                        .collect(),
                })
                .collect();
            let coordinated_bounding_box = mosaic.get_bounding_box();
            let bounding_box = Rectangle::new_from_math_rectangle(coordinated_bounding_box.to_global_rectangle());
            let coordinated_center_of_mass = mosaic.get_center_of_mass();
            let global_center_of_mass = coordinated_center_of_mass.convert_to(global_coordinate_system.clone());
            EnrichedMosaic {
            bounding_box,
            color: Color::Green,
            area: mosaic.get_area(),
            center_of_mass: global_center_of_mass.get_local_point(),
            slice_matrix: slice_matrix_output,
            average_color: RgbColor {
                red: mosaic.get_average_color().x as u8,
                green: mosaic.get_average_color().y as u8,
                blue: mosaic.get_average_color().z as u8,
            },
}})
        .collect()
}

use crate::slices::BasicParams;
use crate::eye::EyeParams;
use crate::object_detection::ObjectDetectionParams;
use crate::eye::ImageDecompositionParams;
use crate::traces::TraceParams;
use std::collections::BTreeMap;

pub struct BasicParamsInput {
    pub do_grayscale: bool,
    pub gradient_threshold: u8,
}

pub struct ImageDecompositionParamsInput {
    pub width: usize,
    pub height: usize,
    pub slice_width: usize,
    pub slice_height: usize,
}

pub struct TraceParamsInput {
    pub num_skeleton: usize,
    pub close_slice_threshold: f64,
}

pub struct EyeParamsInput {
    pub image_decomposition_params: ImageDecompositionParamsInput,
    pub bucket_delta: f64,
    pub trace_params: TraceParamsInput,
    pub target_similarity: f64,
}

pub struct ObjectDetectionParamsInput {
    pub image_decomposition_params: ImageDecompositionParamsInput,
    pub bucket_delta: f64,
    pub trace_params: TraceParamsInput,
    pub target_similarity: f64,
}

pub struct OrdinarySession {
    pub basic_params: BasicParams,
}

pub struct EyeSession {
    pub basic_params: BasicParams,
    pub eye_params: EyeParams,
}

pub struct ObjectSession {
    pub basic_params: BasicParams,
    pub object_detection_params: ObjectDetectionParams,
}

pub enum Session {
    Ordinary(OrdinarySession),
    Eye(EyeSession),
    Object(ObjectSession),
}

pub struct Service{
    sessions: BTreeMap<String, Session>,
}

impl Service {
    pub fn new() -> Self {
        Service {
            sessions: BTreeMap::new(),
        }
    }

    pub fn create_ordinary_session(&mut self, session_id: String, basic_params_input: BasicParamsInput) {
        let basic_params = BasicParams::new(basic_params_input.do_grayscale, basic_params_input.gradient_threshold);
        self.sessions.insert(session_id, Session::Ordinary(OrdinarySession { basic_params }));
    }

    pub fn create_eye_session(&mut self, session_id: String, basic_params_input: BasicParamsInput, eye_params_input: EyeParamsInput) {
        let basic_params = BasicParams::new(basic_params_input.do_grayscale, basic_params_input.gradient_threshold);
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
        self.sessions.insert(session_id, Session::Eye(EyeSession { basic_params, eye_params }));
    }

    pub fn create_object_session(&mut self, session_id: String, basic_params_input: BasicParamsInput, object_detection_params_input: ObjectDetectionParamsInput) {
        let basic_params = BasicParams::new(basic_params_input.do_grayscale, basic_params_input.gradient_threshold);
        let image_decomposition_params = ImageDecompositionParams::new(
            object_detection_params_input.image_decomposition_params.width,
            object_detection_params_input.image_decomposition_params.height,
            object_detection_params_input.image_decomposition_params.slice_width,
            object_detection_params_input.image_decomposition_params.slice_height,
        );
        let trace_params = TraceParams::new(
            object_detection_params_input.trace_params.num_skeleton,
            object_detection_params_input.trace_params.close_slice_threshold,
        );
        let object_detection_params = ObjectDetectionParams::new(
            image_decomposition_params,
            object_detection_params_input.bucket_delta,
            trace_params,
            object_detection_params_input.target_similarity,);
        self.sessions.insert(session_id, Session::Object(ObjectSession { basic_params, object_detection_params }));
    }

    pub fn update_basic_params(&mut self, session_id: String, basic_params_input: BasicParamsInput) {
        if let Some(session) = self.sessions.get_mut(&session_id) {
            match session {
                Session::Ordinary(ordinary_session) => {
                    ordinary_session.basic_params = BasicParams::new(basic_params_input.do_grayscale, basic_params_input.gradient_threshold);
                },
                Session::Eye(eye_session) => {
                    eye_session.basic_params = BasicParams::new(basic_params_input.do_grayscale, basic_params_input.gradient_threshold);
                },
                Session::Object(object_session) => {
                    object_session.basic_params = BasicParams::new(basic_params_input.do_grayscale, basic_params_input.gradient_threshold);
                },
            }
        }
    }

    pub fn update_eye_image_decomposition_params(&mut self, session_id: String, image_decomposition_params_input: ImageDecompositionParamsInput) {
        if let Some(session) = self.sessions.get_mut(&session_id) {
            match session {
                Session::Eye(eye_session) => {
                    eye_session.eye_params.image_decomposition_params = ImageDecompositionParams::new(
                        image_decomposition_params_input.width,
                        image_decomposition_params_input.height,
                        image_decomposition_params_input.slice_width,
                        image_decomposition_params_input.slice_height,
                    );
                },
                Session::Object(object_session) => {
                    object_session.object_detection_params.image_decomposition_params = ImageDecompositionParams::new(
                        image_decomposition_params_input.width,
                        image_decomposition_params_input.height,
                        image_decomposition_params_input.slice_width,
                        image_decomposition_params_input.slice_height,
                    );
                },
                _ => {},
            }
        }
    }

    pub fn update_trace_params(&mut self, session_id: String, trace_params_input: TraceParamsInput) {
        if let Some(session) = self.sessions.get_mut(&session_id) {
            match session {
                Session::Eye(eye_session) => {
                    eye_session.eye_params.trace_params = TraceParams::new(
                        trace_params_input.num_skeleton,
                        trace_params_input.close_slice_threshold,
                    );
                },
                Session::Object(object_session) => {
                    object_session.object_detection_params.trace_params = TraceParams::new(
                        trace_params_input.num_skeleton,
                        trace_params_input.close_slice_threshold,
                    );
                },
                _ => {},
            }
        }
    }

    pub fn update_bucket_delta(&mut self, session_id: String, bucket_delta: f64) {
        if let Some(session) = self.sessions.get_mut(&session_id) {
            match session {
                Session::Eye(eye_session) => {
                    eye_session.eye_params.bucket_delta = bucket_delta;
                },
                Session::Object(object_session) => {
                    object_session.object_detection_params.bucket_delta = bucket_delta;
                },
                _ => {},
            }
        }
    }

    pub fn update_target_similarity(&mut self, session_id: String, target_similarity: f64) {
        if let Some(session) = self.sessions.get_mut(&session_id) {
            match session {
                Session::Eye(eye_session) => {
                    eye_session.eye_params.target_similarity = target_similarity;
                },
                Session::Object(object_session) => {
                    object_session.object_detection_params.target_similarity = target_similarity;
                },
                _ => {},
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
                },
                _ => {},
            }
        }
    }

    pub fn update_object_detection_params(&mut self, session_id: String, object_detection_params_input: ObjectDetectionParamsInput) {
        if let Some(session) = self.sessions.get_mut(&session_id) {
            match session {
                Session::Object(object_session) => {
                    let image_decomposition_params = ImageDecompositionParams::new(
                        object_detection_params_input.image_decomposition_params.width,
                        object_detection_params_input.image_decomposition_params.height,
                        object_detection_params_input.image_decomposition_params.slice_width,
                        object_detection_params_input.image_decomposition_params.slice_height,
                    );
                    let trace_params = TraceParams::new(
                        object_detection_params_input.trace_params.num_skeleton,
                        object_detection_params_input.trace_params.close_slice_threshold,
                    );
                    object_session.object_detection_params = ObjectDetectionParams::new(
                        image_decomposition_params,
                        object_detection_params_input.bucket_delta,
                        trace_params,
                        object_detection_params_input.target_similarity,
                    );
                },
                _ => {},
            }
        }
    }
}
mod funcs;
mod object_detection_params;
mod reference_object;

use funcs::combine_boxes;
pub use funcs::detect_objects;
#[cfg(test)]
use funcs::i32_from_f64;
pub use object_detection_params::ObjectDetectionParams;
pub use reference_object::ReferenceObject;

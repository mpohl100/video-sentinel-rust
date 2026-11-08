#![warn(clippy::pedantic)]
#![allow(
    clippy::missing_errors_doc,
    clippy::missing_panics_doc,
    clippy::too_many_lines,
    clippy::unused_async_trait_impl
)]

use crate::slices::WrappedRgbImage;
use num_traits::ToPrimitive;
use opencv::calib3d;
use opencv::core::{Mat, Point2d, Point3d, Vector};

#[derive(Clone, Debug, PartialEq)]
pub struct CameraIntrinsics {
    pub width: u32,
    pub height: u32,
    pub fx: f64,
    pub fy: f64,
    pub cx: f64,
    pub cy: f64,
    pub distortion_model: String,
    pub distortion: Vec<f64>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RigidTransform {
    pub translation: [f64; 3],
    pub rotation_xyzw: [f64; 4],
}

#[derive(Clone, Debug, PartialEq)]
pub struct ThreeDTriangleCalibration {
    pub depth_camera: CameraIntrinsics,
    pub rgb_camera: CameraIntrinsics,
    pub depth_to_rgb: RigidTransform,
}

#[derive(Clone, Debug, PartialEq)]
pub struct DepthFrame {
    pub width: u32,
    pub height: u32,
    /// Depth in metres; zero, negative, NaN, and infinite values are invalid.
    pub metres: Vec<f32>,
}

#[derive(Clone)]
pub struct ThreeDTriangleFrame {
    pub rgb: WrappedRgbImage,
    pub depth: DepthFrame,
}

#[derive(Clone, Debug, PartialEq)]
pub struct RegisteredDepth {
    pub width: u32,
    pub height: u32,
    /// RGB-camera Z in metres; `None` represents an invalid or uncovered pixel.
    pub metres: Vec<Option<f32>>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Triangle {
    pub vertices: [[f64; 3]; 3],
}

const MAX_IMAGE_PIXELS: usize = 64_000_000;

#[derive(Clone, Debug, PartialEq)]
pub enum ProcessThreeDTriangleFrameResult {
    Success(Vec<Triangle>),
    SessionNotFound,
    InvalidInput(String),
}

pub(super) fn validate_calibration(calibration: &ThreeDTriangleCalibration) -> Result<(), String> {
    validate_camera(&calibration.depth_camera)?;
    validate_camera(&calibration.rgb_camera)?;
    validate_transform(calibration.depth_to_rgb)
}

pub fn register_depth(
    depth: &DepthFrame,
    depth_camera: &CameraIntrinsics,
    rgb_camera: &CameraIntrinsics,
    depth_to_rgb: RigidTransform,
    rgb_width: u32,
    rgb_height: u32,
) -> Result<RegisteredDepth, String> {
    validate_camera(depth_camera)?;
    validate_camera(rgb_camera)?;
    validate_transform(depth_to_rgb)?;
    validate_dimensions(depth.width, depth.height)?;
    validate_dimensions(rgb_width, rgb_height)?;
    let depth_len = pixel_count(depth.width, depth.height)?;
    if depth.metres.len() != depth_len {
        return Err(format!(
            "depth buffer has {} samples, expected {depth_len}",
            depth.metres.len()
        ));
    }

    let output_len = pixel_count(rgb_width, rgb_height)?;
    let mut output = vec![None; output_len];
    let depth_intrinsics = scaled_intrinsics(depth_camera, depth.width, depth.height)?;
    let rgb_intrinsics = scaled_intrinsics(rgb_camera, rgb_width, rgb_height)?;
    let depth_matrix = camera_matrix(depth_intrinsics)?;
    let rgb_matrix = camera_matrix(rgb_intrinsics)?;
    let depth_distortion = distortion_matrix(depth_camera)?;
    let rgb_distortion = distortion_matrix(rgb_camera)?;
    let rotation = quaternion_matrix(depth_to_rgb.rotation_xyzw)?;

    let mut depth_pixels = Vector::<Point2d>::new();
    let mut depth_values = Vec::new();
    for y in 0..depth.height {
        for x in 0..depth.width {
            let source_index = usize::try_from(y).expect("u32 fits usize")
                * usize::try_from(depth.width).expect("u32 fits usize")
                + usize::try_from(x).expect("u32 fits usize");
            let z = f64::from(depth.metres[source_index]);
            if z.is_finite() && z > 0.0 {
                depth_pixels.push(Point2d::new(f64::from(x), f64::from(y)));
                depth_values.push(z);
            }
        }
    }
    if depth_pixels.is_empty() {
        return Ok(RegisteredDepth {
            width: rgb_width,
            height: rgb_height,
            metres: output,
        });
    }

    let mut normalized_depth_pixels = Vector::<Point2d>::new();
    calib3d::undistort_points_def(
        &depth_pixels,
        &mut normalized_depth_pixels,
        &depth_matrix,
        &depth_distortion,
    )
    .map_err(|error| format!("OpenCV failed to undistort depth pixels: {error}"))?;

    let mut rgb_points = Vector::<Point3d>::new();
    for (normalized, depth_metres) in normalized_depth_pixels.iter().zip(depth_values) {
        let depth_point = [
            normalized.x * depth_metres,
            normalized.y * depth_metres,
            depth_metres,
        ];
        let rgb_point = transform_point(rotation, depth_to_rgb.translation, depth_point);
        if rgb_point.iter().any(|coordinate| !coordinate.is_finite()) || rgb_point[2] <= 0.0 {
            continue;
        }
        rgb_points.push(Point3d::new(rgb_point[0], rgb_point[1], rgb_point[2]));
    }
    if rgb_points.is_empty() {
        return Ok(RegisteredDepth {
            width: rgb_width,
            height: rgb_height,
            metres: output,
        });
    }

    let identity_rotation = Vector::from_slice(&[0.0_f64; 3]);
    let zero_translation = Vector::from_slice(&[0.0_f64; 3]);
    let mut projected_rgb_pixels = Vector::<Point2d>::new();
    calib3d::project_points_def(
        &rgb_points,
        &identity_rotation,
        &zero_translation,
        &rgb_matrix,
        &rgb_distortion,
        &mut projected_rgb_pixels,
    )
    .map_err(|error| format!("OpenCV failed to project RGB pixels: {error}"))?;

    for (rgb_point, projected_pixel) in rgb_points.iter().zip(projected_rgb_pixels.iter()) {
        let (u, v) = (projected_pixel.x, projected_pixel.y);
        if !u.is_finite() || !v.is_finite() {
            continue;
        }
        let (pixel_x, pixel_y) = (u.round(), v.round());
        if pixel_x < 0.0
            || pixel_y < 0.0
            || pixel_x >= f64::from(rgb_width)
            || pixel_y >= f64::from(rgb_height)
        {
            continue;
        }
        let (Some(pixel_x), Some(pixel_y)) = (pixel_x.to_u32(), pixel_y.to_u32()) else {
            continue;
        };
        let index = usize::try_from(pixel_y).expect("u32 fits usize")
            * usize::try_from(rgb_width).expect("u32 fits usize")
            + usize::try_from(pixel_x).expect("u32 fits usize");
        let Some(projected_depth) = rgb_point
            .z
            .to_f32()
            .filter(|depth| depth.is_finite() && *depth > 0.0)
        else {
            continue;
        };
        if output[index].is_none_or(|old_depth| projected_depth < old_depth) {
            output[index] = Some(projected_depth);
        }
    }

    Ok(RegisteredDepth {
        width: rgb_width,
        height: rgb_height,
        metres: output,
    })
}

#[derive(Clone, Copy)]
struct ScaledIntrinsics {
    fx: f64,
    fy: f64,
    cx: f64,
    cy: f64,
}

fn validate_dimensions(width: u32, height: u32) -> Result<(), String> {
    if width == 0 || height == 0 {
        return Err("image dimensions must be non-zero".to_owned());
    }
    pixel_count(width, height)?;
    Ok(())
}

fn pixel_count(width: u32, height: u32) -> Result<usize, String> {
    let count = usize::try_from(width)
        .ok()
        .and_then(|w| usize::try_from(height).ok().and_then(|h| w.checked_mul(h)))
        .ok_or_else(|| "image dimensions overflow addressable memory".to_owned())?;
    if count > MAX_IMAGE_PIXELS {
        return Err(format!(
            "image contains {count} pixels; maximum supported is {MAX_IMAGE_PIXELS}"
        ));
    }
    Ok(count)
}

fn validate_camera(camera: &CameraIntrinsics) -> Result<(), String> {
    validate_dimensions(camera.width, camera.height)?;
    if !camera.fx.is_finite()
        || !camera.fy.is_finite()
        || camera.fx <= 0.0
        || camera.fy <= 0.0
        || !camera.cx.is_finite()
        || !camera.cy.is_finite()
        || camera.distortion.iter().any(|value| !value.is_finite())
    {
        return Err(
            "camera intrinsics and distortion must be finite with positive focal lengths"
                .to_owned(),
        );
    }
    if camera.distortion_model != "plumb_bob"
        && camera.distortion_model != "rational_polynomial"
        && camera.distortion_model != "none"
        && !camera.distortion_model.is_empty()
    {
        return Err(format!(
            "unsupported distortion model `{}`; use plumb_bob, rational_polynomial, or none",
            camera.distortion_model
        ));
    }
    let max_coefficients = if camera.distortion_model == "rational_polynomial" {
        8
    } else {
        5
    };
    if camera.distortion.len() > max_coefficients {
        return Err(format!(
            "distortion model `{}` accepts at most {max_coefficients} coefficients",
            camera.distortion_model
        ));
    }
    Ok(())
}

fn validate_transform(transform: RigidTransform) -> Result<(), String> {
    if transform.translation.iter().any(|value| !value.is_finite())
        || transform
            .rotation_xyzw
            .iter()
            .any(|value| !value.is_finite())
    {
        return Err("depth-to-RGB transform must be finite".to_owned());
    }
    let norm = transform
        .rotation_xyzw
        .iter()
        .map(|value| value * value)
        .sum::<f64>()
        .sqrt();
    if norm <= f64::EPSILON {
        return Err("depth-to-RGB quaternion must have non-zero norm".to_owned());
    }
    Ok(())
}

fn scaled_intrinsics(
    camera: &CameraIntrinsics,
    width: u32,
    height: u32,
) -> Result<ScaledIntrinsics, String> {
    let sx = f64::from(width) / f64::from(camera.width);
    let sy = f64::from(height) / f64::from(camera.height);
    if !sx.is_finite() || !sy.is_finite() || sx <= 0.0 || sy <= 0.0 {
        return Err("invalid camera resolution scaling".to_owned());
    }
    let scaled = ScaledIntrinsics {
        fx: camera.fx * sx,
        fy: camera.fy * sy,
        cx: camera.cx * sx,
        cy: camera.cy * sy,
    };
    if !scaled.fx.is_finite()
        || !scaled.fy.is_finite()
        || scaled.fx <= 0.0
        || scaled.fy <= 0.0
        || !scaled.cx.is_finite()
        || !scaled.cy.is_finite()
    {
        return Err("scaled camera intrinsics are invalid".to_owned());
    }
    Ok(scaled)
}

fn camera_matrix(intrinsics: ScaledIntrinsics) -> Result<Mat, String> {
    Mat::from_slice_2d(&[
        [intrinsics.fx, 0.0, intrinsics.cx],
        [0.0, intrinsics.fy, intrinsics.cy],
        [0.0, 0.0, 1.0],
    ])
    .map_err(|error| format!("OpenCV failed to create camera matrix: {error}"))
}

fn distortion_matrix(camera: &CameraIntrinsics) -> Result<Vector<f64>, String> {
    if camera.distortion_model == "none" || camera.distortion_model.is_empty() {
        return Ok(Vector::new());
    }
    let coefficient_count = if camera.distortion_model == "rational_polynomial" {
        8
    } else {
        5
    };
    let mut coefficients = vec![0.0; coefficient_count];
    if camera.distortion.len() > coefficients.len() {
        return Err("unsupported distortion coefficient count".to_owned());
    }
    coefficients[..camera.distortion.len()].copy_from_slice(&camera.distortion);
    Ok(Vector::from_slice(&coefficients))
}

fn quaternion_matrix(quaternion: [f64; 4]) -> Result<[[f64; 3]; 3], String> {
    let norm = quaternion
        .iter()
        .map(|value| value * value)
        .sum::<f64>()
        .sqrt();
    if !norm.is_finite() || norm <= f64::EPSILON {
        return Err("depth-to-RGB quaternion must have non-zero norm".to_owned());
    }
    let [x, y, z, w] = quaternion.map(|value| value / norm);
    Ok([
        [
            1.0 - 2.0 * (y * y + z * z),
            2.0 * (x * y - z * w),
            2.0 * (x * z + y * w),
        ],
        [
            2.0 * (x * y + z * w),
            1.0 - 2.0 * (x * x + z * z),
            2.0 * (y * z - x * w),
        ],
        [
            2.0 * (x * z - y * w),
            2.0 * (y * z + x * w),
            1.0 - 2.0 * (x * x + y * y),
        ],
    ])
}

fn transform_point(rotation: [[f64; 3]; 3], translation: [f64; 3], point: [f64; 3]) -> [f64; 3] {
    [
        rotation[0][0].mul_add(
            point[0],
            rotation[0][1].mul_add(point[1], rotation[0][2] * point[2]),
        ) + translation[0],
        rotation[1][0].mul_add(
            point[0],
            rotation[1][1].mul_add(point[1], rotation[1][2] * point[2]),
        ) + translation[1],
        rotation[2][0].mul_add(
            point[0],
            rotation[2][1].mul_add(point[1], rotation[2][2] * point[2]),
        ) + translation[2],
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mosaics::Results;
    use crate::service::{
        BasicParamsInput, CreateThreeDTriangleSessionResult, Service, ThreeDTriangleCalibration,
        ThreeDTriangleFrame,
    };

    fn camera(width: u32, height: u32) -> CameraIntrinsics {
        CameraIntrinsics {
            width,
            height,
            fx: 1.0,
            fy: 1.0,
            cx: 0.0,
            cy: 0.0,
            distortion_model: "none".to_owned(),
            distortion: vec![],
        }
    }

    fn identity() -> RigidTransform {
        RigidTransform {
            translation: [0.0; 3],
            rotation_xyzw: [0.0, 0.0, 0.0, 1.0],
        }
    }

    #[test]
    fn registers_depth_at_higher_resolution_without_output_sized_scratch_assumptions() {
        let depth_camera = camera(4, 2);
        let rgb_camera = camera(2, 1);
        let depth = DepthFrame {
            width: 4,
            height: 2,
            metres: vec![1.0; 8],
        };
        let registered = register_depth(&depth, &depth_camera, &rgb_camera, identity(), 2, 1)
            .expect("larger depth input should register safely");
        assert_eq!(registered.metres, vec![Some(1.0), Some(1.0)]);
    }

    #[test]
    fn registers_depth_at_lower_resolution() {
        let depth_camera = camera(2, 1);
        let rgb_camera = camera(4, 2);
        let depth = DepthFrame {
            width: 2,
            height: 1,
            metres: vec![1.0, 2.0],
        };
        let registered = register_depth(&depth, &depth_camera, &rgb_camera, identity(), 4, 2)
            .expect("smaller depth input should register");
        assert_eq!(registered.metres[0], Some(1.0));
        assert_eq!(registered.metres[1], Some(2.0));
    }

    #[test]
    fn uses_opencv_distortion_for_unprojection_and_projection() {
        let mut distorted_camera = camera(5, 1);
        distorted_camera.cx = 2.0;
        distorted_camera.distortion_model = "plumb_bob".to_owned();
        distorted_camera.distortion = vec![1.0, 0.0, 0.0, 0.0, 0.0];
        let mut pinhole_camera = camera(5, 1);
        pinhole_camera.cx = 2.0;
        let source_depth = DepthFrame {
            width: 5,
            height: 1,
            metres: vec![0.0, 0.0, 0.0, 0.0, 1.0],
        };
        let unprojected = register_depth(
            &source_depth,
            &distorted_camera,
            &pinhole_camera,
            identity(),
            5,
            1,
        )
        .expect("OpenCV should undistort source pixels");
        assert_eq!(unprojected.metres[3], Some(1.0));

        let source_depth = DepthFrame {
            width: 5,
            height: 1,
            metres: vec![0.0, 0.0, 0.0, 1.0, 0.0],
        };
        let projected = register_depth(
            &source_depth,
            &pinhole_camera,
            &distorted_camera,
            identity(),
            5,
            1,
        )
        .expect("OpenCV should distort projected pixels");
        assert_eq!(projected.metres[4], Some(1.0));
    }

    #[test]
    fn filters_invalid_negative_and_behind_camera_depth() {
        let camera = camera(3, 1);
        let transform = RigidTransform {
            translation: [0.0, 0.0, -2.0],
            rotation_xyzw: [0.0, 0.0, 0.0, 1.0],
        };
        let depth = DepthFrame {
            width: 3,
            height: 1,
            metres: vec![f32::NAN, -1.0, 1.0],
        };
        let registered = register_depth(&depth, &camera, &camera, transform, 3, 1)
            .expect("invalid samples should be skipped");
        assert_eq!(registered.metres, vec![None; 3]);
    }

    #[test]
    fn requires_a_real_transform_quaternion() {
        let camera = camera(1, 1);
        let depth = DepthFrame {
            width: 1,
            height: 1,
            metres: vec![1.0],
        };
        let error = register_depth(
            &depth,
            &camera,
            &camera,
            RigidTransform {
                translation: [0.0; 3],
                rotation_xyzw: [0.0; 4],
            },
            1,
            1,
        )
        .expect_err("zero quaternion should be rejected");
        assert!(error.contains("quaternion"));
    }

    #[tokio::test]
    async fn triangle_session_returns_empty_triangles_and_enforces_unique_ids() {
        let mut service = Service::new();
        let camera = camera(1, 1);
        let calibration = ThreeDTriangleCalibration {
            depth_camera: camera.clone(),
            rgb_camera: camera,
            depth_to_rgb: identity(),
        };
        assert_eq!(
            service.create_three_d_triangle_session("triangles", Some(calibration.clone())),
            CreateThreeDTriangleSessionResult::Success
        );
        assert_eq!(
            service.create_three_d_triangle_session("triangles", Some(calibration)),
            CreateThreeDTriangleSessionResult::SessionAlreadyExists
        );
        let request = service
            .get_triangles_request("triangles", None)
            .expect("known session should return a request");
        assert_eq!(
            request
                .get_triangles_with(|| {
                    Ok(ThreeDTriangleFrame {
                        rgb: crate::slices::WrappedRgbImage::new_with_color(1, 1, [0; 3]),
                        depth: DepthFrame {
                            width: 1,
                            height: 1,
                            metres: vec![1.0],
                        },
                    })
                })
                .await,
            ProcessThreeDTriangleFrameResult::Success(vec![])
        );
        assert!(service.get_triangles_request("missing", None).is_none());

        assert!(matches!(
            service.create_ordinary_session(
                "ordinary",
                BasicParamsInput {
                    do_grayscale: true,
                    gradient_threshold: 15,
                },
                Results::Absolute,
            ),
            crate::service::CreateOrdinarySessionResult::Success
        ));
        assert_eq!(
            service.create_three_d_triangle_session("ordinary", None),
            CreateThreeDTriangleSessionResult::SessionAlreadyExists
        );
    }
}

#![warn(clippy::pedantic)]
#![allow(
    clippy::missing_errors_doc,
    clippy::missing_panics_doc,
    clippy::too_many_lines,
    clippy::unused_async_trait_impl
)]

use serde::{Deserialize, Serialize};

use super::Header;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct CameraInfo {
    pub header: Header,
    pub height: u32,
    pub width: u32,
    pub distortion_model: String,
    pub d: Vec<f64>,
    #[serde(with = "serde_big_array::BigArray")]
    pub k: [f64; 9],
    #[serde(with = "serde_big_array::BigArray")]
    pub r: [f64; 9],
    #[serde(with = "serde_big_array::BigArray")]
    pub p: [f64; 12],
    pub binning_x: u32,
    pub binning_y: u32,
    pub roi: RegionOfInterest,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct RegionOfInterest {
    pub x_offset: u32,
    pub y_offset: u32,
    pub height: u32,
    pub width: u32,
    pub do_rectify: bool,
}

impl CameraInfo {
    pub fn camera_intrinsics(&self) -> Result<vs_service::service::CameraIntrinsics, String> {
        let frame_width = self.width / self.binning_x.max(1);
        let frame_height = self.height / self.binning_y.max(1);
        let (roi_width, roi_height) = if self.roi.width == 0 || self.roi.height == 0 {
            (frame_width, frame_height)
        } else {
            (
                self.roi.width / self.binning_x.max(1),
                self.roi.height / self.binning_y.max(1),
            )
        };
        let width = roi_width.max(1);
        let height = roi_height.max(1);
        let binning_x = f64::from(self.binning_x.max(1));
        let binning_y = f64::from(self.binning_y.max(1));
        let cx = (self.k[2] - f64::from(self.roi.x_offset)) / binning_x;
        let cy = (self.k[5] - f64::from(self.roi.y_offset)) / binning_y;
        let fx = self.k[0] / binning_x;
        let fy = self.k[4] / binning_y;
        if self.k[0] == 0.0 || self.width == 0 || self.height == 0 {
            return Err("CameraInfo is uncalibrated or has zero dimensions".to_owned());
        }
        Ok(vs_service::service::CameraIntrinsics {
            width,
            height,
            fx,
            fy,
            cx,
            cy,
            distortion_model: self.distortion_model.clone(),
            distortion: self.d.clone(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ros2_client::builtin_interfaces::Time;

    #[test]
    fn applies_roi_and_binning_to_raw_intrinsics_without_using_rectification_matrices() {
        let camera_info = CameraInfo {
            header: Header {
                stamp: Time::ZERO,
                frame_id: "camera_optical".to_owned(),
            },
            height: 4,
            width: 8,
            distortion_model: "plumb_bob".to_owned(),
            d: vec![0.1, 0.2, 0.0, 0.0, 0.0],
            k: [10.0, 0.0, 6.0, 0.0, 8.0, 5.0, 0.0, 0.0, 1.0],
            r: [0.0; 9],
            p: [0.0; 12],
            binning_x: 2,
            binning_y: 1,
            roi: RegionOfInterest {
                x_offset: 2,
                y_offset: 1,
                width: 8,
                height: 4,
                do_rectify: false,
            },
        };
        let intrinsics = camera_info
            .camera_intrinsics()
            .expect("calibrated CameraInfo should convert");
        assert_eq!((intrinsics.width, intrinsics.height), (4, 4));
        assert_eq!((intrinsics.fx, intrinsics.fy), (5.0, 8.0));
        assert_eq!((intrinsics.cx, intrinsics.cy), (2.0, 4.0));
        assert_eq!(intrinsics.distortion, camera_info.d);
    }
}

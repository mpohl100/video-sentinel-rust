#![warn(clippy::pedantic)]
#![allow(
    clippy::missing_errors_doc,
    clippy::missing_panics_doc,
    clippy::too_many_lines,
    clippy::unused_async_trait_impl
)]

use opencv::core::{self, Mat, Scalar, Vec3b, Vector as CvVector};
use opencv::imgcodecs;
use opencv::imgproc;
use opencv::prelude::*;
use std::sync::{Arc, Mutex, MutexGuard};

use super::{i32_from_u32, point_in_polygon, u32_from_i32, u32_from_usize, usize_from_u32};

#[derive(Clone)]
pub struct WrappedRgbImage {
    image: Arc<Mutex<Mat>>,
}

impl WrappedRgbImage {
    #[must_use]
    pub fn new(image: Mat) -> Self {
        Self {
            image: Arc::new(Mutex::new(image)),
        }
    }

    #[must_use]
    pub fn new_with_color(width: u32, height: u32, color: [u8; 3]) -> Self {
        let image = Mat::new_rows_cols_with_default(
            i32_from_u32(height),
            i32_from_u32(width),
            core::CV_8UC3,
            Scalar::new(
                f64::from(color[0]),
                f64::from(color[1]),
                f64::from(color[2]),
                0.0,
            ),
        )
        .expect("failed to create RGB image");
        Self::new(image)
    }

    pub fn from_rgb_data(width: u32, height: u32, data: &[u8]) -> Result<Self, opencv::Error> {
        let expected_len = usize_from_u32(width) * usize_from_u32(height) * 3;
        if data.len() != expected_len {
            return Err(opencv::Error::new(
                core::StsUnmatchedSizes,
                format!(
                    "expected {expected_len} bytes for a {width}x{height} RGB image, got {}",
                    data.len()
                ),
            ));
        }

        let mut image = Mat::new_rows_cols_with_default(
            i32_from_u32(height),
            i32_from_u32(width),
            core::CV_8UC3,
            Scalar::default(),
        )?;
        image.data_bytes_mut()?.copy_from_slice(data);
        Ok(Self::new(image))
    }

    pub fn from_file(path: &str) -> Result<Self, opencv::Error> {
        let bgr = imgcodecs::imread(path, imgcodecs::IMREAD_COLOR)?;
        let mut rgb = Mat::default();
        imgproc::cvt_color(&bgr, &mut rgb, imgproc::COLOR_BGR2RGB, 0)?;
        Ok(Self::new(rgb))
    }

    pub fn from_jpeg_bytes(bytes: &[u8]) -> Result<Self, opencv::Error> {
        let compressed = CvVector::<u8>::from_slice(bytes);
        let bgr = imgcodecs::imdecode(&compressed, imgcodecs::IMREAD_COLOR)?;
        let mut rgb = Mat::default();
        imgproc::cvt_color(&bgr, &mut rgb, imgproc::COLOR_BGR2RGB, 0)?;
        Ok(Self::new(rgb))
    }

    #[must_use]
    pub fn new_from_ascii_art(ascii_art: &str) -> Self {
        let lines: Vec<&str> = ascii_art.lines().collect();
        let height = u32_from_usize(lines.len());
        let width = u32_from_usize(lines.iter().map(|line| line.len()).max().unwrap_or(0));
        let image = WrappedRgbImage::new_with_color(width, height, [0, 0, 0]);
        for (y, line) in lines.iter().enumerate() {
            for (x, char) in line.chars().enumerate() {
                let pixel_value = if char == '#' { 255 } else { 0 };
                image.set_pixel(
                    u32_from_usize(x),
                    u32_from_usize(y),
                    [pixel_value, pixel_value, pixel_value],
                );
            }
        }
        image
    }

    #[must_use]
    pub fn width(&self) -> u32 {
        u32_from_i32(self.image.lock().unwrap().cols())
    }

    #[must_use]
    pub fn height(&self) -> u32 {
        u32_from_i32(self.image.lock().unwrap().rows())
    }

    pub fn get_mat(&self) -> MutexGuard<'_, Mat> {
        self.image.lock().unwrap()
    }

    #[must_use]
    pub fn get_pixel(&self, x: u32, y: u32) -> [u8; 3] {
        let image = self.image.lock().unwrap();
        let pixel = image
            .at_2d::<Vec3b>(i32_from_u32(y), i32_from_u32(x))
            .expect("failed to read RGB pixel");
        [pixel[0], pixel[1], pixel[2]]
    }

    pub fn set_pixel(&self, x: u32, y: u32, color: [u8; 3]) {
        let mut image = self.image.lock().unwrap();
        let pixel = image
            .at_2d_mut::<Vec3b>(i32_from_u32(y), i32_from_u32(x))
            .expect("failed to write RGB pixel");
        *pixel = Vec3b::from(color);
    }

    pub fn rgb_data(&self) -> Result<Vec<u8>, opencv::Error> {
        let image = self.image.lock().unwrap();
        Ok(image.data_bytes()?.to_vec())
    }

    pub fn fill_polygon(&self, points: &[(i32, i32)], color: [u8; 3]) {
        if points.len() < 3 {
            return;
        }
        let min_x = u32_from_i32(points.iter().map(|(x, _)| *x).min().unwrap().max(0));
        let min_y = u32_from_i32(points.iter().map(|(_, y)| *y).min().unwrap().max(0));
        let max_x = points
            .iter()
            .map(|(x, _)| *x)
            .max()
            .unwrap()
            .min(i32_from_u32(self.width()) - 1);
        let max_y = points
            .iter()
            .map(|(_, y)| *y)
            .max()
            .unwrap()
            .min(i32_from_u32(self.height()) - 1);
        let max_x = u32_from_i32(max_x);
        let max_y = u32_from_i32(max_y);

        for y in min_y..=max_y {
            for x in min_x..=max_x {
                if point_in_polygon((f64::from(x), f64::from(y)), points) {
                    self.set_pixel(x, y, color);
                }
            }
        }
    }

    pub fn fill_circle(&self, center: (i32, i32), radius: i32, color: [u8; 3]) {
        let min_x = u32_from_i32((center.0 - radius).max(0));
        let min_y = u32_from_i32((center.1 - radius).max(0));
        let max_x = u32_from_i32((center.0 + radius).min(i32_from_u32(self.width()) - 1));
        let max_y = u32_from_i32((center.1 + radius).min(i32_from_u32(self.height()) - 1));
        let radius_squared = f64::from(radius * radius);

        for y in min_y..=max_y {
            for x in min_x..=max_x {
                let dx = f64::from(x) - f64::from(center.0);
                let dy = f64::from(y) - f64::from(center.1);
                if dx * dx + dy * dy <= radius_squared {
                    self.set_pixel(x, y, color);
                }
            }
        }
    }

    pub fn draw_rectangle_outline(
        &self,
        top_left: (i32, i32),
        bottom_right: (i32, i32),
        color: [u8; 3],
    ) {
        if self.width() == 0 || self.height() == 0 {
            return;
        }

        let min_x = u32_from_i32(top_left.0.clamp(0, i32_from_u32(self.width()) - 1));
        let min_y = u32_from_i32(top_left.1.clamp(0, i32_from_u32(self.height()) - 1));
        let max_x = u32_from_i32(bottom_right.0.clamp(0, i32_from_u32(self.width()) - 1));
        let max_y = u32_from_i32(bottom_right.1.clamp(0, i32_from_u32(self.height()) - 1));
        if min_x > max_x || min_y > max_y {
            return;
        }

        for x in min_x..=max_x {
            self.set_pixel(x, min_y, color);
            self.set_pixel(x, max_y, color);
        }
        for y in min_y..=max_y {
            self.set_pixel(min_x, y, color);
            self.set_pixel(max_x, y, color);
        }
    }

    pub fn grayscale_mat(&self) -> Result<Mat, opencv::Error> {
        let image = self.image.lock().unwrap();
        let mut gray = Mat::default();
        imgproc::cvt_color(&*image, &mut gray, imgproc::COLOR_RGB2GRAY, 0)?;
        Ok(gray)
    }
}

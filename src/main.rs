use std::env;
use std::error::Error;
use std::path::Path;

use image::{ImageBuffer, Rgb};
use video_rs::Decoder;

type RgbImage = ImageBuffer<Rgb<u8>, Vec<u8>>;

fn main() {
    if let Err(error) = run() {
        eprintln!("error: {error}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), Box<dyn Error>> {
    let video_path = env::args()
        .nth(1)
        .ok_or("usage: video-sentinel-rust <video-path>")?;

    video_rs::init()?;

    let mut decoder = Decoder::new(Path::new(&video_path))?;

    for (frame_index, frame_result) in decoder.decode_iter().enumerate() {
        let (_timestamp, frame) = match frame_result {
            Ok(frame) => frame,
            Err(error) => {
                eprintln!("stopping after frame {frame_index}: {error}");
                break;
            }
        };

        let rgb_image = frame_to_rgb_image(frame)?;
        let first_pixel = rgb_image.get_pixel(0, 0);
        println!(
            "frame {frame_index}: first pixel rgb({}, {}, {})",
            first_pixel[0], first_pixel[1], first_pixel[2]
        );
    }

    Ok(())
}

fn frame_to_rgb_image(frame: video_rs::Frame) -> Result<RgbImage, Box<dyn Error>> {
    let (height, width, channels) = frame.dim();
    if channels != 3 {
        return Err(format!("expected 3 channels, got {channels}").into());
    }

    let data = if let Some(slice) = frame.as_slice_memory_order() {
        slice.to_vec()
    } else {
        frame.iter().copied().collect()
    };

    ImageBuffer::from_raw(width as u32, height as u32, data)
        .ok_or_else(|| "failed to create image buffer from decoded frame data".into())
}

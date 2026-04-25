use std::env;
use std::error::Error;
use std::path::Path;

use image::{ImageBuffer, Rgb};
use video_rs::Decoder;
use rs_math3d::Vec3d;

use video_sentinel::slices::{calculate_slices, Rectangle, BasicParams, find_connected_slices};

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
        .ok_or("usage: video-sentinel <video-path>")?;

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
        let rectangle = Rectangle::new(Vec3d::new(0.0, 0.0, 0.0), Vec3d::new(1.0, 1.0, 1.0));
        let mut slices = calculate_slices(rgb_image.clone(), rectangle, BasicParams::new(true, 15));
        let connected_slices = find_connected_slices(&mut slices);
        println!("frame {frame_index}: found {} connected slices", connected_slices.len());
        // draw bounding boxes around connected slices and save the image for debugging
        let mut output_image = rgb_image.clone();
        for slice in connected_slices {
            let bounding_box = slice.get_bounding_box();
            let (x1, y1) = (bounding_box.get_top_left().x as u32, bounding_box.get_top_left().y as u32);
            let (x2, y2) = (bounding_box.get_bottom_right().x as u32, bounding_box.get_bottom_right().y as u32);
            for x in x1..=x2 {
                output_image.put_pixel(x, y1, Rgb([0, 255, 0]));
                output_image.put_pixel(x, y2, Rgb([0, 255, 0]));
            }
            for y in y1..=y2 {
                output_image.put_pixel(x1, y, Rgb([0, 255, 0]));
                output_image.put_pixel(x2, y, Rgb([0, 255, 0]));
            }
        }

        // stream the output image to the output video file
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

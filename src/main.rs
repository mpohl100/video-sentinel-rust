use std::error::Error;
use std::path::PathBuf;

use image::{ImageBuffer, Rgb};
use rs_math3d::Vec3d;
use video_rs::{Decoder, Encoder, Frame};

use video_sentinel::slices::{BasicParams, Rectangle, WrappedRgbImage, calculate_slices, find_connected_slices};

type RgbImage = ImageBuffer<Rgb<u8>, Vec<u8>>;

struct CliArgs {
    input_video: PathBuf,
    output_video: PathBuf,
}

fn main() {
    if let Err(error) = run() {
        eprintln!("error: {error}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), Box<dyn Error>> {
    let args = parse_args()?;

    video_rs::init()?;

    let mut decoder = Decoder::new(args.input_video.as_path())?;
    let (width, height) = decoder.size_out();
    let frame_rate = decoder.frame_rate();
    let settings =
        video_rs::encode::Settings::preset_h264_yuv420p(width as usize, height as usize, false);
    let mut encoder = Encoder::new(args.output_video.as_path(), settings)?;

    for (frame_index, frame_result) in decoder.decode_iter().enumerate() {
        let (timestamp, frame) = match frame_result {
            Ok(frame) => frame,
            Err(error) => {
                eprintln!("stopping after frame {frame_index}: {error}");
                break;
            }
        };

        println!("processing frame {frame_index} at timestamp {timestamp} ms");
        let rgb_image = frame_to_rgb_image(frame)?;
        let wrapped_rgb_image = WrappedRgbImage::new(rgb_image.clone());
        let rectangle = Rectangle::new(
            Vec3d::new(0.0, 0.0, 0.0),
            Vec3d::new(width as f64, height as f64, 0.0),
        );
        let mut slices = calculate_slices(wrapped_rgb_image.clone(), rectangle, BasicParams::new(true, 15));
        println!("frame {frame_index}: calculated slices");
        let connected_slices = find_connected_slices(&mut slices);
        println!(
            "frame {frame_index}: found {} connected slices",
            connected_slices.len()
        );
        // draw bounding boxes around connected slices and save the image for debugging
        let mut output_image = rgb_image.clone();
        let mut slice_counter = 0;
        for slice in connected_slices {
            let bounding_box = slice.get_bounding_box();
            println!(
                "Bounding box: top left ({}, {}), bottom right ({}, {}), area {}",
                bounding_box.get_top_left().x,
                bounding_box.get_top_left().y,
                bounding_box.get_bottom_right().x,
                bounding_box.get_bottom_right().y,
                bounding_box.get_area()
            );
            if bounding_box.get_area() > 10.0 {
                slice_counter += 1;
                let (x1, y1) = (
                    bounding_box.get_top_left().x as u32,
                    bounding_box.get_top_left().y as u32,
                );
                let (x2, y2) = (
                    bounding_box.get_bottom_right().x as u32,
                    bounding_box.get_bottom_right().y as u32,
                );
                for x in x1..=x2 {
                    output_image.put_pixel(x, y1, Rgb([0, 255, 0]));
                    output_image.put_pixel(x, y2, Rgb([0, 255, 0]));
                }
                for y in y1..=y2 {
                    output_image.put_pixel(x1, y, Rgb([0, 255, 0]));
                    output_image.put_pixel(x2, y, Rgb([0, 255, 0]));
                }
            }
        }
        println!(
            "frame {frame_index}: drew {} connected slices",
            slice_counter
        );

        let output_frame = rgb_image_to_frame(output_image)?;
        encoder.encode(&output_frame, timestamp)?;
    }

    encoder.finish()?;

    println!(
        "wrote annotated output to {} at {frame_rate:.3} fps",
        args.output_video.display()
    );

    Ok(())
}

fn parse_args() -> Result<CliArgs, Box<dyn Error>> {
    let mut input_video = None;
    let mut output_video = None;
    let mut args = std::env::args().skip(1);

    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--input-video" => {
                let value = args
                    .next()
                    .ok_or("missing value for --input-video; usage: --input-video <path> --output-video <path>")?;
                if input_video.replace(PathBuf::from(value)).is_some() {
                    return Err("--input-video specified more than once".into());
                }
            }
            "--output-video" => {
                let value = args
                    .next()
                    .ok_or("missing value for --output-video; usage: --input-video <path> --output-video <path>")?;
                if output_video.replace(PathBuf::from(value)).is_some() {
                    return Err("--output-video specified more than once".into());
                }
            }
            "--help" | "-h" => {
                return Err(
                    "usage: video-sentinel-exe --input-video <path> --output-video <path>".into(),
                );
            }
            _ => {
                return Err(format!(
                    "unrecognized argument `{arg}`; usage: --input-video <path> --output-video <path>"
                )
                .into());
            }
        }
    }

    Ok(CliArgs {
        input_video: input_video.ok_or(
            "missing required --input-video; usage: --input-video <path> --output-video <path>",
        )?,
        output_video: output_video.ok_or(
            "missing required --output-video; usage: --input-video <path> --output-video <path>",
        )?,
    })
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

fn rgb_image_to_frame(image: RgbImage) -> Result<Frame, Box<dyn Error>> {
    let (width, height) = image.dimensions();
    Frame::from_shape_vec((height as usize, width as usize, 3), image.into_raw())
        .map_err(|error| format!("failed to create output frame: {error}").into())
}

use std::error::Error;
use std::path::PathBuf;

use clap::{Parser, Subcommand};
use image::{ImageBuffer, Rgb};
use rs_math3d::Vec3d;
use video_rs::{Decoder, Encoder, Frame};

use video_sentinel::service::{
    BasicParamsInput, CreateEyeSessionResult, CreateObjectSessionResult,
    CreateOrdinarySessionResult, EyeParamsInput, GetRectanglesResult, ObjectDetectionParamsInput,
    Service, TileParamsInput, TraceParamsInput,
};
use video_sentinel::slices::{Color, Rectangle, WrappedRgbImage};

type RgbImage = ImageBuffer<Rgb<u8>, Vec<u8>>;

#[derive(Parser)]
#[command(name = "video-sentinel-exe")]
struct CliArgs {
    #[arg(long)]
    input_video: PathBuf,

    #[arg(long)]
    output_video: PathBuf,

    #[arg(long, default_value = "video-session")]
    session_id: String,

    #[arg(long, default_value_t = true)]
    do_grayscale: bool,

    #[arg(long, default_value_t = 15)]
    gradient_threshold: u8,

    #[command(subcommand)]
    session: SessionArgs,
}

#[derive(Subcommand)]
enum SessionArgs {
    Ordinary,
    Eye(TrackingParamsArgs),
    Object(ObjectDetectionArgs),
}

#[derive(clap::Args, Clone)]
struct TrackingParamsArgs {
    #[arg(long, default_value_t = 0.1)]
    tile_x: f64,

    #[arg(long, default_value_t = 0.1)]
    tile_y: f64,

    #[arg(long, default_value_t = 10.0)]
    bucket_delta: f64,

    #[arg(long, default_value_t = 8)]
    num_skeleton: usize,

    #[arg(long, default_value_t = 15.0)]
    close_slice_threshold: f64,

    #[arg(long, default_value_t = 0.8)]
    target_similarity: f64,
}

#[derive(clap::Args, Clone)]
struct ObjectDetectionArgs {
    #[command(flatten)]
    tracking: TrackingParamsArgs,

    #[arg(long = "object-file", required = true)]
    object_files: Vec<PathBuf>,

    #[arg(long = "object-rectangle", required = true, value_name = "x1,y1,x2,y2")]
    object_rectangles: Vec<String>,
}

fn main() {
    if let Err(error) = run() {
        eprintln!("error: {error}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), Box<dyn Error>> {
    let args = CliArgs::parse();

    video_rs::init()?;

    let mut decoder = Decoder::new(args.input_video.as_path())?;
    let (width, height) = decoder.size_out();
    let frame_rate = decoder.frame_rate();
    let settings =
        video_rs::encode::Settings::preset_h264_yuv420p(width as usize, height as usize, false);
    let mut encoder = Encoder::new(args.output_video.as_path(), settings)?;

    let mut service = Service::new();
    configure_session(&mut service, &args)?;

    let mut previous_image: Option<WrappedRgbImage> = None;

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

        let previous_for_service = match &args.session {
            SessionArgs::Eye(_) => Some(
                previous_image
                    .clone()
                    .unwrap_or_else(|| wrapped_rgb_image.clone()),
            ),
            _ => None,
        };

        let enriched_mosaics = match service.get_rectangles(
            args.session_id.clone(),
            wrapped_rgb_image.clone(),
            previous_for_service,
        ) {
            GetRectanglesResult::Success(mosaics) => mosaics,
            GetRectanglesResult::SessionNotFound => {
                return Err(format!("session {} not found", args.session_id).into());
            }
            GetRectanglesResult::PreviousImageRequiredForEyeSession => {
                return Err("eye session requires a previous frame".into());
            }
        };

        let mut output_image = rgb_image;
        for mosaic in enriched_mosaics {
            draw_rectangle(&mut output_image, &mosaic.bounding_box, &mosaic.color);
        }

        let output_frame = rgb_image_to_frame(output_image)?;
        encoder.encode(&output_frame, timestamp)?;

        previous_image = Some(wrapped_rgb_image);
    }

    encoder.finish()?;

    println!(
        "wrote annotated output to {} at {frame_rate:.3} fps",
        args.output_video.display()
    );

    Ok(())
}

fn configure_session(service: &mut Service, args: &CliArgs) -> Result<(), Box<dyn Error>> {
    let basic_params = BasicParamsInput {
        do_grayscale: args.do_grayscale,
        gradient_threshold: args.gradient_threshold,
    };

    match &args.session {
        SessionArgs::Ordinary => {
            match service.create_ordinary_session(args.session_id.clone(), basic_params) {
                CreateOrdinarySessionResult::Success => Ok(()),
                CreateOrdinarySessionResult::SessionAlreadyExists => {
                    Err(format!("session {} already exists", args.session_id).into())
                }
            }
        }
        SessionArgs::Eye(tracking) => {
            let eye_params = to_eye_params_input(tracking);
            match service.create_eye_session(args.session_id.clone(), basic_params, eye_params) {
                CreateEyeSessionResult::Success => Ok(()),
                CreateEyeSessionResult::SessionAlreadyExists => {
                    Err(format!("session {} already exists", args.session_id).into())
                }
            }
        }
        SessionArgs::Object(object_args) => {
            if object_args.object_files.len() != object_args.object_rectangles.len() {
                return Err(format!(
                    "--object-file count ({}) must match --object-rectangle count ({})",
                    object_args.object_files.len(),
                    object_args.object_rectangles.len()
                )
                .into());
            }

            let object_params = to_object_params_input(&object_args.tracking);
            match service.create_object_session(
                args.session_id.clone(),
                basic_params,
                object_params,
            ) {
                CreateObjectSessionResult::Success => {}
                CreateObjectSessionResult::SessionAlreadyExists => {
                    return Err(format!("session {} already exists", args.session_id).into());
                }
            }

            for (index, (file, rectangle)) in object_args
                .object_files
                .iter()
                .zip(object_args.object_rectangles.iter())
                .enumerate()
            {
                let image = image::open(file)?.to_rgb8();
                let wrapped_image = WrappedRgbImage::new(image);
                let surrounding_rectangle = parse_rectangle(rectangle)?;
                let object_id = file
                    .file_stem()
                    .and_then(|stem| stem.to_str())
                    .map(ToOwned::to_owned)
                    .unwrap_or_else(|| format!("object-{index}"));

                match service.add_object_to_be_detected_as_image(
                    args.session_id.clone(),
                    object_id,
                    wrapped_image,
                    surrounding_rectangle,
                ) {
                    video_sentinel::service::AddObjectToBeDetectedResult::Success => {}
                    video_sentinel::service::AddObjectToBeDetectedResult::SessionNotFound => {
                        return Err(format!("session {} not found", args.session_id).into())
                    }
                    video_sentinel::service::AddObjectToBeDetectedResult::SessionTypeDoesNotSupportAddingObjectToBeDetected => {
                        return Err("session type does not support object configuration".into())
                    }
                }
            }

            Ok(())
        }
    }
}

fn to_eye_params_input(args: &TrackingParamsArgs) -> EyeParamsInput {
    EyeParamsInput {
        tile_params: TileParamsInput {
            tile_x: args.tile_x,
            tile_y: args.tile_y,
        },
        bucket_delta: args.bucket_delta,
        trace_params: TraceParamsInput {
            num_skeleton: args.num_skeleton,
            close_slice_threshold: args.close_slice_threshold,
        },
        target_similarity: args.target_similarity,
    }
}

fn to_object_params_input(args: &TrackingParamsArgs) -> ObjectDetectionParamsInput {
    ObjectDetectionParamsInput {
        tile_params: TileParamsInput {
            tile_x: args.tile_x,
            tile_y: args.tile_y,
        },
        bucket_delta: args.bucket_delta,
        trace_params: TraceParamsInput {
            num_skeleton: args.num_skeleton,
            close_slice_threshold: args.close_slice_threshold,
        },
        target_similarity: args.target_similarity,
    }
}

fn parse_rectangle(input: &str) -> Result<Rectangle, Box<dyn Error>> {
    let coords = input
        .split(',')
        .map(str::trim)
        .map(str::parse::<f64>)
        .collect::<Result<Vec<_>, _>>()?;

    if coords.len() != 4 {
        return Err(format!("invalid rectangle `{input}`; expected x1,y1,x2,y2").into());
    }

    let (x1, y1, x2, y2) = (coords[0], coords[1], coords[2], coords[3]);
    if x2 < x1 || y2 < y1 {
        return Err(format!("invalid rectangle `{input}`; expected x2>=x1 and y2>=y1").into());
    }

    Ok(Rectangle::new(
        Vec3d::new(x1, y1, 0.0),
        Vec3d::new(x2, y2, 0.0),
    ))
}

fn draw_rectangle(image: &mut RgbImage, rectangle: &Rectangle, color: &Color) {
    let width = image.width() as i32;
    let height = image.height() as i32;
    if width == 0 || height == 0 {
        return;
    }

    let x1 = rectangle.get_top_left().x.floor() as i32;
    let y1 = rectangle.get_top_left().y.floor() as i32;
    let x2 = rectangle.get_bottom_right().x.ceil() as i32;
    let y2 = rectangle.get_bottom_right().y.ceil() as i32;

    let x1 = x1.clamp(0, width - 1);
    let y1 = y1.clamp(0, height - 1);
    let x2 = x2.clamp(0, width - 1);
    let y2 = y2.clamp(0, height - 1);

    if x1 > x2 || y1 > y2 {
        return;
    }

    let pixel = match color {
        Color::Red => Rgb([255, 0, 0]),
        Color::Green => Rgb([0, 255, 0]),
        Color::Blue => Rgb([0, 0, 255]),
    };

    for x in x1..=x2 {
        image.put_pixel(x as u32, y1 as u32, pixel);
        image.put_pixel(x as u32, y2 as u32, pixel);
    }
    for y in y1..=y2 {
        image.put_pixel(x1 as u32, y as u32, pixel);
        image.put_pixel(x2 as u32, y as u32, pixel);
    }
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

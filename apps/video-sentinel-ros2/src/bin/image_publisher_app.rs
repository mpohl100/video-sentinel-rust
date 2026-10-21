use std::error::Error;
use std::fs;
use std::path::PathBuf;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use clap::Parser;
use ros2_client::{
    Context, DEFAULT_PUBLISHER_QOS, MessageTypeName, Name, NodeName, NodeOptions, Publisher,
    builtin_interfaces::Time,
};
use tokio::time::sleep;
use vs_ros2::ros2_node::{CompressedSensorImage, Header};

const IMAGE_TYPE_PACKAGE: &str = "sensor_msgs";
const IMAGE_TYPE_NAME: &str = "CompressedImage";

#[derive(Debug, Parser)]
#[command(name = "image-publisher-app")]
struct CliArgs {
    #[arg(long)]
    topic: String,
    #[arg(long)]
    image: PathBuf,
    #[arg(long, default_value = "camera")]
    frame_id: String,
    #[arg(long, default_value_t = 2_000)]
    startup_delay_ms: u64,
    #[arg(long, default_value_t = 1_000)]
    post_publish_delay_ms: u64,
    #[arg(long, default_value_t = 5)]
    publish_count: u32,
    #[arg(long, default_value_t = 500)]
    publish_interval_ms: u64,
}

#[tokio::main(flavor = "multi_thread", worker_threads = 2)]
async fn main() {
    if let Err(error) = run_from_cli().await {
        eprintln!("error: {error}");
        std::process::exit(1);
    }
}

async fn run_from_cli() -> Result<(), Box<dyn Error>> {
    let args = CliArgs::parse();
    let image_bytes = fs::read(&args.image)?;

    let context = Context::new()?;
    let unique = unique_suffix();
    let mut node = context.new_node(
        NodeName::new("/", &format!("image_publisher_app_{unique}"))?,
        NodeOptions::new(),
    )?;

    let topic_name = Name::parse(&args.topic)?;
    let topic = node.create_topic(
        &topic_name,
        MessageTypeName::new(IMAGE_TYPE_PACKAGE, IMAGE_TYPE_NAME),
        &DEFAULT_PUBLISHER_QOS,
    )?;
    let publisher: Publisher<CompressedSensorImage> = node.create_publisher(&topic, None)?;

    tokio::spawn(node.spinner()?.spin());

    println!("waiting {} ms before publishing", args.startup_delay_ms);
    sleep(Duration::from_millis(args.startup_delay_ms)).await;

    for publish_index in 0..args.publish_count {
        let message = CompressedSensorImage::new_jpeg(
            Header {
                stamp: Time::now(),
                frame_id: args.frame_id.clone(),
            },
            image_bytes.clone(),
        );
        publish_once(&publisher, message).await?;
        if publish_index + 1 < args.publish_count {
            sleep(Duration::from_millis(args.publish_interval_ms)).await;
        }
    }

    println!(
        "published {} compressed image(s) to {} and waiting {} ms for delivery",
        args.publish_count, args.topic, args.post_publish_delay_ms
    );
    sleep(Duration::from_millis(args.post_publish_delay_ms)).await;

    Ok(())
}

async fn publish_once(
    publisher: &Publisher<CompressedSensorImage>,
    message: CompressedSensorImage,
) -> Result<(), Box<dyn Error>> {
    for _attempt in 0..30 {
        match publisher.async_publish(message.clone()).await {
            Ok(()) => return Ok(()),
            Err(error) if error.to_string().contains("WouldBlock") => {
                sleep(Duration::from_millis(100)).await;
            }
            Err(error) => return Err(Box::<dyn Error>::from(error.to_string())),
        }
    }

    Err("publisher never became ready".into())
}

fn unique_suffix() -> String {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system time should be after epoch")
        .as_nanos();
    format!("u{}_{}", std::process::id(), nanos)
}

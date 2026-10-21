#![warn(clippy::pedantic)]
#![allow(
    clippy::missing_errors_doc,
    clippy::missing_panics_doc,
    clippy::too_many_lines,
    clippy::unused_async_trait_impl
)]

use clap::Parser;
use std::path::PathBuf;

#[derive(Debug, Parser)]
#[command(name = "video-sentinel-ros2-node")]
pub struct CliArgs {
    #[arg(long)]
    pub config: PathBuf,
}

#[cfg(test)]
mod tests {
    use crate::ros2_node::funcs::is_message_stale;
    use crate::ros2_node::funcs::wrapped_rgb_image_from_sensor_image;

    use crate::ros2_node::*;

    use crate::slices::WrappedRgbImage;

    use opencv::prelude::{MatTraitConst, VectorToVec};
    use ros2_client::builtin_interfaces::Time;

    use std::fs;
    use std::path::Path;

    #[test]
    fn default_topics_follow_session_kind() {
        assert_eq!(
            default_output_topic("ordinary", RosSessionType::Ordinary),
            "/video_sentinel/sessions/ordinary/rectangles"
        );
        assert_eq!(
            default_output_topic("eye", RosSessionType::Eye),
            "/video_sentinel/sessions/eye/rectangles"
        );
        assert_eq!(
            default_output_topic("object", RosSessionType::Object),
            "/video_sentinel/sessions/object/labelled_rectangles"
        );
    }

    #[test]
    fn jpeg_sensor_image_converts_to_wrapped_rgb_image() {
        let source = WrappedRgbImage::new_with_color(2, 1, [10, 20, 30]);
        let rgb = source
            .get_mat()
            .try_clone()
            .expect("wrapped image should clone for JPEG encoding");
        let mut bgr = opencv::prelude::Mat::default();
        opencv::imgproc::cvt_color(&rgb, &mut bgr, opencv::imgproc::COLOR_RGB2BGR, 0)
            .expect("RGB image should convert to BGR");
        let mut encoded = opencv::core::Vector::<u8>::new();
        opencv::imgcodecs::imencode(
            ".jpg",
            &bgr,
            &mut encoded,
            &opencv::core::Vector::<i32>::new(),
        )
        .expect("image should encode as JPEG");

        let image = CompressedSensorImage::new_jpeg(
            Header {
                stamp: Time::from_nanos(42),
                frame_id: "camera".to_string(),
            },
            encoded.to_vec(),
        );

        let wrapped = wrapped_rgb_image_from_sensor_image(&image)
            .expect("JPEG image should convert into WrappedRgbImage");
        assert_eq!(wrapped.width(), 2);
        assert_eq!(wrapped.height(), 1);
        let pixel = wrapped.get_pixel(0, 0);
        assert!((i16::from(pixel[0]) - 10).abs() <= 5);
        assert!((i16::from(pixel[1]) - 20).abs() <= 5);
        assert!((i16::from(pixel[2]) - 30).abs() <= 5);
    }

    #[test]
    fn stale_message_check_uses_small_threshold() {
        let now = Time::now().to_nanos();
        assert!(!is_message_stale(Time::from_nanos(now), 250));
        assert!(!is_message_stale(Time::from_nanos(now - 249_000_000), 250));
        assert!(is_message_stale(Time::from_nanos(now - 251_000_000), 250));
        assert!(!is_message_stale(Time::from_nanos(now + 10_000_000), 250));
    }

    #[test]
    fn config_load_resolves_reference_object_paths_relative_to_yaml() {
        let temp_dir = tempfile::tempdir().expect("tempdir should be created");
        let image_path = temp_dir.path().join("reference.jpg");
        fs::write(&image_path, b"fake").expect("reference file should be written");
        let config_path = temp_dir.path().join("session.yaml");
        fs::write(
            &config_path,
            r"
topics:
  image: /camera/image
session:
  type: object
  id: object-session
  reference_objects:
    - source: image_path
      object_id: ref
      image_path: reference.jpg
      surrounding_rectangle:
        top_left: { x: 0.0, y: 0.0 }
        bottom_right: { x: 10.0, y: 10.0 }
",
        )
        .expect("config should be written");

        let config = Ros2NodeConfig::load(&config_path).expect("config should load");
        match config.session() {
            SessionConfig::Object {
                reference_objects, ..
            } => {
                assert_eq!(reference_objects.len(), 1);
                match &reference_objects[0] {
                    ReferenceObjectYaml::ImagePath {
                        image_path: path, ..
                    } => {
                        assert_eq!(path, &image_path);
                    }
                    ReferenceObjectYaml::AsciiArt { .. } => {
                        panic!("expected image-path reference object");
                    }
                }
            }
            _ => panic!("expected object session"),
        }
    }

    #[test]
    fn config_load_supports_ascii_art_reference_objects() {
        let temp_dir = tempfile::tempdir().expect("tempdir should be created");
        let config_path = temp_dir.path().join("session.yaml");
        fs::write(
            &config_path,
            r"
topics:
  image: /camera/image
session:
  type: object
  id: object-session
  reference_objects:
    - source: ascii_art
      object_id: ref
      ascii_art: |
        .##.
        ####
        .##.
",
        )
        .expect("config should be written");

        let config = Ros2NodeConfig::load(&config_path).expect("config should load");
        match config.session() {
            SessionConfig::Object {
                reference_objects, ..
            } => {
                assert_eq!(reference_objects.len(), 1);
                match &reference_objects[0] {
                    ReferenceObjectYaml::AsciiArt {
                        object_id,
                        ascii_art,
                    } => {
                        assert_eq!(object_id, "ref");
                        assert_eq!(ascii_art, ".##.\n####\n.##.\n");
                    }
                    ReferenceObjectYaml::ImagePath { .. } => {
                        panic!("expected ASCII-art reference object");
                    }
                }
            }
            _ => panic!("expected object session"),
        }
    }

    #[test]
    fn example_config_files_load() {
        let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
        for config_name in [
            "config/ros2-session.example.yaml",
            "config/ros2-session-eye.example.yaml",
            "config/ros2-session-object.example.yaml",
        ] {
            let config_path = manifest_dir.join(config_name);
            Ros2NodeConfig::load(&config_path)
                .unwrap_or_else(|error| panic!("failed to load {config_name}: {error}"));
        }
    }
}

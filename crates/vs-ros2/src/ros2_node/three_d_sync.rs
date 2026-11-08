#![warn(clippy::pedantic)]
#![allow(
    clippy::missing_errors_doc,
    clippy::missing_panics_doc,
    clippy::too_many_lines,
    clippy::unused_async_trait_impl
)]

use std::collections::VecDeque;

use super::{CameraInfo, Image};

pub(super) struct MatchedFrame {
    pub rgb: Image,
    pub depth: Image,
    pub rgb_camera_info: CameraInfo,
    pub depth_camera_info: CameraInfo,
}

#[derive(Default)]
pub(super) struct ThreeDFrameSynchronizer {
    rgb: VecDeque<Image>,
    depth: VecDeque<Image>,
    rgb_info: VecDeque<CameraInfo>,
    depth_info: VecDeque<CameraInfo>,
    limit: usize,
}

impl ThreeDFrameSynchronizer {
    pub(super) fn new(limit: usize) -> Self {
        Self {
            limit: limit.max(1),
            ..Self::default()
        }
    }

    pub(super) fn push_rgb(&mut self, message: Image) {
        push_bounded(&mut self.rgb, message, self.limit);
    }

    pub(super) fn push_depth(&mut self, message: Image) {
        push_bounded(&mut self.depth, message, self.limit);
    }

    pub(super) fn push_rgb_info(&mut self, message: CameraInfo) {
        push_bounded(&mut self.rgb_info, message, self.limit);
    }

    pub(super) fn push_depth_info(&mut self, message: CameraInfo) {
        push_bounded(&mut self.depth_info, message, self.limit);
    }

    pub(super) fn discard_image_messages(&mut self) {
        self.rgb.clear();
        self.depth.clear();
    }

    pub(super) fn pop_nearest(&mut self, max_difference_ns: i64) -> Option<MatchedFrame> {
        let max_difference_ns = u128::try_from(max_difference_ns).ok()?;
        let mut best: Option<(u128, usize, usize, usize, usize)> = None;
        let closest_rgb_info = self
            .rgb
            .iter()
            .map(|rgb| {
                self.rgb_info
                    .iter()
                    .enumerate()
                    .filter(|(_, info)| info.header.frame_id == rgb.header.frame_id)
                    .map(|(index, info)| {
                        (
                            index,
                            timestamp_difference(
                                rgb.header.stamp.to_nanos(),
                                info.header.stamp.to_nanos(),
                            ),
                        )
                    })
                    .min_by_key(|(_, difference)| *difference)
                    .map(|(index, _)| index)
            })
            .collect::<Vec<_>>();
        let closest_depth_info = self
            .depth
            .iter()
            .map(|depth| {
                self.depth_info
                    .iter()
                    .enumerate()
                    .filter(|(_, info)| info.header.frame_id == depth.header.frame_id)
                    .map(|(index, info)| {
                        (
                            index,
                            timestamp_difference(
                                depth.header.stamp.to_nanos(),
                                info.header.stamp.to_nanos(),
                            ),
                        )
                    })
                    .min_by_key(|(_, difference)| *difference)
                    .map(|(index, _)| index)
            })
            .collect::<Vec<_>>();
        for (rgb_index, rgb) in self.rgb.iter().enumerate() {
            for (depth_index, depth) in self.depth.iter().enumerate() {
                let image_difference = timestamp_difference(
                    rgb.header.stamp.to_nanos(),
                    depth.header.stamp.to_nanos(),
                );
                if image_difference > max_difference_ns {
                    continue;
                }
                let Some(rgb_info_index) = closest_rgb_info[rgb_index] else {
                    continue;
                };
                let Some(depth_info_index) = closest_depth_info[depth_index] else {
                    continue;
                };
                let score = image_difference;
                if best.is_none_or(|current| score < current.0) {
                    best = Some((
                        score,
                        rgb_index,
                        depth_index,
                        rgb_info_index,
                        depth_info_index,
                    ));
                }
            }
        }
        let (_, rgb_index, depth_index, rgb_info_index, depth_info_index) = best?;
        Some(MatchedFrame {
            rgb: self.rgb.remove(rgb_index)?,
            depth: self.depth.remove(depth_index)?,
            rgb_camera_info: self.rgb_info[rgb_info_index].clone(),
            depth_camera_info: self.depth_info[depth_info_index].clone(),
        })
    }
}

fn push_bounded<T>(queue: &mut VecDeque<T>, value: T, limit: usize) {
    while queue.len() >= limit.max(1) {
        queue.pop_front();
    }
    queue.push_back(value);
}

fn timestamp_difference(lhs: i64, rhs: i64) -> u128 {
    u128::from(lhs.abs_diff(rhs))
}

#[cfg(test)]
mod tests {
    use super::*;
    use ros2_client::builtin_interfaces::Time;

    fn image(stamp: i32, frame: &str) -> Image {
        Image {
            header: super::super::Header {
                stamp: Time::from_nanos(i64::from(stamp) * 1_000_000_000),
                frame_id: frame.to_owned(),
            },
            height: 1,
            width: 1,
            encoding: "rgb8".to_owned(),
            is_bigendian: 0,
            step: 3,
            data: vec![0; 3],
        }
    }

    fn info(stamp: i32, frame: &str) -> CameraInfo {
        CameraInfo {
            header: super::super::Header {
                stamp: Time::from_nanos(i64::from(stamp) * 1_000_000_000),
                frame_id: frame.to_owned(),
            },
            height: 1,
            width: 1,
            distortion_model: "plumb_bob".to_owned(),
            d: vec![],
            k: [1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0],
            r: [0.0; 9],
            p: [0.0; 12],
            binning_x: 0,
            binning_y: 0,
            roi: super::super::camera_info::RegionOfInterest {
                x_offset: 0,
                y_offset: 0,
                height: 0,
                width: 0,
                do_rectify: false,
            },
        }
    }

    #[test]
    fn pairs_nearest_timestamps_and_matching_camera_frame_ids() {
        let mut sync = ThreeDFrameSynchronizer::new(4);
        sync.push_rgb(image(10, "rgb_optical"));
        sync.push_depth(image(11, "depth_optical"));
        sync.push_rgb_info(info(10, "wrong_frame"));
        sync.push_rgb_info(info(10, "rgb_optical"));
        sync.push_depth_info(info(11, "depth_optical"));
        let pair = sync
            .pop_nearest(2_000_000_000)
            .expect("matching frame ids should pair");
        assert_eq!(pair.rgb_camera_info.header.frame_id, "rgb_optical");
        assert_eq!(pair.depth_camera_info.header.frame_id, "depth_optical");
    }

    #[test]
    fn retains_only_a_bounded_number_of_unmatched_messages() {
        let mut sync = ThreeDFrameSynchronizer::new(2);
        sync.push_rgb(image(1, "rgb"));
        sync.push_rgb(image(2, "rgb"));
        sync.push_rgb(image(3, "rgb"));
        sync.push_depth(image(3, "depth"));
        sync.push_rgb_info(info(3, "rgb"));
        sync.push_depth_info(info(3, "depth"));
        let pair = sync.pop_nearest(0).expect("latest RGB image is retained");
        assert_eq!(pair.rgb.header.stamp.to_nanos(), 3_000_000_000);
    }

    #[test]
    fn does_not_pair_frames_outside_configured_time_difference() {
        let mut sync = ThreeDFrameSynchronizer::new(4);
        sync.push_rgb(image(1, "rgb"));
        sync.push_depth(image(2, "depth"));
        sync.push_rgb_info(info(1, "rgb"));
        sync.push_depth_info(info(2, "depth"));
        assert!(sync.pop_nearest(999_999_999).is_none());
    }

    #[test]
    fn static_or_low_rate_camera_info_is_not_limited_by_frame_pair_tolerance() {
        let mut sync = ThreeDFrameSynchronizer::new(4);
        sync.push_rgb(image(100, "rgb"));
        sync.push_depth(image(100, "depth"));
        sync.push_rgb_info(info(0, "rgb"));
        sync.push_depth_info(info(0, "depth"));

        assert!(sync.pop_nearest(0).is_some());
    }
}

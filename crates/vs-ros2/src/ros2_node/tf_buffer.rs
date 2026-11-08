#![warn(clippy::pedantic)]
#![allow(
    clippy::missing_errors_doc,
    clippy::missing_panics_doc,
    clippy::too_many_lines,
    clippy::unused_async_trait_impl
)]

use std::collections::{BTreeMap, VecDeque};

use num_traits::ToPrimitive;
use ros2_client::builtin_interfaces::Time;

use super::TransformStamped;
use vs_service::service::RigidTransform;

#[derive(Clone, Debug)]
struct StampedTransform {
    stamp: i64,
    parent: String,
    child: String,
    transform: RigidTransform,
}

#[derive(Default)]
pub(super) struct TfBuffer {
    static_transforms: Vec<StampedTransform>,
    dynamic_transforms: BTreeMap<String, VecDeque<StampedTransform>>,
}

impl TfBuffer {
    pub(super) fn insert(
        &mut self,
        message: &TransformStamped,
        is_static: bool,
        history_limit: usize,
    ) {
        let transform = StampedTransform {
            stamp: message.header.stamp.to_nanos(),
            parent: message.header.frame_id.clone(),
            child: message.child_frame_id.clone(),
            transform: RigidTransform {
                translation: [
                    message.transform.translation.x,
                    message.transform.translation.y,
                    message.transform.translation.z,
                ],
                rotation_xyzw: [
                    message.transform.rotation.x,
                    message.transform.rotation.y,
                    message.transform.rotation.z,
                    message.transform.rotation.w,
                ],
            },
        };
        if transform.parent.is_empty()
            || transform.child.is_empty()
            || transform.parent == transform.child
            || !is_valid(transform.transform)
        {
            return;
        }

        if is_static {
            self.static_transforms
                .retain(|existing| existing.child != transform.child);
            self.static_transforms.push(transform);
            while self.static_transforms.len() > history_limit.max(1) {
                self.static_transforms.remove(0);
            }
            return;
        }

        let current_child = transform.child.clone();
        let history = self
            .dynamic_transforms
            .entry(transform.child.clone())
            .or_default();
        if let Some(existing) = history
            .iter_mut()
            .find(|item| item.stamp == transform.stamp)
        {
            *existing = transform;
        } else {
            let position = history
                .iter()
                .position(|item| item.stamp > transform.stamp)
                .unwrap_or(history.len());
            history.insert(position, transform);
        }
        while history.len() > history_limit.max(1) {
            history.pop_front();
        }
        if self.dynamic_transforms.len() > history_limit.max(1)
            && let Some(oldest_child) = self
                .dynamic_transforms
                .keys()
                .find(|child| **child != current_child)
                .cloned()
        {
            self.dynamic_transforms.remove(&oldest_child);
        }
    }

    pub(super) fn lookup(&self, source: &str, target: &str, stamp: Time) -> Option<RigidTransform> {
        if source.is_empty() || target.is_empty() {
            return None;
        }
        if source == target {
            return Some(RigidTransform {
                translation: [0.0; 3],
                rotation_xyzw: [0.0, 0.0, 0.0, 1.0],
            });
        }
        let stamp = stamp.to_nanos();
        let mut edges = self.static_transforms.clone();
        for history in self.dynamic_transforms.values() {
            let index = history.partition_point(|entry| entry.stamp < stamp);
            let sampled = if let Some(exact) =
                history.get(index).filter(|entry| entry.stamp == stamp)
            {
                Some(exact.transform)
            } else {
                match (
                    index.checked_sub(1).and_then(|i| history.get(i)),
                    history.get(index),
                ) {
                    (Some(before), Some(after)) if before.stamp < stamp && stamp < after.stamp => {
                        let amount = (stamp - before.stamp).to_f64()?
                            / (after.stamp - before.stamp).to_f64()?;
                        interpolate(before.transform, after.transform, amount)
                    }
                    _ => None,
                }
            };
            if let Some(transform) = sampled
                && let Some(entry) = history.front()
            {
                let mut selected = entry.clone();
                selected.transform = transform;
                edges.push(selected);
            }
        }

        let mut queue = VecDeque::from([(
            source.to_owned(),
            RigidTransform {
                translation: [0.0; 3],
                rotation_xyzw: [0.0, 0.0, 0.0, 1.0],
            },
        )]);
        let mut visited = vec![source.to_owned()];
        while let Some((frame, frame_from_source)) = queue.pop_front() {
            for edge in &edges {
                let (neighbor, neighbor_from_frame) = if edge.child == frame {
                    (edge.parent.as_str(), edge.transform)
                } else if edge.parent == frame {
                    (edge.child.as_str(), inverse(edge.transform)?)
                } else {
                    continue;
                };
                if visited.iter().any(|seen| seen == neighbor) {
                    continue;
                }
                let neighbor_from_source = compose(neighbor_from_frame, frame_from_source)?;
                if neighbor == target {
                    return Some(neighbor_from_source);
                }
                visited.push(neighbor.to_owned());
                queue.push_back((neighbor.to_owned(), neighbor_from_source));
            }
        }
        None
    }
}

fn is_valid(transform: RigidTransform) -> bool {
    transform.translation.iter().all(|value| value.is_finite())
        && transform
            .rotation_xyzw
            .iter()
            .all(|value| value.is_finite())
        && transform
            .rotation_xyzw
            .iter()
            .map(|value| value * value)
            .sum::<f64>()
            > f64::EPSILON
}

fn normalized_quaternion(quaternion: [f64; 4]) -> Option<[f64; 4]> {
    let norm = quaternion
        .iter()
        .map(|value| value * value)
        .sum::<f64>()
        .sqrt();
    if !norm.is_finite() || norm <= f64::EPSILON {
        return None;
    }
    Some(quaternion.map(|value| value / norm))
}

fn multiply_quaternion(lhs: [f64; 4], rhs: [f64; 4]) -> Option<[f64; 4]> {
    let [x1, y1, z1, w1] = lhs;
    let [x2, y2, z2, w2] = rhs;
    normalized_quaternion([
        w1 * x2 + x1 * w2 + y1 * z2 - z1 * y2,
        w1 * y2 - x1 * z2 + y1 * w2 + z1 * x2,
        w1 * z2 + x1 * y2 - y1 * x2 + z1 * w2,
        w1 * w2 - x1 * x2 - y1 * y2 - z1 * z2,
    ])
}

fn rotate(quaternion: [f64; 4], vector: [f64; 3]) -> Option<[f64; 3]> {
    let [x, y, z, w] = normalized_quaternion(quaternion)?;
    let qv = [x, y, z];
    let cross = [
        qv[1] * vector[2] - qv[2] * vector[1],
        qv[2] * vector[0] - qv[0] * vector[2],
        qv[0] * vector[1] - qv[1] * vector[0],
    ];
    let cross2 = [
        qv[1] * cross[2] - qv[2] * cross[1],
        qv[2] * cross[0] - qv[0] * cross[2],
        qv[0] * cross[1] - qv[1] * cross[0],
    ];
    Some([
        vector[0] + 2.0 * (w * cross[0] + cross2[0]),
        vector[1] + 2.0 * (w * cross[1] + cross2[1]),
        vector[2] + 2.0 * (w * cross[2] + cross2[2]),
    ])
}

fn compose(lhs: RigidTransform, rhs: RigidTransform) -> Option<RigidTransform> {
    let translated = rotate(lhs.rotation_xyzw, rhs.translation)?;
    Some(RigidTransform {
        translation: [
            translated[0] + lhs.translation[0],
            translated[1] + lhs.translation[1],
            translated[2] + lhs.translation[2],
        ],
        rotation_xyzw: multiply_quaternion(lhs.rotation_xyzw, rhs.rotation_xyzw)?,
    })
}

fn inverse(transform: RigidTransform) -> Option<RigidTransform> {
    let [x, y, z, w] = normalized_quaternion(transform.rotation_xyzw)?;
    let rotation_xyzw = [-x, -y, -z, w];
    let rotated = rotate(rotation_xyzw, transform.translation)?;
    Some(RigidTransform {
        translation: [-rotated[0], -rotated[1], -rotated[2]],
        rotation_xyzw,
    })
}

fn interpolate(lhs: RigidTransform, rhs: RigidTransform, amount: f64) -> Option<RigidTransform> {
    let left = normalized_quaternion(lhs.rotation_xyzw)?;
    let mut right = normalized_quaternion(rhs.rotation_xyzw)?;
    let mut dot = left.iter().zip(right).map(|(a, b)| a * b).sum::<f64>();
    if dot < 0.0 {
        right = right.map(|value| -value);
        dot = -dot;
    }
    let rotation_xyzw = if dot > 0.9995 {
        normalized_quaternion(std::array::from_fn(|index| {
            left[index] + amount * (right[index] - left[index])
        }))?
    } else {
        let theta = dot.clamp(-1.0, 1.0).acos();
        let denominator = theta.sin();
        let lhs_weight = ((1.0 - amount) * theta).sin() / denominator;
        let rhs_weight = (amount * theta).sin() / denominator;
        normalized_quaternion(std::array::from_fn(|index| {
            lhs_weight * left[index] + rhs_weight * right[index]
        }))?
    };
    Some(RigidTransform {
        translation: std::array::from_fn(|index| {
            lhs.translation[index] + amount * (rhs.translation[index] - lhs.translation[index])
        }),
        rotation_xyzw,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn transform(parent: &str, child: &str, stamp: i32, translation: [f64; 3]) -> TransformStamped {
        TransformStamped {
            header: super::super::Header {
                stamp: Time::from_nanos(i64::from(stamp) * 1_000_000_000),
                frame_id: parent.to_owned(),
            },
            child_frame_id: child.to_owned(),
            transform: super::super::Transform {
                translation: super::super::Vector3 {
                    x: translation[0],
                    y: translation[1],
                    z: translation[2],
                },
                rotation: super::super::Quaternion {
                    x: 0.0,
                    y: 0.0,
                    z: 0.0,
                    w: 1.0,
                },
            },
        }
    }

    #[test]
    fn resolves_a_multi_edge_tf_path_and_inverse() {
        let mut buffer = TfBuffer::default();
        buffer.insert(&transform("base", "depth", 1, [1.0, 0.0, 0.0]), true, 10);
        buffer.insert(&transform("base", "rgb", 1, [0.0, 2.0, 0.0]), true, 10);
        let depth_to_rgb = buffer
            .lookup("depth", "rgb", Time::from_nanos(1_000_000_000))
            .expect("TF path exists");
        assert_eq!(depth_to_rgb.translation, [1.0, -2.0, 0.0]);
        let rgb_to_depth = buffer
            .lookup("rgb", "depth", Time::from_nanos(1_000_000_000))
            .expect("inverse path exists");
        assert_eq!(rgb_to_depth.translation, [-1.0, 2.0, 0.0]);
    }

    #[test]
    fn interpolates_dynamic_transforms_at_frame_time() {
        let mut buffer = TfBuffer::default();
        buffer.insert(&transform("base", "depth", 1, [0.0, 0.0, 0.0]), false, 10);
        buffer.insert(&transform("base", "depth", 3, [2.0, 0.0, 0.0]), false, 10);
        let resolved = buffer
            .lookup("depth", "base", Time::from_nanos(2_000_000_000))
            .expect("bracketing transforms permit lookup");
        assert_eq!(resolved.translation, [1.0, 0.0, 0.0]);
        assert!(
            buffer
                .lookup("depth", "base", Time::from_nanos(4_000_000_000))
                .is_none()
        );
    }

    #[test]
    fn does_not_assume_identity_for_unrelated_frames() {
        let buffer = TfBuffer::default();
        assert!(
            buffer
                .lookup(
                    "depth_optical",
                    "rgb_optical",
                    Time::from_nanos(1_000_000_000)
                )
                .is_none()
        );
    }
}

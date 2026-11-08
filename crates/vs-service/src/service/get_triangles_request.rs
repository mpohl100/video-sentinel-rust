#![warn(clippy::pedantic)]
#![allow(
    clippy::missing_errors_doc,
    clippy::missing_panics_doc,
    clippy::too_many_lines,
    clippy::unused_async_trait_impl
)]

use super::{
    ProcessThreeDTriangleFrameResult, ThreeDTriangleCalibration, ThreeDTriangleFrame, Triangle,
};
use std::sync::{Arc, Mutex};
use tokio::sync::{mpsc, watch};

type FrameFactory = Box<dyn FnOnce() -> Result<ThreeDTriangleFrame, String> + Send + 'static>;

struct ActiveCalculation {
    id: u64,
    result: watch::Receiver<Option<ProcessThreeDTriangleFrameResult>>,
}

#[derive(Default)]
struct CalculationState {
    next_id: u64,
    active: Option<ActiveCalculation>,
}

struct TriangleJob {
    id: u64,
    calibration: ThreeDTriangleCalibration,
    frame: ThreeDTriangleFrame,
    result: watch::Sender<Option<ProcessThreeDTriangleFrameResult>>,
}

pub(super) struct TriangleCalculation {
    sender: mpsc::Sender<TriangleJob>,
    receiver: Mutex<Option<mpsc::Receiver<TriangleJob>>>,
    state: Arc<tokio::sync::Mutex<CalculationState>>,
}

impl TriangleCalculation {
    pub(super) fn new() -> Self {
        let (sender, receiver) = mpsc::channel(1);
        Self {
            sender,
            receiver: Mutex::new(Some(receiver)),
            state: Arc::new(tokio::sync::Mutex::new(CalculationState::default())),
        }
    }

    fn start_worker(&self) {
        let Some(receiver) = self.receiver.lock().unwrap().take() else {
            return;
        };
        tokio::spawn(run_worker(receiver, Arc::clone(&self.state)));
    }

    async fn start<F>(
        &self,
        calibration: ThreeDTriangleCalibration,
        frame_factory: F,
    ) -> (
        watch::Receiver<Option<ProcessThreeDTriangleFrameResult>>,
        bool,
    )
    where
        F: FnOnce() -> Result<ThreeDTriangleFrame, String> + Send + 'static,
    {
        self.start_worker();
        let (result_receiver, calculation_id) = {
            let mut state = self.state.lock().await;
            if let Some(active) = &state.active {
                (active.result.clone(), None)
            } else {
                let id = state.next_id;
                state.next_id = state.next_id.wrapping_add(1);
                let (result_sender, result_receiver) = watch::channel(None);
                state.active = Some(ActiveCalculation {
                    id,
                    result: result_receiver.clone(),
                });
                (result_receiver, Some((id, result_sender)))
            }
        };

        let started = calculation_id.is_some();
        if let Some((id, result_sender)) = calculation_id {
            tokio::spawn(submit_triangle_job(
                id,
                calibration,
                Box::new(frame_factory),
                self.sender.clone(),
                Arc::clone(&self.state),
                result_sender,
            ));
        }
        (result_receiver, started)
    }
}

pub struct GetTrianglesRequest {
    calculation: Arc<TriangleCalculation>,
    calibration: ThreeDTriangleCalibration,
}

pub struct GetTrianglesCalculation {
    result: watch::Receiver<Option<ProcessThreeDTriangleFrameResult>>,
}

impl GetTrianglesCalculation {
    pub async fn result(self) -> ProcessThreeDTriangleFrameResult {
        wait_for_result(self.result).await
    }
}

impl GetTrianglesRequest {
    pub(super) fn new(
        calculation: Arc<TriangleCalculation>,
        calibration: ThreeDTriangleCalibration,
    ) -> Self {
        Self {
            calculation,
            calibration,
        }
    }

    pub async fn get_triangles_with<F>(self, frame_factory: F) -> ProcessThreeDTriangleFrameResult
    where
        F: FnOnce() -> Result<ThreeDTriangleFrame, String> + Send + 'static,
    {
        let (result, _) = self
            .calculation
            .start(self.calibration, frame_factory)
            .await;
        wait_for_result(result).await
    }

    /// Starts a calculation only if the session is idle.
    ///
    /// If another calculation is active, this drops the frame factory without
    /// converting the frame and returns `None`.
    pub async fn try_start_with<F>(self, frame_factory: F) -> Option<GetTrianglesCalculation>
    where
        F: FnOnce() -> Result<ThreeDTriangleFrame, String> + Send + 'static,
    {
        let (result, started) = self
            .calculation
            .start(self.calibration, frame_factory)
            .await;
        started.then_some(GetTrianglesCalculation { result })
    }
}

async fn submit_triangle_job(
    id: u64,
    calibration: ThreeDTriangleCalibration,
    frame_factory: FrameFactory,
    sender: mpsc::Sender<TriangleJob>,
    state: Arc<tokio::sync::Mutex<CalculationState>>,
    result: watch::Sender<Option<ProcessThreeDTriangleFrameResult>>,
) {
    let Ok(permit) = sender.reserve_owned().await else {
        finish_calculation(
            id,
            ProcessThreeDTriangleFrameResult::InvalidInput(
                "triangle job channel closed".to_owned(),
            ),
            state,
            result,
        )
        .await;
        return;
    };
    let frame = match tokio::task::spawn_blocking(frame_factory).await {
        Ok(Ok(frame)) => frame,
        Ok(Err(error)) => {
            finish_calculation(
                id,
                ProcessThreeDTriangleFrameResult::InvalidInput(error),
                state,
                result,
            )
            .await;
            return;
        }
        Err(error) => {
            finish_calculation(
                id,
                ProcessThreeDTriangleFrameResult::InvalidInput(format!(
                    "frame conversion task failed: {error}"
                )),
                state,
                result,
            )
            .await;
            return;
        }
    };
    permit.send(TriangleJob {
        id,
        calibration,
        frame,
        result,
    });
}

async fn run_worker(
    mut receiver: mpsc::Receiver<TriangleJob>,
    state: Arc<tokio::sync::Mutex<CalculationState>>,
) {
    while let Some(job) = receiver.recv().await {
        let TriangleJob {
            id,
            calibration,
            frame,
            result,
        } = job;
        let calculation_result = tokio::task::spawn_blocking(move || {
            let registered = super::register_depth(
                &frame.depth,
                &calibration.depth_camera,
                &calibration.rgb_camera,
                calibration.depth_to_rgb,
                frame.rgb.width(),
                frame.rgb.height(),
            );
            match registered {
                Ok(_) => {
                    // TODO: Extract mesh triangles from the RGB image and registered depth.
                    ProcessThreeDTriangleFrameResult::Success(Vec::<Triangle>::new())
                }
                Err(error) => ProcessThreeDTriangleFrameResult::InvalidInput(error),
            }
        })
        .await
        .unwrap_or_else(|error| {
            ProcessThreeDTriangleFrameResult::InvalidInput(format!(
                "triangle registration task failed: {error}"
            ))
        });
        finish_calculation(id, calculation_result, Arc::clone(&state), result).await;
    }
}

async fn finish_calculation(
    id: u64,
    result: ProcessThreeDTriangleFrameResult,
    state: Arc<tokio::sync::Mutex<CalculationState>>,
    result_sender: watch::Sender<Option<ProcessThreeDTriangleFrameResult>>,
) {
    let mut state = state.lock().await;
    if state.active.as_ref().is_some_and(|active| active.id == id) {
        let _ = result_sender.send(Some(result));
        state.active = None;
    }
}

async fn wait_for_result(
    mut receiver: watch::Receiver<Option<ProcessThreeDTriangleFrameResult>>,
) -> ProcessThreeDTriangleFrameResult {
    loop {
        if let Some(result) = receiver.borrow_and_update().clone() {
            return result;
        }
        if receiver.changed().await.is_err() {
            return ProcessThreeDTriangleFrameResult::InvalidInput(
                "triangle calculation stopped unexpectedly".to_owned(),
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::service::{CameraIntrinsics, DepthFrame, RigidTransform, Service};
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::{Arc, Condvar};

    fn calibration() -> ThreeDTriangleCalibration {
        ThreeDTriangleCalibration {
            depth_camera: CameraIntrinsics {
                width: 1,
                height: 1,
                fx: 1.0,
                fy: 1.0,
                cx: 0.0,
                cy: 0.0,
                distortion_model: "none".to_owned(),
                distortion: Vec::new(),
            },
            rgb_camera: CameraIntrinsics {
                width: 1,
                height: 1,
                fx: 1.0,
                fy: 1.0,
                cx: 0.0,
                cy: 0.0,
                distortion_model: "none".to_owned(),
                distortion: Vec::new(),
            },
            depth_to_rgb: RigidTransform {
                translation: [0.0; 3],
                rotation_xyzw: [0.0, 0.0, 0.0, 1.0],
            },
        }
    }

    fn frame() -> ThreeDTriangleFrame {
        ThreeDTriangleFrame {
            rgb: crate::slices::WrappedRgbImage::new_with_color(1, 1, [0; 3]),
            depth: DepthFrame {
                width: 1,
                height: 1,
                metres: vec![1.0],
            },
        }
    }

    #[tokio::test]
    async fn overlapping_requests_coalesce_and_only_convert_the_admitted_frame() {
        let mut service = Service::new();
        service.create_three_d_triangle_session("triangles", Some(calibration()));
        let first = service
            .get_triangles_request("triangles", None)
            .expect("calibrated session has a request");
        let joined = service
            .get_triangles_request("triangles", None)
            .expect("calibrated session has a request");
        let first_conversions = Arc::new(AtomicUsize::new(0));
        let joined_conversions = Arc::new(AtomicUsize::new(0));
        let (started_sender, started_receiver) = tokio::sync::oneshot::channel();
        let gate = Arc::new((std::sync::Mutex::new(false), Condvar::new()));
        let first_gate = Arc::clone(&gate);
        let first_count = Arc::clone(&first_conversions);
        let first_task = tokio::spawn(async move {
            first
                .get_triangles_with(move || {
                    first_count.fetch_add(1, Ordering::SeqCst);
                    let _ = started_sender.send(());
                    let (lock, condition) = &*first_gate;
                    let mut released = lock.lock().unwrap();
                    while !*released {
                        released = condition.wait(released).unwrap();
                    }
                    Ok(frame())
                })
                .await
        });
        started_receiver
            .await
            .expect("first frame conversion should start");
        let joined_count = Arc::clone(&joined_conversions);
        let (joined_result, joined_started) = joined
            .calculation
            .start(joined.calibration, move || {
                joined_count.fetch_add(1, Ordering::SeqCst);
                Ok(frame())
            })
            .await;
        assert!(!joined_started);
        {
            let (lock, condition) = &*gate;
            *lock.lock().unwrap() = true;
            condition.notify_all();
        }
        let first_result = first_task.await.expect("first task should finish");
        let joined_result = wait_for_result(joined_result).await;
        assert_eq!(
            first_result,
            ProcessThreeDTriangleFrameResult::Success(Vec::new())
        );
        assert_eq!(joined_result, first_result);
        assert_eq!(first_conversions.load(Ordering::SeqCst), 1);
        assert_eq!(joined_conversions.load(Ordering::SeqCst), 0);
    }

    #[tokio::test]
    async fn try_start_with_drops_busy_frames_and_admits_after_completion() {
        let mut service = Service::new();
        service.create_three_d_triangle_session("triangles", Some(calibration()));
        let (started_sender, started_receiver) = tokio::sync::oneshot::channel();
        let gate = Arc::new((std::sync::Mutex::new(false), Condvar::new()));
        let worker_gate = Arc::clone(&gate);
        let first = service
            .get_triangles_request("triangles", None)
            .unwrap()
            .try_start_with(move || {
                let _ = started_sender.send(());
                let (lock, condition) = &*worker_gate;
                let mut released = lock.lock().unwrap();
                while !*released {
                    released = condition.wait(released).unwrap();
                }
                Ok(frame())
            })
            .await
            .expect("idle session should admit a frame");
        started_receiver
            .await
            .expect("frame conversion should start");

        let dropped_conversions = Arc::new(AtomicUsize::new(0));
        let dropped_counter = Arc::clone(&dropped_conversions);
        let dropped = service
            .get_triangles_request("triangles", None)
            .unwrap()
            .try_start_with(move || {
                dropped_counter.fetch_add(1, Ordering::SeqCst);
                Ok(frame())
            })
            .await;
        assert!(dropped.is_none());
        assert_eq!(dropped_conversions.load(Ordering::SeqCst), 0);

        {
            let (lock, condition) = &*gate;
            *lock.lock().unwrap() = true;
            condition.notify_all();
        }
        assert_eq!(
            first.result().await,
            ProcessThreeDTriangleFrameResult::Success(Vec::new())
        );
        let next = service
            .get_triangles_request("triangles", None)
            .unwrap()
            .try_start_with(|| Ok(frame()))
            .await
            .expect("session should admit a frame after completion");
        assert_eq!(
            next.result().await,
            ProcessThreeDTriangleFrameResult::Success(Vec::new())
        );
    }
}

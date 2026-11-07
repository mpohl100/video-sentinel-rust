#![warn(clippy::pedantic)]
#![allow(
    clippy::missing_errors_doc,
    clippy::missing_panics_doc,
    clippy::too_many_lines,
    clippy::unused_async_trait_impl
)]

use super::{GetRectanglesResult, Session};
use crate::slices::WrappedRgbImage;
use std::sync::{Arc, Mutex};
use tokio::sync::{mpsc, watch};

type ImageFactory = Box<dyn FnOnce() -> Result<WrappedRgbImage, String> + Send + 'static>;

struct ActiveCalculation {
    id: u64,
    result: watch::Receiver<Option<GetRectanglesResult>>,
}

#[derive(Default)]
struct CalculationState {
    next_id: u64,
    active: Option<ActiveCalculation>,
}

struct ImageJob {
    id: u64,
    session: Session,
    image: WrappedRgbImage,
    result: watch::Sender<Option<GetRectanglesResult>>,
}

pub(super) struct SessionCalculation {
    sender: mpsc::Sender<ImageJob>,
    receiver: Mutex<Option<mpsc::Receiver<ImageJob>>>,
    state: Arc<tokio::sync::Mutex<CalculationState>>,
}

impl SessionCalculation {
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

    pub(super) async fn submit<F>(
        &self,
        mut session: Session,
        image_factory: F,
    ) -> GetRectanglesResult
    where
        F: FnOnce() -> Result<WrappedRgbImage, String> + Send + 'static,
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

        if let Some((id, result_sender)) = calculation_id {
            refresh_cache_for_admission(&mut session);
            let sender = self.sender.clone();
            let state = Arc::clone(&self.state);
            tokio::spawn(async move {
                submit_image_job(
                    id,
                    session,
                    Box::new(image_factory),
                    sender,
                    state,
                    result_sender,
                )
                .await;
            });
        } else {
            drop(image_factory);
        }

        wait_for_result(result_receiver).await
    }
}

fn refresh_cache_for_admission(session: &mut Session) {
    if let Session::Eye(eye_session) = session {
        let cache = eye_session.cache_state.lock().unwrap();
        eye_session.cached_previous_mosaics = if cache.0 == eye_session.cache_generation {
            cache.1.clone()
        } else {
            None
        };
    }
}

pub struct GetRectanglesRequest {
    session: Session,
}

impl GetRectanglesRequest {
    pub(super) fn new(session: Session) -> Self {
        Self { session }
    }

    pub async fn get_rectangles_with<F>(self, image_factory: F) -> GetRectanglesResult
    where
        F: FnOnce() -> Result<WrappedRgbImage, String> + Send + 'static,
    {
        let calculation = self.session.calculation().clone();
        calculation.submit(self.session, image_factory).await
    }
}

async fn submit_image_job(
    id: u64,
    session: Session,
    image_factory: ImageFactory,
    sender: mpsc::Sender<ImageJob>,
    state: Arc<tokio::sync::Mutex<CalculationState>>,
    result: watch::Sender<Option<GetRectanglesResult>>,
) {
    let Ok(permit) = sender.reserve_owned().await else {
        finish_calculation(
            id,
            GetRectanglesResult::ComputationFailed("session image job channel closed".to_owned()),
            state,
            result,
        )
        .await;
        return;
    };
    let image = match tokio::task::spawn_blocking(image_factory).await {
        Ok(Ok(image)) => image,
        Ok(Err(error)) => {
            finish_calculation(
                id,
                GetRectanglesResult::ComputationFailed(error),
                state,
                result,
            )
            .await;
            return;
        }
        Err(error) => {
            finish_calculation(
                id,
                GetRectanglesResult::ComputationFailed(format!(
                    "image conversion task failed: {error}"
                )),
                state,
                result,
            )
            .await;
            return;
        }
    };

    let job = ImageJob {
        id,
        session,
        image,
        result: result.clone(),
    };
    permit.send(job);
}

async fn run_worker(
    mut receiver: mpsc::Receiver<ImageJob>,
    state: Arc<tokio::sync::Mutex<CalculationState>>,
) {
    while let Some(job) = receiver.recv().await {
        let ImageJob {
            id,
            session,
            image,
            result,
        } = job;
        let calculation_result = tokio::spawn(calculate(session, image))
            .await
            .unwrap_or_else(|error| {
                GetRectanglesResult::ComputationFailed(format!(
                    "session calculation task failed: {error}"
                ))
            });
        finish_calculation(id, calculation_result, Arc::clone(&state), result).await;
    }
}

async fn finish_calculation(
    id: u64,
    result: GetRectanglesResult,
    state: Arc<tokio::sync::Mutex<CalculationState>>,
    result_sender: watch::Sender<Option<GetRectanglesResult>>,
) {
    let mut state = state.lock().await;
    if state.active.as_ref().is_some_and(|active| active.id == id) {
        let _ = result_sender.send(Some(result));
        state.active = None;
    }
}

async fn wait_for_result(
    mut receiver: watch::Receiver<Option<GetRectanglesResult>>,
) -> GetRectanglesResult {
    loop {
        if let Some(result) = receiver.borrow_and_update().clone() {
            return result;
        }
        if receiver.changed().await.is_err() {
            return GetRectanglesResult::ComputationFailed(
                "session calculation task stopped unexpectedly".to_owned(),
            );
        }
    }
}

async fn calculate(session: Session, image: WrappedRgbImage) -> GetRectanglesResult {
    use super::{calculate_eye, calculate_object, calculate_ordinary, deduce_enriched_mosaic};

    match session {
        Session::Eye(eye_session) => {
            let result_mode = eye_session.results;
            match calculate_eye(&eye_session, image).await {
                Ok((mosaics, new_cache)) => {
                    {
                        let mut cache = eye_session.cache_state.lock().unwrap();
                        if cache.0 == eye_session.cache_generation {
                            cache.1 = Some(new_cache);
                        }
                    }
                    let result = tokio::task::spawn_blocking(move || {
                        GetRectanglesResult::Success(
                            mosaics
                                .into_iter()
                                .map(|mosaic| {
                                    deduce_enriched_mosaic(
                                        mosaic.mosaic(),
                                        mosaic.color().clone(),
                                        mosaic.label().cloned(),
                                        result_mode,
                                    )
                                })
                                .collect(),
                        )
                    })
                    .await;
                    result.unwrap_or_else(|error| {
                        GetRectanglesResult::ComputationFailed(format!(
                            "failed to enrich eye calculation result: {error}"
                        ))
                    })
                }
                Err(error) => GetRectanglesResult::ComputationFailed(error),
            }
        }
        Session::Object(object_session) => match calculate_object(&object_session, image).await {
            Ok(mosaics) => {
                let result_mode = object_session.results;
                let result = tokio::task::spawn_blocking(move || {
                    GetRectanglesResult::Success(
                        mosaics
                            .into_iter()
                            .map(|mosaic| {
                                deduce_enriched_mosaic(
                                    mosaic.mosaic(),
                                    mosaic.color().clone(),
                                    mosaic.label().cloned(),
                                    result_mode,
                                )
                            })
                            .collect(),
                    )
                })
                .await;
                result.unwrap_or_else(|error| {
                    GetRectanglesResult::ComputationFailed(format!(
                        "failed to enrich object calculation result: {error}"
                    ))
                })
            }
            Err(error) => GetRectanglesResult::ComputationFailed(error),
        },
        Session::Ordinary(ordinary_session) => {
            match tokio::task::spawn_blocking(move || {
                calculate_ordinary(&ordinary_session, &image)
                    .into_iter()
                    .map(|mosaic| {
                        deduce_enriched_mosaic(
                            mosaic.mosaic(),
                            mosaic.color().clone(),
                            mosaic.label().cloned(),
                            ordinary_session.results,
                        )
                    })
                    .collect()
            })
            .await
            {
                Ok(mosaics) => GetRectanglesResult::Success(mosaics),
                Err(error) => GetRectanglesResult::ComputationFailed(format!(
                    "ordinary calculation task failed: {error}"
                )),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mosaics::Results;
    use crate::service::{
        BasicParamsInput, EyeParamsInput, Service, TileParamsInput, TraceParamsInput,
    };
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::{Arc, Condvar};

    struct DropSignal(Option<tokio::sync::oneshot::Sender<()>>);

    impl Drop for DropSignal {
        fn drop(&mut self) {
            if let Some(sender) = self.0.take() {
                let _ = sender.send(());
            }
        }
    }

    #[tokio::test]
    async fn concurrent_requests_share_results_and_skip_joined_conversion() {
        let service = ordinary_service();
        let first = service.get_rectangles_request("ordinary").unwrap();
        let joined = service.get_rectangles_request("ordinary").unwrap();
        let first_conversions = Arc::new(AtomicUsize::new(0));
        let joined_conversions = Arc::new(AtomicUsize::new(0));
        let (started_sender, started_receiver) = tokio::sync::oneshot::channel();
        let release_lock = Arc::new((std::sync::Mutex::new(false), Condvar::new()));
        let first_release = Arc::clone(&release_lock);
        let first_count = Arc::clone(&first_conversions);

        let first_task = tokio::spawn(async move {
            first
                .get_rectangles_with(move || {
                    first_count.fetch_add(1, Ordering::SeqCst);
                    let _ = started_sender.send(());
                    let (lock, condvar) = &*first_release;
                    let mut released = lock.lock().unwrap();
                    while !*released {
                        released = condvar.wait(released).unwrap();
                    }
                    Ok(WrappedRgbImage::new_with_color(80, 80, [255, 255, 255]))
                })
                .await
        });
        started_receiver.await.unwrap();

        let joined_count = Arc::clone(&joined_conversions);
        let (joined_dropped_sender, joined_dropped) = tokio::sync::oneshot::channel();
        let joined_drop_signal = DropSignal(Some(joined_dropped_sender));
        let joined_task = tokio::spawn(async move {
            joined
                .get_rectangles_with(move || {
                    let _signal = joined_drop_signal;
                    joined_count.fetch_add(1, Ordering::SeqCst);
                    Ok(WrappedRgbImage::new_with_color(80, 80, [0, 0, 0]))
                })
                .await
        });
        joined_dropped.await.unwrap();
        {
            let (lock, condvar) = &*release_lock;
            *lock.lock().unwrap() = true;
            condvar.notify_all();
        }

        let first_result = first_task.await.unwrap();
        let joined_result = joined_task.await.unwrap();
        let first_mosaics = success(first_result);
        let joined_mosaics = success(joined_result);
        assert_eq!(first_conversions.load(Ordering::SeqCst), 1);
        assert_eq!(joined_conversions.load(Ordering::SeqCst), 0);
        assert_eq!(first_mosaics.len(), joined_mosaics.len());
        assert_eq!(
            first_mosaics
                .iter()
                .map(mosaic_signature)
                .collect::<Vec<_>>(),
            joined_mosaics
                .iter()
                .map(mosaic_signature)
                .collect::<Vec<_>>()
        );
        assert!(!first_mosaics.is_empty());
        assert_eq!(first_mosaics[0].average_color.red, 255);

        let next = service.get_rectangles_request("ordinary").unwrap();
        let next_conversions = Arc::new(AtomicUsize::new(0));
        let next_count = Arc::clone(&next_conversions);
        let next_result = next
            .get_rectangles_with(move || {
                next_count.fetch_add(1, Ordering::SeqCst);
                Ok(WrappedRgbImage::new_with_color(80, 80, [0, 0, 0]))
            })
            .await;
        let next_mosaics = success(next_result);
        assert_eq!(next_conversions.load(Ordering::SeqCst), 1);
        assert!(!next_mosaics.is_empty());
    }

    #[tokio::test]
    async fn cancellation_does_not_cancel_shared_calculation() {
        let service = ordinary_service();
        let owner = service.get_rectangles_request("ordinary").unwrap();
        let waiter = service.get_rectangles_request("ordinary").unwrap();
        let (started_sender, started_receiver) = tokio::sync::oneshot::channel();
        let (waiter_dropped_sender, waiter_dropped) = tokio::sync::oneshot::channel();
        let waiter_drop_signal = DropSignal(Some(waiter_dropped_sender));
        let release_lock = Arc::new((std::sync::Mutex::new(false), Condvar::new()));
        let owner_release = Arc::clone(&release_lock);
        let owner_task = tokio::spawn(async move {
            owner
                .get_rectangles_with(move || {
                    let _ = started_sender.send(());
                    let (lock, condvar) = &*owner_release;
                    let mut released = lock.lock().unwrap();
                    while !*released {
                        released = condvar.wait(released).unwrap();
                    }
                    Ok(WrappedRgbImage::new_with_color(64, 64, [255, 255, 255]))
                })
                .await
        });
        started_receiver.await.unwrap();
        let waiter_task = tokio::spawn(async move {
            waiter
                .get_rectangles_with(move || {
                    let _signal = waiter_drop_signal;
                    Ok(WrappedRgbImage::new_with_color(64, 64, [0, 0, 0]))
                })
                .await
        });
        waiter_dropped.await.unwrap();
        owner_task.abort();
        {
            let (lock, condvar) = &*release_lock;
            *lock.lock().unwrap() = true;
            condvar.notify_all();
        }
        let mosaics = success(waiter_task.await.unwrap());
        assert!(!mosaics.is_empty());
        assert_eq!(mosaics[0].average_color.red, 255);
    }

    #[tokio::test]
    async fn conversion_failure_releases_admission_for_next_frame() {
        let service = ordinary_service();
        let invalid = service.get_rectangles_request("ordinary").unwrap();
        let result = invalid
            .get_rectangles_with(|| Err("invalid image".to_owned()))
            .await;
        assert!(matches!(
            result,
            GetRectanglesResult::ComputationFailed(error) if error == "invalid image"
        ));

        let valid = service.get_rectangles_request("ordinary").unwrap();
        assert!(
            !success(
                valid
                    .get_rectangles_with(|| {
                        Ok(WrappedRgbImage::new_with_color(64, 64, [255, 255, 255]))
                    })
                    .await
            )
            .is_empty()
        );
    }

    #[tokio::test]
    async fn eye_getter_and_delayed_request_refresh_completed_cache() {
        let mut service = Service::new();
        service.create_eye_session(
            "eye",
            BasicParamsInput {
                do_grayscale: false,
                gradient_threshold: 15,
            },
            EyeParamsInput {
                tile_params: TileParamsInput {
                    tile_x: 0.5,
                    tile_y: 0.5,
                },
                bucket_delta: 0.25,
                trace_params: TraceParamsInput { num_skeleton: 12 },
                target_similarity: 1.1,
            },
            Results::Absolute,
        );
        let delayed_request = service.get_rectangles_request("eye").unwrap();

        assert!(matches!(
            service
                .get_rectangles(
                    "eye",
                    WrappedRgbImage::new_with_color(16, 16, [255, 255, 255])
                )
                .await,
            GetRectanglesResult::Success(_)
        ));
        assert!(
            service
                .get_eye_session("eye")
                .unwrap()
                .cached_previous_mosaics
                .is_some()
        );

        let delayed_result = delayed_request
            .get_rectangles_with(|| Ok(WrappedRgbImage::new_with_color(16, 16, [255, 255, 255])))
            .await;
        let mosaics = success(delayed_result);
        assert!(!mosaics.is_empty());
        assert!(
            mosaics
                .iter()
                .all(|mosaic| mosaic.color == crate::slices::Color::Red)
        );

        let invalidated_request = service.get_rectangles_request("eye").unwrap();
        service.update_basic_params(
            "eye",
            BasicParamsInput {
                do_grayscale: false,
                gradient_threshold: 15,
            },
        );
        let invalidated_result = invalidated_request
            .get_rectangles_with(|| Ok(WrappedRgbImage::new_with_color(16, 16, [255, 255, 255])))
            .await;
        let mosaics = success(invalidated_result);
        assert!(!mosaics.is_empty());
        assert!(
            mosaics
                .iter()
                .all(|mosaic| mosaic.color == crate::slices::Color::Blue)
        );
        assert!(
            service
                .get_eye_session("eye")
                .unwrap()
                .cached_previous_mosaics
                .is_none()
        );
    }

    #[tokio::test]
    async fn ordinary_calculation_does_not_block_current_thread_runtime() {
        let service = ordinary_service();
        let request = service.get_rectangles_request("ordinary").unwrap();
        let (image_ready_sender, image_ready_receiver) = tokio::sync::oneshot::channel();
        let task = tokio::spawn(async move {
            request
                .get_rectangles_with(move || {
                    let _ = image_ready_sender.send(());
                    Ok(WrappedRgbImage::new_with_color(700, 700, [255, 255, 255]))
                })
                .await
        });
        image_ready_receiver.await.unwrap();
        tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        assert!(!task.is_finished());
        assert!(!success(task.await.unwrap()).is_empty());
    }

    fn ordinary_service() -> Service {
        let mut service = Service::new();
        service.create_ordinary_session(
            "ordinary",
            BasicParamsInput {
                do_grayscale: false,
                gradient_threshold: 15,
            },
            Results::Absolute,
        );
        service
    }

    fn success(result: GetRectanglesResult) -> Vec<super::super::EnrichedMosaic> {
        match result {
            GetRectanglesResult::Success(mosaics) => mosaics,
            GetRectanglesResult::SessionNotFound => panic!("session should exist"),
            GetRectanglesResult::ComputationFailed(error) => panic!("calculation failed: {error}"),
        }
    }

    fn mosaic_signature(mosaic: &super::super::EnrichedMosaic) -> String {
        let top_left = mosaic.bounding_box.get_top_left();
        let bottom_right = mosaic.bounding_box.get_bottom_right();
        let circle_center = mosaic.bounding_circle.center;
        let center = mosaic.center_of_mass;
        let color = match &mosaic.color {
            crate::slices::Color::Red => "red",
            crate::slices::Color::Green => "green",
            crate::slices::Color::Blue => "blue",
        };
        let results = match mosaic.results {
            Results::Absolute => "absolute",
            Results::Relative => "relative",
        };
        let slices: Vec<_> = mosaic
            .slice_matrix
            .iter()
            .map(|line| {
                (
                    line.line_number,
                    line.slices
                        .iter()
                        .map(|slice| (slice.start.x, slice.start.y, slice.end.x, slice.end.y))
                        .collect::<Vec<_>>(),
                )
            })
            .collect();
        format!(
            "{:?}|{:?}|{:?}|{:?}|{:?}|{:?}|{:?}|{:?}|{:?}|{:?}|{color}|{results}",
            (top_left.x, top_left.y, top_left.z),
            (bottom_right.x, bottom_right.y, bottom_right.z),
            (circle_center.x, circle_center.y, circle_center.z),
            mosaic.bounding_circle.radius,
            (center.x, center.y, center.z),
            mosaic.area,
            (
                mosaic.average_color.red,
                mosaic.average_color.green,
                mosaic.average_color.blue
            ),
            mosaic.label,
            mosaic.slice_matrix.len(),
            slices
        )
    }
}

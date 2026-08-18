//! Safe, optional NDI video-output boundary.
//!
//! The public API contains only application-owned configuration, status,
//! metrics, and [`vtuber_core::VideoOutputFrame`]. NDI SDK types and the
//! binding feature remain private to this crate. The default build is a
//! deterministic feature-disabled stub so the rest of the workspace does not
//! require an installed NDI SDK.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

use std::fmt;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Condvar, Mutex, MutexGuard};
use vtuber_core::{FrameSeq, VideoOutputFrame, VideoOutputPixelFormat, VideoOutputProfile};

/// Returns whether this build includes the explicit NDI SDK backend.
#[must_use]
pub const fn is_sdk_feature_enabled() -> bool {
    cfg!(feature = "ndi-sdk")
}

/// Stable error codes emitted by the optional output backend.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum NdiErrorCode {
    /// The SDK feature was not enabled in this build.
    FeatureDisabled,
    /// The runtime library could not be found by the operating system.
    RuntimeNotFound,
    /// The NDI runtime failed to initialize.
    RuntimeInitFailed,
    /// The named sender could not be created.
    SenderCreateFailed,
    /// A frame could not be submitted to the sender.
    SendFailed,
    /// A worker could not be stopped and joined cleanly.
    WorkerStopFailed,
    /// A second start was requested while the sender was active.
    AlreadyRunning,
    /// The output configuration is invalid.
    InvalidConfiguration,
    /// A frame did not satisfy the fixed BGRA output contract.
    InvalidFrame,
    /// A frame was submitted while the sender was not active.
    NotRunning,
}

impl NdiErrorCode {
    /// Returns the stable machine-readable error code.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::FeatureDisabled => "NDI_FEATURE_DISABLED",
            Self::RuntimeNotFound => "NDI_RUNTIME_NOT_FOUND",
            Self::RuntimeInitFailed => "NDI_RUNTIME_INIT_FAILED",
            Self::SenderCreateFailed => "NDI_SENDER_CREATE_FAILED",
            Self::SendFailed => "NDI_SEND_FAILED",
            Self::WorkerStopFailed => "NDI_WORKER_STOP_FAILED",
            Self::AlreadyRunning => "NDI_ALREADY_RUNNING",
            Self::InvalidConfiguration => "NDI_INVALID_CONFIGURATION",
            Self::InvalidFrame => "NDI_INVALID_FRAME",
            Self::NotRunning => "NDI_NOT_RUNNING",
        }
    }
}

/// A stable, user-safe backend error.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NdiOutputError {
    /// Stable error classification.
    pub code: NdiErrorCode,
    /// Short diagnostic message without local paths or SDK handles.
    pub message: String,
}

impl NdiOutputError {
    fn new(code: NdiErrorCode, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
        }
    }
}

impl fmt::Display for NdiOutputError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}: {}", self.code.as_str(), self.message)
    }
}

impl std::error::Error for NdiOutputError {}

/// Runtime state visible to the application/UI layer.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub enum NdiOutputStatus {
    /// No sender worker exists.
    #[default]
    Off,
    /// The worker is initializing the runtime and named sender.
    Starting,
    /// The sender is publishing frames.
    Live {
        /// Number of currently connected receivers at the last worker poll.
        connections: u32,
        /// Requested stable source name.
        source_name: String,
    },
    /// The sender stopped because of a recoverable failure.
    Error {
        /// Stable error classification.
        code: NdiErrorCode,
        /// User-safe diagnostic detail.
        message: String,
    },
}

/// Configuration for one named transparent video sender.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NdiOutputConfig {
    /// UTF-8 source name shown to NDI finders and OBS.
    pub source_name: String,
    /// Fixed output dimensions and frame rate.
    pub profile: VideoOutputProfile,
}

impl Default for NdiOutputConfig {
    fn default() -> Self {
        Self {
            source_name: "vrm-bevy-vtuber".to_owned(),
            profile: VideoOutputProfile::DEFAULT,
        }
    }
}

impl NdiOutputConfig {
    fn validate(&self) -> Result<(), NdiOutputError> {
        if self.source_name.trim().is_empty()
            || self.source_name.chars().any(char::is_control)
            || self.source_name.contains('\0')
        {
            return Err(NdiOutputError::new(
                NdiErrorCode::InvalidConfiguration,
                "source name must be non-empty UTF-8 without control characters",
            ));
        }
        if self.profile.width == 0 || self.profile.height == 0 || self.profile.fps == 0 {
            return Err(NdiOutputError::new(
                NdiErrorCode::InvalidConfiguration,
                "output width, height, and fps must be non-zero",
            ));
        }
        if self.profile.pixel_format != VideoOutputPixelFormat::Bgra8StraightAlpha {
            return Err(NdiOutputError::new(
                NdiErrorCode::InvalidConfiguration,
                "only BGRA8 straight-alpha output is supported",
            ));
        }
        Ok(())
    }
}

/// Commands understood by the backend controller.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum NdiOutputCommand {
    /// Start a sender with the supplied source name and profile.
    Start(NdiOutputConfig),
    /// Stop the current sender; stopping an already-off sender is safe.
    Stop,
}

/// Result of attempting to submit a frame to the bounded mailbox.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NdiSubmitResult {
    /// The mailbox was empty and now contains the frame.
    Submitted,
    /// A pending frame was replaced by this newer frame.
    Replaced,
    /// The sender is not active or is shutting down.
    RejectedNotRunning,
}

/// A transport-neutral description of the NDI High Bandwidth video mapping.
///
/// This type intentionally uses no NDI SDK enum. It can be tested in the
/// normal SDK-free build and is the only descriptor passed into the optional
/// binding adapter.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct NdiVideoFrameMapping {
    /// Frame width in pixels.
    pub width: i32,
    /// Frame height in pixels.
    pub height: i32,
    /// Validated BGRA row stride in bytes.
    pub stride_bytes: i32,
    /// Frame-rate numerator.
    pub frame_rate_n: i32,
    /// Frame-rate denominator.
    pub frame_rate_d: i32,
    /// Square-pixel picture aspect ratio.
    pub picture_aspect_ratio: f32,
    /// Standard NDI FourCC selected by this mapping.
    pub four_cc: NdiFourCc,
}

/// FourCC values exposed by the transport-neutral mapping.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NdiFourCc {
    /// NDI BGRA with a preserved alpha byte.
    Bgra,
}

/// Validates the #46 frame contract and maps it to standard NDI video fields.
pub fn map_video_frame(
    frame: &VideoOutputFrame,
    profile: VideoOutputProfile,
) -> Result<NdiVideoFrameMapping, NdiOutputError> {
    if frame.pixel_format != VideoOutputPixelFormat::Bgra8StraightAlpha {
        return Err(NdiOutputError::new(
            NdiErrorCode::InvalidFrame,
            "frame pixel format is not BGRA8 straight alpha",
        ));
    }
    let stride = profile.packed_stride_bytes();
    let expected_len = stride
        .checked_mul(profile.height as usize)
        .ok_or_else(|| NdiOutputError::new(NdiErrorCode::InvalidFrame, "frame size overflow"))?;
    if frame.width != profile.width
        || frame.height != profile.height
        || frame.stride_bytes != stride
        || frame.data.len() != expected_len
    {
        return Err(NdiOutputError::new(
            NdiErrorCode::InvalidFrame,
            "frame dimensions, stride, or data length do not match the output profile",
        ));
    }
    let width = i32::try_from(frame.width).map_err(|_| {
        NdiOutputError::new(NdiErrorCode::InvalidFrame, "frame width exceeds NDI range")
    })?;
    let height = i32::try_from(frame.height).map_err(|_| {
        NdiOutputError::new(NdiErrorCode::InvalidFrame, "frame height exceeds NDI range")
    })?;
    let stride_bytes = i32::try_from(frame.stride_bytes).map_err(|_| {
        NdiOutputError::new(NdiErrorCode::InvalidFrame, "frame stride exceeds NDI range")
    })?;
    let frame_rate_n = i32::try_from(profile.fps).map_err(|_| {
        NdiOutputError::new(NdiErrorCode::InvalidFrame, "frame rate exceeds NDI range")
    })?;
    Ok(NdiVideoFrameMapping {
        width,
        height,
        stride_bytes,
        frame_rate_n,
        frame_rate_d: 1,
        picture_aspect_ratio: profile.width as f32 / profile.height as f32,
        four_cc: NdiFourCc::Bgra,
    })
}

/// Bounded counters collected by the sender boundary.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct NdiOutputMetrics {
    /// Frames accepted into the latest-value mailbox.
    pub submitted_frames: u64,
    /// Frames successfully handed to the SDK sender.
    pub sent_frames: u64,
    /// Pending frames replaced by a newer frame.
    pub replaced_frames: u64,
    /// Frames rejected or discarded during shutdown.
    pub dropped_frames: u64,
    /// Malformed frames rejected before an SDK call.
    pub rejected_frames: u64,
    /// Sender/runtime initialization failures.
    pub start_failures: u64,
    /// Most recent frame sequence successfully handed to the SDK.
    pub last_frame_seq: Option<FrameSeq>,
}

#[derive(Debug)]
struct MetricsInner {
    submitted_frames: AtomicU64,
    sent_frames: AtomicU64,
    replaced_frames: AtomicU64,
    dropped_frames: AtomicU64,
    rejected_frames: AtomicU64,
    start_failures: AtomicU64,
    last_frame_seq: AtomicU64,
}

impl Default for MetricsInner {
    fn default() -> Self {
        Self {
            submitted_frames: AtomicU64::new(0),
            sent_frames: AtomicU64::new(0),
            replaced_frames: AtomicU64::new(0),
            dropped_frames: AtomicU64::new(0),
            rejected_frames: AtomicU64::new(0),
            start_failures: AtomicU64::new(0),
            last_frame_seq: AtomicU64::new(u64::MAX),
        }
    }
}

impl MetricsInner {
    fn reset(&self) {
        for counter in [
            &self.submitted_frames,
            &self.sent_frames,
            &self.replaced_frames,
            &self.dropped_frames,
            &self.rejected_frames,
            &self.start_failures,
        ] {
            counter.store(0, Ordering::Relaxed);
        }
        self.last_frame_seq.store(u64::MAX, Ordering::Relaxed);
    }

    fn snapshot(&self) -> NdiOutputMetrics {
        let last_frame_seq = match self.last_frame_seq.load(Ordering::Relaxed) {
            u64::MAX => None,
            seq => Some(FrameSeq(seq)),
        };
        NdiOutputMetrics {
            submitted_frames: self.submitted_frames.load(Ordering::Relaxed),
            sent_frames: self.sent_frames.load(Ordering::Relaxed),
            replaced_frames: self.replaced_frames.load(Ordering::Relaxed),
            dropped_frames: self.dropped_frames.load(Ordering::Relaxed),
            rejected_frames: self.rejected_frames.load(Ordering::Relaxed),
            start_failures: self.start_failures.load(Ordering::Relaxed),
            last_frame_seq,
        }
    }
}

#[derive(Debug)]
struct MailboxState {
    latest: Option<VideoOutputFrame>,
    closed: bool,
}

#[derive(Debug)]
struct LatestFrameMailbox {
    state: Mutex<MailboxState>,
    available: Condvar,
}

impl LatestFrameMailbox {
    fn new() -> Self {
        Self {
            state: Mutex::new(MailboxState {
                latest: None,
                closed: false,
            }),
            available: Condvar::new(),
        }
    }

    fn submit(&self, frame: VideoOutputFrame) -> NdiSubmitResult {
        let mut state = recover_lock(self.state.lock());
        if state.closed {
            return NdiSubmitResult::RejectedNotRunning;
        }
        let result = if state.latest.replace(frame).is_some() {
            NdiSubmitResult::Replaced
        } else {
            NdiSubmitResult::Submitted
        };
        self.available.notify_one();
        result
    }

    #[cfg(any(feature = "ndi-sdk", test))]
    fn take(&self, stop_requested: impl Fn() -> bool) -> Option<VideoOutputFrame> {
        let mut state = recover_lock(self.state.lock());
        loop {
            if let Some(frame) = state.latest.take() {
                return Some(frame);
            }
            if state.closed || stop_requested() {
                return None;
            }
            state = self
                .available
                .wait_timeout(state, std::time::Duration::from_millis(50))
                .map_or_else(|poisoned| poisoned.into_inner().0, |result| result.0);
        }
    }

    fn close(&self) -> bool {
        let mut state = recover_lock(self.state.lock());
        state.closed = true;
        let discarded = state.latest.take().is_some();
        self.available.notify_all();
        discarded
    }
}

fn recover_lock<T>(result: std::sync::LockResult<MutexGuard<'_, T>>) -> MutexGuard<'_, T> {
    result.unwrap_or_else(|poisoned| poisoned.into_inner())
}

#[derive(Debug)]
struct SharedState {
    status: Mutex<NdiOutputStatus>,
    mailbox: Mutex<Option<Arc<LatestFrameMailbox>>>,
    metrics: MetricsInner,
}

impl SharedState {
    fn status(&self) -> NdiOutputStatus {
        recover_lock(self.status.lock()).clone()
    }

    fn set_status(&self, status: NdiOutputStatus) {
        *recover_lock(self.status.lock()) = status;
    }

    fn replace_status_error(&self, error: &NdiOutputError) {
        self.set_status(NdiOutputStatus::Error {
            code: error.code,
            message: error.message.clone(),
        });
    }
}

/// Owns at most one sender worker and its bounded latest-frame mailbox.
pub struct NdiOutputController {
    shared: Arc<SharedState>,
    #[cfg(feature = "ndi-sdk")]
    worker: Option<vtuber_core::WorkerHandle<WorkerExit>>,
}

impl fmt::Debug for NdiOutputController {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut debug = formatter.debug_struct("NdiOutputController");
        debug.field("status", &self.status());
        #[cfg(feature = "ndi-sdk")]
        debug.field("worker_present", &self.worker.is_some());
        debug.finish()
    }
}

impl Default for NdiOutputController {
    fn default() -> Self {
        Self::new()
    }
}

impl NdiOutputController {
    /// Creates an inactive controller without touching the NDI runtime.
    #[must_use]
    pub fn new() -> Self {
        Self {
            shared: Arc::new(SharedState {
                status: Mutex::new(NdiOutputStatus::Off),
                mailbox: Mutex::new(None),
                metrics: MetricsInner::default(),
            }),
            #[cfg(feature = "ndi-sdk")]
            worker: None,
        }
    }

    /// Applies one start/stop command.
    pub fn apply(&mut self, command: NdiOutputCommand) -> Result<(), NdiOutputError> {
        match command {
            NdiOutputCommand::Start(config) => self.start(config),
            NdiOutputCommand::Stop => self.stop(),
        }
    }

    /// Returns the current worker status.
    #[must_use]
    pub fn status(&self) -> NdiOutputStatus {
        self.shared.status()
    }

    /// Returns a bounded snapshot of sender metrics.
    #[must_use]
    pub fn metrics(&self) -> NdiOutputMetrics {
        self.shared.metrics.snapshot()
    }

    /// Starts one sender worker.
    ///
    /// With the default feature set this transitions to a typed
    /// `NDI_FEATURE_DISABLED` error without spawning a thread. With
    /// `ndi-sdk`, runtime initialization and sender creation occur inside the
    /// worker so no SDK handle crosses the application boundary.
    pub fn start(&mut self, config: NdiOutputConfig) -> Result<(), NdiOutputError> {
        self.reap_finished_worker();
        #[cfg(feature = "ndi-sdk")]
        if self.worker.is_some() {
            return Err(NdiOutputError::new(
                NdiErrorCode::AlreadyRunning,
                "NDI output is already starting or live",
            ));
        }
        if !matches!(
            self.status(),
            NdiOutputStatus::Off | NdiOutputStatus::Error { .. }
        ) {
            return Err(NdiOutputError::new(
                NdiErrorCode::AlreadyRunning,
                "NDI output is already starting or live",
            ));
        }
        if let Err(error) = config.validate() {
            self.shared.replace_status_error(&error);
            return Err(error);
        }
        self.shared.metrics.reset();
        self.shared.set_status(NdiOutputStatus::Starting);
        let mailbox = Arc::new(LatestFrameMailbox::new());
        *recover_lock(self.shared.mailbox.lock()) = Some(Arc::clone(&mailbox));

        #[cfg(not(feature = "ndi-sdk"))]
        {
            mailbox.close();
            *recover_lock(self.shared.mailbox.lock()) = None;
            let error = NdiOutputError::new(
                NdiErrorCode::FeatureDisabled,
                "NDI output was not enabled for this build",
            );
            self.shared
                .metrics
                .start_failures
                .fetch_add(1, Ordering::Relaxed);
            self.shared.replace_status_error(&error);
            Err(error)
        }

        #[cfg(feature = "ndi-sdk")]
        {
            let shared = Arc::clone(&self.shared);
            self.worker = Some(vtuber_core::WorkerHandle::spawn(
                "ndi-output-sender",
                move |stop| run_ndi_worker(shared, mailbox, config, stop),
            ));
            Ok(())
        }
    }

    /// Stops and joins the sender worker. The operation is idempotent.
    pub fn stop(&mut self) -> Result<(), NdiOutputError> {
        let mailbox = recover_lock(self.shared.mailbox.lock()).take();
        if let Some(mailbox) = mailbox
            && mailbox.close()
        {
            self.shared
                .metrics
                .dropped_frames
                .fetch_add(1, Ordering::Relaxed);
        }
        #[cfg(not(feature = "ndi-sdk"))]
        {
            self.shared.set_status(NdiOutputStatus::Off);
            Ok(())
        }
        #[cfg(feature = "ndi-sdk")]
        {
            let Some(worker) = self.worker.take() else {
                self.shared.set_status(NdiOutputStatus::Off);
                return Ok(());
            };
            worker.stop();
            match worker.join() {
                vtuber_core::WorkerResult::Completed(WorkerExit::Stopped)
                | vtuber_core::WorkerResult::Completed(WorkerExit::StartupFailed) => {
                    self.shared.set_status(NdiOutputStatus::Off);
                    Ok(())
                }
                vtuber_core::WorkerResult::Panicked | vtuber_core::WorkerResult::SpawnFailed => {
                    let error = NdiOutputError::new(
                        NdiErrorCode::WorkerStopFailed,
                        "NDI sender worker did not join cleanly",
                    );
                    self.shared.replace_status_error(&error);
                    Err(error)
                }
            }
        }
    }

    /// Submits a frame without waiting for the network sender.
    pub fn submit_frame(&self, frame: VideoOutputFrame) -> NdiSubmitResult {
        let status = self.status();
        if !matches!(
            status,
            NdiOutputStatus::Starting | NdiOutputStatus::Live { .. }
        ) {
            return NdiSubmitResult::RejectedNotRunning;
        }
        let result = recover_lock(self.shared.mailbox.lock())
            .as_ref()
            .map_or(NdiSubmitResult::RejectedNotRunning, |mailbox| {
                mailbox.submit(frame)
            });
        match result {
            NdiSubmitResult::Submitted => {
                self.shared
                    .metrics
                    .submitted_frames
                    .fetch_add(1, Ordering::Relaxed);
            }
            NdiSubmitResult::Replaced => {
                self.shared
                    .metrics
                    .submitted_frames
                    .fetch_add(1, Ordering::Relaxed);
                self.shared
                    .metrics
                    .replaced_frames
                    .fetch_add(1, Ordering::Relaxed);
            }
            NdiSubmitResult::RejectedNotRunning => {}
        }
        result
    }

    #[cfg(feature = "ndi-sdk")]
    fn reap_finished_worker(&mut self) {
        let finished = self
            .worker
            .as_ref()
            .is_some_and(vtuber_core::WorkerHandle::is_finished);
        if finished {
            let worker = self.worker.take().expect("finished worker exists");
            let _ = worker.join();
            *recover_lock(self.shared.mailbox.lock()) = None;
        }
    }

    #[cfg(not(feature = "ndi-sdk"))]
    fn reap_finished_worker(&mut self) {}
}

impl Drop for NdiOutputController {
    fn drop(&mut self) {
        let _ = self.stop();
    }
}

#[cfg(feature = "ndi-sdk")]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum WorkerExit {
    Stopped,
    StartupFailed,
}

#[cfg(feature = "ndi-sdk")]
fn run_ndi_worker(
    shared: Arc<SharedState>,
    mailbox: Arc<LatestFrameMailbox>,
    config: NdiOutputConfig,
    stop: vtuber_core::StopToken,
) -> WorkerExit {
    use grafton_ndi::{NDI, PixelFormat, ScanType, Sender, SenderOptions, VideoFrame};

    let ndi = match NDI::new() {
        Ok(ndi) => ndi,
        Err(error) => {
            let mapped = map_runtime_error(error.to_string());
            shared
                .metrics
                .start_failures
                .fetch_add(1, Ordering::Relaxed);
            shared.replace_status_error(&mapped);
            return WorkerExit::StartupFailed;
        }
    };
    let options = SenderOptions::builder(config.source_name.clone())
        .clock_video(true)
        .clock_audio(false)
        .build();
    let sender = match Sender::new(&ndi, &options) {
        Ok(sender) => sender,
        Err(_error) => {
            let mapped = NdiOutputError::new(
                NdiErrorCode::SenderCreateFailed,
                "could not create NDI sender",
            );
            shared
                .metrics
                .start_failures
                .fetch_add(1, Ordering::Relaxed);
            shared.replace_status_error(&mapped);
            return WorkerExit::StartupFailed;
        }
    };
    shared.set_status(NdiOutputStatus::Live {
        connections: 0,
        source_name: config.source_name.clone(),
    });
    let mut last_connection_poll = std::time::Instant::now();
    loop {
        let Some(frame) = mailbox.take(|| stop.is_stopped()) else {
            return WorkerExit::Stopped;
        };
        let mapping = match map_video_frame(&frame, config.profile) {
            Ok(mapping) => mapping,
            Err(_) => {
                shared
                    .metrics
                    .rejected_frames
                    .fetch_add(1, Ordering::Relaxed);
                continue;
            }
        };
        let mut ndi_frame = match VideoFrame::builder()
            .resolution(mapping.width, mapping.height)
            .pixel_format(PixelFormat::BGRA)
            .frame_rate(mapping.frame_rate_n, mapping.frame_rate_d)
            .aspect_ratio(mapping.picture_aspect_ratio)
            .scan_type(ScanType::Progressive)
            .build()
        {
            Ok(frame) => frame,
            Err(_error) => {
                let mapped = NdiOutputError::new(
                    NdiErrorCode::SendFailed,
                    "NDI rejected the validated BGRA frame",
                );
                shared.replace_status_error(&mapped);
                return WorkerExit::StartupFailed;
            }
        };
        if ndi_frame.replace_data(frame.data.to_vec()).is_err() {
            let error = NdiOutputError::new(
                NdiErrorCode::SendFailed,
                "NDI frame storage rejected the validated BGRA frame",
            );
            shared.replace_status_error(&error);
            return WorkerExit::StartupFailed;
        }
        sender.send_video(&ndi_frame);
        shared.metrics.sent_frames.fetch_add(1, Ordering::Relaxed);
        shared
            .metrics
            .last_frame_seq
            .store(frame.frame_seq.0, Ordering::Relaxed);
        if last_connection_poll.elapsed() >= std::time::Duration::from_millis(500) {
            if let Ok(connections) = sender.connection_count(std::time::Duration::from_millis(10)) {
                shared.set_status(NdiOutputStatus::Live {
                    connections,
                    source_name: config.source_name.clone(),
                });
            }
            last_connection_poll = std::time::Instant::now();
        }
    }
}

#[cfg(feature = "ndi-sdk")]
fn map_runtime_error(message: String) -> NdiOutputError {
    let lower = message.to_ascii_lowercase();
    let code = if lower.contains("not found")
        || lower.contains("load")
        || lower.contains("library")
        || lower.contains("dll")
    {
        NdiErrorCode::RuntimeNotFound
    } else {
        NdiErrorCode::RuntimeInitFailed
    };
    NdiOutputError::new(code, "NDI runtime could not be initialized")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Instant;

    fn frame(seq: u64) -> VideoOutputFrame {
        VideoOutputFrame::new_bgra8(
            2,
            1,
            FrameSeq(seq),
            vtuber_core::MonoTimeNs(seq),
            vec![1, 2, 3, 4, 5, 6, 7, 8],
        )
        .expect("test frame has a valid packed shape")
    }

    #[test]
    fn mapping_preserves_bgra_alpha_and_profile_fields() {
        let source = frame(7);
        let source_bytes = source.data.clone();
        let mapping = map_video_frame(
            &source,
            VideoOutputProfile {
                width: 2,
                height: 1,
                fps: 60,
                pixel_format: VideoOutputPixelFormat::Bgra8StraightAlpha,
            },
        )
        .expect("valid frame maps");
        assert_eq!(mapping.four_cc, NdiFourCc::Bgra);
        assert_eq!(mapping.stride_bytes, 8);
        assert_eq!(mapping.frame_rate_n, 60);
        assert_eq!(mapping.frame_rate_d, 1);
        assert_eq!(mapping.picture_aspect_ratio, 2.0);
        assert_eq!(source.data, source_bytes);
    }

    #[test]
    fn malformed_frame_is_rejected_before_mapping() {
        let mut invalid = frame(1);
        invalid.stride_bytes = 4;
        let error = map_video_frame(
            &invalid,
            VideoOutputProfile {
                width: 2,
                height: 1,
                fps: 60,
                pixel_format: VideoOutputPixelFormat::Bgra8StraightAlpha,
            },
        )
        .expect_err("wrong stride must be rejected");
        assert_eq!(error.code, NdiErrorCode::InvalidFrame);
    }

    #[test]
    fn mailbox_replaces_old_frame_and_stays_capacity_one() {
        let mailbox = LatestFrameMailbox::new();
        assert_eq!(mailbox.submit(frame(1)), NdiSubmitResult::Submitted);
        assert_eq!(mailbox.submit(frame(2)), NdiSubmitResult::Replaced);
        assert_eq!(
            mailbox.take(|| false).expect("latest frame").frame_seq,
            FrameSeq(2)
        );
        assert!(mailbox.take(|| true).is_none());
    }

    #[test]
    fn closed_mailbox_releases_waiting_consumer() {
        let mailbox = Arc::new(LatestFrameMailbox::new());
        let worker_mailbox = Arc::clone(&mailbox);
        let started = std::thread::spawn(move || worker_mailbox.take(|| false));
        std::thread::sleep(std::time::Duration::from_millis(10));
        assert!(!started.is_finished());
        mailbox.close();
        assert!(started.join().expect("consumer joined").is_none());
    }

    #[test]
    fn burst_submission_remains_bounded_and_non_blocking() {
        let mailbox = LatestFrameMailbox::new();
        let begin = Instant::now();
        for sequence in 0..1000 {
            let _ = mailbox.submit(frame(sequence));
        }
        assert!(begin.elapsed() < std::time::Duration::from_secs(1));
        assert_eq!(
            mailbox.take(|| false).expect("latest frame").frame_seq,
            FrameSeq(999)
        );
    }

    #[test]
    fn slow_consumer_does_not_block_producer() {
        let mailbox = Arc::new(LatestFrameMailbox::new());
        let consumer_mailbox = Arc::clone(&mailbox);
        let consumer_ready = Arc::new(std::sync::Barrier::new(2));
        let consumer_took_frame = Arc::new(std::sync::Barrier::new(2));
        let consumer_ready_thread = Arc::clone(&consumer_ready);
        let consumer_took_frame_thread = Arc::clone(&consumer_took_frame);
        let consumer = std::thread::spawn(move || {
            consumer_ready_thread.wait();
            let frame = consumer_mailbox.take(|| false);
            consumer_took_frame_thread.wait();
            std::thread::sleep(std::time::Duration::from_millis(20));
            frame
        });
        consumer_ready.wait();
        assert_eq!(mailbox.submit(frame(0)), NdiSubmitResult::Submitted);
        consumer_took_frame.wait();
        let begin = Instant::now();
        for sequence in 1..1000 {
            let _ = mailbox.submit(frame(sequence));
        }
        assert!(begin.elapsed() < std::time::Duration::from_secs(1));
        assert!(consumer.join().expect("slow consumer joined").is_some());
        mailbox.close();
    }

    #[test]
    fn controller_is_off_without_start() {
        let controller = NdiOutputController::new();
        assert_eq!(controller.status(), NdiOutputStatus::Off);
        assert_eq!(controller.metrics(), NdiOutputMetrics::default());
        assert_eq!(
            controller.submit_frame(frame(1)),
            NdiSubmitResult::RejectedNotRunning
        );
    }

    #[cfg(not(feature = "ndi-sdk"))]
    #[test]
    fn feature_off_start_is_typed_error_and_stop_is_idempotent() {
        let mut controller = NdiOutputController::new();
        let error = controller
            .start(NdiOutputConfig::default())
            .expect_err("feature is off");
        assert_eq!(error.code, NdiErrorCode::FeatureDisabled);
        assert!(matches!(
            controller.status(),
            NdiOutputStatus::Error {
                code: NdiErrorCode::FeatureDisabled,
                ..
            }
        ));
        controller.stop().expect("stop is idempotent");
        assert_eq!(controller.status(), NdiOutputStatus::Off);
    }
}

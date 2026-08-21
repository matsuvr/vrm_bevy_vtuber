//! Engine-independent face-retargeting mode and readiness contracts.
//!
//! The Direct MediaPipe path remains the safe default.  The experimental GNM
//! path may become authoritative only after its calibration, decoder, and
//! avatar capability gates are all satisfied.

/// The user-selectable face retargeting implementation.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum FaceRetargetingMode {
    /// Existing MediaPipe blendshape-to-coarse-expression path.
    #[default]
    DirectMediaPipe,
    /// Experimental GNM Head v3 to ARKit52 path.
    GnmPerfectSync,
}

/// Readiness of the experimental GNM path.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum GnmReadiness {
    /// No validated GNM model is available.
    #[default]
    Unavailable,
    /// A model is present, but neutral identity calibration is incomplete.
    NeedsCalibration,
    /// Calibration is complete and the decoder is still being learned.
    LearningDecoder,
    /// All runtime gates passed.
    Ready,
    /// A runtime quality gate failed without invalidating the application.
    Degraded,
    /// A recoverable GNM error is being presented to the user.
    Error,
}

impl GnmReadiness {
    /// Returns whether the GNM path is allowed to become authoritative.
    #[must_use]
    pub const fn is_ready(self) -> bool {
        matches!(self, Self::Ready)
    }

    const fn fallback_reason(self) -> GnmFallbackReason {
        match self {
            Self::Unavailable => GnmFallbackReason::Unavailable,
            Self::NeedsCalibration => GnmFallbackReason::NeedsCalibration,
            Self::LearningDecoder => GnmFallbackReason::LearningDecoder,
            Self::Ready => GnmFallbackReason::NoEffectivePerfectSync,
            Self::Degraded => GnmFallbackReason::Degraded,
            Self::Error => GnmFallbackReason::Error,
        }
    }
}

/// Why a requested GNM mode is currently falling back to Direct MediaPipe.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum GnmFallbackReason {
    /// No model/runtime is available.
    Unavailable,
    /// Neutral identity calibration has not completed.
    NeedsCalibration,
    /// The decoder has not passed its learning gate.
    LearningDecoder,
    /// Runtime quality is below the activation threshold.
    Degraded,
    /// A recoverable GNM error occurred.
    Error,
    /// The avatar has no effective Perfect Sync channel.
    NoEffectivePerfectSync,
    /// The latest GNM frame was not valid for the current source sequence.
    InvalidFrame,
}

/// Small capability/readiness snapshot shared by orchestration, UI, and the
/// avatar bridge.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FaceRetargetingStatus {
    /// Mode requested by the user.
    pub requested_mode: FaceRetargetingMode,
    /// Mode that is authoritative for the next control frame.
    pub active_mode: FaceRetargetingMode,
    /// Current experimental GNM readiness.
    pub gnm_readiness: GnmReadiness,
    /// Number of Perfect Sync channels present on the active model.
    pub perfect_sync_present_channels: u16,
    /// Number of present Perfect Sync channels with an effective morph bind.
    pub perfect_sync_effective_channels: u16,
    /// Number of decoder channels that passed the reliability gate.
    pub reliable_decoder_channels: u16,
    /// Current fallback, if the requested mode is not authoritative.
    pub fallback: Option<GnmFallbackReason>,
}

impl Default for FaceRetargetingStatus {
    fn default() -> Self {
        Self {
            requested_mode: FaceRetargetingMode::DirectMediaPipe,
            active_mode: FaceRetargetingMode::DirectMediaPipe,
            gnm_readiness: GnmReadiness::Unavailable,
            perfect_sync_present_channels: 0,
            perfect_sync_effective_channels: 0,
            reliable_decoder_channels: 0,
            fallback: None,
        }
    }
}

impl FaceRetargetingStatus {
    /// Requests a mode and immediately recomputes the safe authority.
    pub fn request_mode(&mut self, mode: FaceRetargetingMode) {
        self.requested_mode = mode;
        self.recompute_authority();
    }

    /// Publishes GNM readiness and recomputes the safe authority.
    pub fn set_gnm_readiness(&mut self, readiness: GnmReadiness) {
        self.gnm_readiness = readiness;
        self.recompute_authority();
    }

    /// Publishes active-avatar Perfect Sync capability counts.
    pub fn set_perfect_sync_capability(
        &mut self,
        present_channels: usize,
        effective_channels: usize,
    ) {
        self.perfect_sync_present_channels = present_channels.min(u16::MAX as usize) as u16;
        self.perfect_sync_effective_channels = effective_channels.min(u16::MAX as usize) as u16;
        self.recompute_authority();
    }

    /// Publishes the decoder's reliable output-channel count.
    pub fn set_reliable_decoder_channels(&mut self, reliable_channels: usize) {
        self.reliable_decoder_channels = reliable_channels.min(u16::MAX as usize) as u16;
        self.recompute_authority();
    }

    /// Returns whether GNM may authoritatively populate detailed face values.
    #[must_use]
    pub const fn uses_gnm_authority(&self) -> bool {
        matches!(self.active_mode, FaceRetargetingMode::GnmPerfectSync)
    }

    /// Marks the current frame invalid without changing the requested mode.
    /// The next valid ready frame may recover automatically.
    pub fn mark_invalid_frame(&mut self) {
        if self.requested_mode == FaceRetargetingMode::GnmPerfectSync {
            self.active_mode = FaceRetargetingMode::DirectMediaPipe;
            self.fallback = Some(GnmFallbackReason::InvalidFrame);
        }
    }

    fn recompute_authority(&mut self) {
        if self.requested_mode == FaceRetargetingMode::DirectMediaPipe {
            self.active_mode = FaceRetargetingMode::DirectMediaPipe;
            self.fallback = None;
            return;
        }
        if !self.gnm_readiness.is_ready() {
            self.active_mode = FaceRetargetingMode::DirectMediaPipe;
            self.fallback = Some(self.gnm_readiness.fallback_reason());
            return;
        }
        if self.perfect_sync_present_channels == 0 || self.perfect_sync_effective_channels == 0 {
            self.active_mode = FaceRetargetingMode::DirectMediaPipe;
            self.fallback = Some(GnmFallbackReason::NoEffectivePerfectSync);
            return;
        }
        self.active_mode = FaceRetargetingMode::GnmPerfectSync;
        self.fallback = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_is_direct_and_does_not_require_gnm_work() {
        let status = FaceRetargetingStatus::default();
        assert_eq!(status.requested_mode, FaceRetargetingMode::DirectMediaPipe);
        assert_eq!(status.active_mode, FaceRetargetingMode::DirectMediaPipe);
        assert!(!status.uses_gnm_authority());
    }

    #[test]
    fn unready_gnm_request_falls_back_without_freezing_tracking() {
        let mut status = FaceRetargetingStatus::default();
        status.request_mode(FaceRetargetingMode::GnmPerfectSync);
        assert_eq!(status.active_mode, FaceRetargetingMode::DirectMediaPipe);
        assert_eq!(status.fallback, Some(GnmFallbackReason::Unavailable));
    }

    #[test]
    fn ready_partial_capability_can_activate_gnm() {
        let mut status = FaceRetargetingStatus::default();
        status.request_mode(FaceRetargetingMode::GnmPerfectSync);
        status.set_perfect_sync_capability(52, 17);
        status.set_reliable_decoder_channels(17);
        status.set_gnm_readiness(GnmReadiness::Ready);
        assert!(status.uses_gnm_authority());
        assert_eq!(status.perfect_sync_effective_channels, 17);
    }

    #[test]
    fn ready_without_effective_capability_stays_direct() {
        let mut status = FaceRetargetingStatus::default();
        status.request_mode(FaceRetargetingMode::GnmPerfectSync);
        status.set_gnm_readiness(GnmReadiness::Ready);
        assert!(!status.uses_gnm_authority());
        assert_eq!(
            status.fallback,
            Some(GnmFallbackReason::NoEffectivePerfectSync)
        );
    }

    #[test]
    fn invalid_frame_only_drops_current_authority() {
        let mut status = FaceRetargetingStatus::default();
        status.request_mode(FaceRetargetingMode::GnmPerfectSync);
        status.set_perfect_sync_capability(52, 52);
        status.set_gnm_readiness(GnmReadiness::Ready);
        status.mark_invalid_frame();
        assert!(!status.uses_gnm_authority());
        assert_eq!(status.fallback, Some(GnmFallbackReason::InvalidFrame));
    }
}

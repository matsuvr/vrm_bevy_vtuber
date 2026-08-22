//! Pure neutral-calibration selection and immutable identity output contract.
//!
//! This module does not implement the numerical multi-frame identity solve. It
//! establishes the parts that are independent of that solver: sample admission,
//! pose-diversity diagnostics, model/mapping version binding, and a structurally
//! read-only identity object for later tracking.

use crate::{
    DenseCoverageSummary, DenseMappingVersion, DenseObservationStatus, GnmExpressionState,
    GnmIdentityState, GnmModel, GnmVersion,
};

/// Summary of one neutral-window candidate used for deterministic sample selection.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct NeutralCalibrationCandidate {
    /// Monotonic source-frame sequence.
    pub source_seq: u64,
    /// Monotonic capture timestamp in microseconds.
    pub captured_at_micros: u64,
    /// Dense observation coverage from Issue #53.
    pub coverage: DenseCoverageSummary,
    /// Candidate reprojection RMS in normalized image coordinates.
    pub reprojection_rms: f32,
    /// Optional normalized expression-activity proxy in `[0, 1]`.
    /// Absence means unavailable, not neutral and not expressive.
    pub expression_activity: Option<f32>,
    /// Pose nuisance estimate used only for diversity diagnostics.
    pub yaw_radians: f32,
    /// Pose nuisance estimate used only for diversity diagnostics.
    pub pitch_radians: f32,
    /// Whether upstream tracking marked this candidate degraded/lost.
    pub tracking_degraded: bool,
}

/// Typed reason a neutral calibration candidate was not admitted.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum NeutralCalibrationRejectionReason {
    /// Source sequence duplicated an earlier candidate.
    DuplicateSourceSequence,
    /// Source sequence regressed.
    RegressedSourceSequence,
    /// Capture timestamp failed strict monotonicity.
    RegressedTimestamp,
    /// Dense observation coverage was insufficient.
    InsufficientDenseCoverage,
    /// Upstream lifecycle marked the sample degraded/lost.
    DegradedTracking,
    /// One or more candidate metrics were non-finite or inconsistent.
    InvalidMetrics,
    /// Reprojection residual exceeded the configured bound.
    ExcessiveReprojectionResidual,
    /// Optional expression-activity proxy exceeded the configured neutral-window bound.
    ExpressionContamination,
}

/// Rejection record retaining the candidate index and source identity.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct NeutralCalibrationRejection {
    /// Index in the input candidate slice.
    pub candidate_index: usize,
    /// Candidate source sequence.
    pub source_seq: u64,
    /// Typed rejection reason.
    pub reason: NeutralCalibrationRejectionReason,
}

/// Typed readiness of the selected calibration window before numerical identity fitting.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum NeutralCalibrationReadiness {
    /// Too few accepted candidates remain.
    InsufficientSamples,
    /// Accepted samples are too near-identical in yaw/pitch to claim useful diversity.
    InsufficientPoseDiversity,
    /// Selection gates pass and the accepted dense observations may enter the identity solver.
    ReadyForIdentitySolve,
}

/// Pose-diversity diagnostics over accepted candidates.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct NeutralPoseDiversity {
    /// Accepted yaw span in radians.
    pub yaw_span_radians: f32,
    /// Accepted pitch span in radians.
    pub pitch_span_radians: f32,
    /// Fraction of accepted samples after the first that are near-duplicates of
    /// the previous accepted yaw/pitch estimate.
    pub near_duplicate_fraction: f32,
}

/// Aggregate pre-solve diagnostics for a neutral candidate window.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct NeutralCalibrationWindowDiagnostics {
    /// Total candidates examined.
    pub total_candidates: usize,
    /// Candidates admitted to the numerical identity solve.
    pub accepted_candidates: usize,
    /// Candidates rejected by typed gates.
    pub rejected_candidates: usize,
    /// Pose diversity over accepted candidates.
    pub pose_diversity: NeutralPoseDiversity,
    /// Readiness after count/diversity checks.
    pub readiness: NeutralCalibrationReadiness,
}

/// Deterministic selection result; accepted indices point back to caller-owned dense observations.
#[derive(Clone, Debug, PartialEq)]
pub struct NeutralCalibrationSelection {
    /// Input indices accepted for the future shared-identity solve.
    pub accepted_indices: Vec<usize>,
    /// Typed rejection records.
    pub rejections: Vec<NeutralCalibrationRejection>,
    /// Aggregate window diagnostics.
    pub diagnostics: NeutralCalibrationWindowDiagnostics,
}

/// Typed thresholds for neutral candidate selection and pose-diversity diagnostics.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct NeutralCalibrationSelectionConfig {
    min_accepted_samples: usize,
    max_reprojection_rms: f32,
    max_expression_activity: f32,
    min_pose_span_radians: f32,
    near_duplicate_pose_distance_radians: f32,
    max_near_duplicate_fraction: f32,
}

impl NeutralCalibrationSelectionConfig {
    /// Creates a selection configuration without hidden thresholds.
    pub fn new(
        min_accepted_samples: usize,
        max_reprojection_rms: f32,
        max_expression_activity: f32,
        min_pose_span_radians: f32,
        near_duplicate_pose_distance_radians: f32,
        max_near_duplicate_fraction: f32,
    ) -> Result<Self, GnmIdentityCalibrationError> {
        if min_accepted_samples == 0 {
            return Err(GnmIdentityCalibrationError::InvalidSelectionConfig(
                "min_accepted_samples must be positive",
            ));
        }
        for (field, value) in [
            ("max_reprojection_rms", max_reprojection_rms),
            ("min_pose_span_radians", min_pose_span_radians),
            (
                "near_duplicate_pose_distance_radians",
                near_duplicate_pose_distance_radians,
            ),
        ] {
            if !value.is_finite() || value < 0.0 {
                return Err(GnmIdentityCalibrationError::InvalidSelectionConfig(field));
            }
        }
        if !max_expression_activity.is_finite() || !(0.0..=1.0).contains(&max_expression_activity) {
            return Err(GnmIdentityCalibrationError::InvalidSelectionConfig(
                "max_expression_activity must be within [0, 1]",
            ));
        }
        if !max_near_duplicate_fraction.is_finite()
            || !(0.0..=1.0).contains(&max_near_duplicate_fraction)
        {
            return Err(GnmIdentityCalibrationError::InvalidSelectionConfig(
                "max_near_duplicate_fraction must be within [0, 1]",
            ));
        }
        Ok(Self {
            min_accepted_samples,
            max_reprojection_rms,
            max_expression_activity,
            min_pose_span_radians,
            near_duplicate_pose_distance_radians,
            max_near_duplicate_fraction,
        })
    }
}

/// Selects neutral calibration samples without consulting MediaPipe blendshapes as authority.
pub fn select_neutral_calibration_candidates(
    candidates: &[NeutralCalibrationCandidate],
    config: NeutralCalibrationSelectionConfig,
) -> NeutralCalibrationSelection {
    let mut accepted_indices = Vec::new();
    let mut rejections = Vec::new();
    let mut last_seen: Option<(u64, u64)> = None;

    for (candidate_index, candidate) in candidates.iter().enumerate() {
        let reason = sequence_rejection(last_seen, *candidate)
            .or_else(|| candidate_metric_rejection(*candidate, config));
        last_seen = Some((candidate.source_seq, candidate.captured_at_micros));
        if let Some(reason) = reason {
            rejections.push(NeutralCalibrationRejection {
                candidate_index,
                source_seq: candidate.source_seq,
                reason,
            });
        } else {
            accepted_indices.push(candidate_index);
        }
    }

    let pose_diversity = pose_diversity(candidates, &accepted_indices, config);
    let readiness = if accepted_indices.len() < config.min_accepted_samples {
        NeutralCalibrationReadiness::InsufficientSamples
    } else if pose_diversity
        .yaw_span_radians
        .max(pose_diversity.pitch_span_radians)
        < config.min_pose_span_radians
        || pose_diversity.near_duplicate_fraction > config.max_near_duplicate_fraction
    {
        NeutralCalibrationReadiness::InsufficientPoseDiversity
    } else {
        NeutralCalibrationReadiness::ReadyForIdentitySolve
    };

    NeutralCalibrationSelection {
        diagnostics: NeutralCalibrationWindowDiagnostics {
            total_candidates: candidates.len(),
            accepted_candidates: accepted_indices.len(),
            rejected_candidates: rejections.len(),
            pose_diversity,
            readiness,
        },
        accepted_indices,
        rejections,
    }
}

fn sequence_rejection(
    previous: Option<(u64, u64)>,
    candidate: NeutralCalibrationCandidate,
) -> Option<NeutralCalibrationRejectionReason> {
    let (previous_seq, previous_timestamp) = previous?;
    if candidate.source_seq == previous_seq {
        Some(NeutralCalibrationRejectionReason::DuplicateSourceSequence)
    } else if candidate.source_seq < previous_seq {
        Some(NeutralCalibrationRejectionReason::RegressedSourceSequence)
    } else if candidate.captured_at_micros <= previous_timestamp {
        Some(NeutralCalibrationRejectionReason::RegressedTimestamp)
    } else {
        None
    }
}

fn candidate_metric_rejection(
    candidate: NeutralCalibrationCandidate,
    config: NeutralCalibrationSelectionConfig,
) -> Option<NeutralCalibrationRejectionReason> {
    if candidate.coverage.mapped_points == 0
        || candidate.coverage.valid_points > candidate.coverage.mapped_points
        || !candidate.coverage.effective_weight.is_finite()
        || candidate.coverage.effective_weight < 0.0
        || !candidate.reprojection_rms.is_finite()
        || candidate.reprojection_rms < 0.0
        || !candidate.yaw_radians.is_finite()
        || !candidate.pitch_radians.is_finite()
        || candidate
            .expression_activity
            .is_some_and(|value| !value.is_finite() || !(0.0..=1.0).contains(&value))
    {
        return Some(NeutralCalibrationRejectionReason::InvalidMetrics);
    }
    if candidate.coverage.status == DenseObservationStatus::Insufficient {
        return Some(NeutralCalibrationRejectionReason::InsufficientDenseCoverage);
    }
    if candidate.tracking_degraded {
        return Some(NeutralCalibrationRejectionReason::DegradedTracking);
    }
    if candidate.reprojection_rms > config.max_reprojection_rms {
        return Some(NeutralCalibrationRejectionReason::ExcessiveReprojectionResidual);
    }
    if candidate
        .expression_activity
        .is_some_and(|activity| activity > config.max_expression_activity)
    {
        return Some(NeutralCalibrationRejectionReason::ExpressionContamination);
    }
    None
}

fn pose_diversity(
    candidates: &[NeutralCalibrationCandidate],
    accepted_indices: &[usize],
    config: NeutralCalibrationSelectionConfig,
) -> NeutralPoseDiversity {
    if accepted_indices.is_empty() {
        return NeutralPoseDiversity {
            yaw_span_radians: 0.0,
            pitch_span_radians: 0.0,
            near_duplicate_fraction: 1.0,
        };
    }

    let first = candidates[accepted_indices[0]];
    let mut yaw_min = first.yaw_radians;
    let mut yaw_max = first.yaw_radians;
    let mut pitch_min = first.pitch_radians;
    let mut pitch_max = first.pitch_radians;
    let mut near_duplicates = 0usize;

    for pair in accepted_indices.windows(2) {
        let previous = candidates[pair[0]];
        let current = candidates[pair[1]];
        let dyaw = current.yaw_radians - previous.yaw_radians;
        let dpitch = current.pitch_radians - previous.pitch_radians;
        let distance = (dyaw * dyaw + dpitch * dpitch).sqrt();
        if distance <= config.near_duplicate_pose_distance_radians {
            near_duplicates += 1;
        }
    }
    for index in accepted_indices.iter().copied() {
        let candidate = candidates[index];
        yaw_min = yaw_min.min(candidate.yaw_radians);
        yaw_max = yaw_max.max(candidate.yaw_radians);
        pitch_min = pitch_min.min(candidate.pitch_radians);
        pitch_max = pitch_max.max(candidate.pitch_radians);
    }

    let comparisons = accepted_indices.len().saturating_sub(1);
    NeutralPoseDiversity {
        yaw_span_radians: yaw_max - yaw_min,
        pitch_span_radians: pitch_max - pitch_min,
        near_duplicate_fraction: if comparisons == 0 {
            1.0
        } else {
            near_duplicates as f32 / comparisons as f32
        },
    }
}

/// Semantically fixed GNM identity supplied read-only to tracking.
#[derive(Clone, Debug, PartialEq)]
pub struct FixedGnmIdentity(GnmIdentityState);

impl FixedGnmIdentity {
    /// Wraps a validated identity whose dimension matches the loaded model.
    pub fn new(
        identity: GnmIdentityState,
        model: &GnmModel,
    ) -> Result<Self, GnmIdentityCalibrationError> {
        if identity.values().len() != model.identity_dimension() {
            return Err(GnmIdentityCalibrationError::IdentityDimensionMismatch {
                expected: model.identity_dimension(),
                actual: identity.values().len(),
            });
        }
        Ok(Self(identity))
    }

    /// Returns the immutable model identity state.
    pub fn state(&self) -> &GnmIdentityState {
        &self.0
    }

    /// Returns coefficients as a read-only slice.
    pub fn values(&self) -> &[f32] {
        self.0.values()
    }
}

/// Optional person-specific neutral geometry scales for later projectors.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct NeutralNormalizationScales {
    /// Inter-ocular distance in model-space units when measured.
    pub inter_ocular: Option<f32>,
    /// Neutral mouth width in model-space units when measured.
    pub mouth_width: Option<f32>,
    /// Neutral eye aperture in model-space units when measured.
    pub eye_aperture: Option<f32>,
}

impl NeutralNormalizationScales {
    fn validate(self) -> Result<(), GnmIdentityCalibrationError> {
        for (field, value) in [
            ("inter_ocular", self.inter_ocular),
            ("mouth_width", self.mouth_width),
            ("eye_aperture", self.eye_aperture),
        ] {
            if value.is_some_and(|value| !value.is_finite() || value <= 0.0) {
                return Err(GnmIdentityCalibrationError::InvalidOutput {
                    field,
                    reason: "normalization scale must be finite and positive when available",
                });
            }
        }
        Ok(())
    }
}

/// Diagnostics produced by a future numerical identity solve.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct IdentityFitDiagnostics {
    /// Accepted dense sample count used by the shared-identity solve.
    pub accepted_samples: usize,
    /// Rejected candidate count.
    pub rejected_samples: usize,
    /// Final aggregate dense reprojection RMS.
    pub reprojection_rms: f32,
    /// Number of identity dimensions actively solved/retained.
    pub active_identity_dimension: usize,
    /// Optional conditioning estimate. Absence means not measured, not well-conditioned.
    pub condition_number: Option<f64>,
    /// Pose-diversity summary of the selected window.
    pub pose_diversity: NeutralPoseDiversity,
}

/// Immutable identity calibration handed to later tracking/projector stages.
#[derive(Clone, Debug, PartialEq)]
pub struct GnmIdentityCalibration {
    model_version: GnmVersion,
    mapping_version: DenseMappingVersion,
    identity: FixedGnmIdentity,
    neutral_expression_reference: GnmExpressionState,
    neutral_surface_reference: Vec<[f32; 3]>,
    normalization_scales: NeutralNormalizationScales,
    diagnostics: IdentityFitDiagnostics,
}

impl GnmIdentityCalibration {
    /// Builds a version-bound, finite calibration object from numerical-solver output.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        model: &GnmModel,
        mapping_version: DenseMappingVersion,
        identity: FixedGnmIdentity,
        neutral_expression_reference: GnmExpressionState,
        neutral_surface_reference: Vec<[f32; 3]>,
        normalization_scales: NeutralNormalizationScales,
        diagnostics: IdentityFitDiagnostics,
    ) -> Result<Self, GnmIdentityCalibrationError> {
        if mapping_version.model_version != model.version() {
            return Err(GnmIdentityCalibrationError::VersionMismatch {
                calibration_model: mapping_version.model_version,
                runtime_model: model.version(),
            });
        }
        if identity.values().len() != model.identity_dimension() {
            return Err(GnmIdentityCalibrationError::IdentityDimensionMismatch {
                expected: model.identity_dimension(),
                actual: identity.values().len(),
            });
        }
        if neutral_expression_reference.values().len() != model.expression_dimension() {
            return Err(GnmIdentityCalibrationError::ExpressionDimensionMismatch {
                expected: model.expression_dimension(),
                actual: neutral_expression_reference.values().len(),
            });
        }
        if neutral_surface_reference.is_empty()
            || neutral_surface_reference
                .iter()
                .flatten()
                .any(|value| !value.is_finite())
        {
            return Err(GnmIdentityCalibrationError::InvalidOutput {
                field: "neutral_surface_reference",
                reason: "neutral surface reference must be non-empty and finite",
            });
        }
        normalization_scales.validate()?;
        if !diagnostics.reprojection_rms.is_finite() || diagnostics.reprojection_rms < 0.0 {
            return Err(GnmIdentityCalibrationError::InvalidOutput {
                field: "reprojection_rms",
                reason: "must be finite and non-negative",
            });
        }
        if diagnostics.active_identity_dimension == 0
            || diagnostics.active_identity_dimension > model.identity_dimension()
        {
            return Err(GnmIdentityCalibrationError::InvalidOutput {
                field: "active_identity_dimension",
                reason: "must be within the loaded model identity dimension",
            });
        }
        if diagnostics
            .condition_number
            .is_some_and(|value| !value.is_finite() || value <= 0.0)
        {
            return Err(GnmIdentityCalibrationError::InvalidOutput {
                field: "condition_number",
                reason: "must be finite and positive when measured",
            });
        }
        for (field, value) in [
            ("pose yaw span", diagnostics.pose_diversity.yaw_span_radians),
            (
                "pose pitch span",
                diagnostics.pose_diversity.pitch_span_radians,
            ),
            (
                "near duplicate fraction",
                diagnostics.pose_diversity.near_duplicate_fraction,
            ),
        ] {
            if !value.is_finite() || value < 0.0 {
                return Err(GnmIdentityCalibrationError::InvalidOutput {
                    field,
                    reason: "pose diagnostic must be finite and non-negative",
                });
            }
        }

        Ok(Self {
            model_version: model.version(),
            mapping_version,
            identity,
            neutral_expression_reference,
            neutral_surface_reference,
            normalization_scales,
            diagnostics,
        })
    }

    /// Returns the loaded GNM model version to which this calibration is bound.
    pub fn model_version(&self) -> GnmVersion {
        self.model_version
    }

    /// Returns the exact dense mapping version used for calibration.
    pub fn mapping_version(&self) -> DenseMappingVersion {
        self.mapping_version
    }

    /// Returns the fixed identity through a read-only reference.
    pub fn identity(&self) -> &FixedGnmIdentity {
        &self.identity
    }

    /// Returns the neutral expression reference through a read-only reference.
    pub fn neutral_expression_reference(&self) -> &GnmExpressionState {
        &self.neutral_expression_reference
    }

    /// Returns the neutral selected-surface geometry through a read-only slice.
    pub fn neutral_surface_reference(&self) -> &[[f32; 3]] {
        &self.neutral_surface_reference
    }

    /// Returns optional normalization scales.
    pub fn normalization_scales(&self) -> NeutralNormalizationScales {
        self.normalization_scales
    }

    /// Returns numerical calibration diagnostics.
    pub fn diagnostics(&self) -> IdentityFitDiagnostics {
        self.diagnostics
    }

    /// Returns whether the calibration exactly matches the runtime model/mapping boundary.
    pub fn matches_runtime(&self, model: &GnmModel, mapping: DenseMappingVersion) -> bool {
        self.model_version == model.version() && self.mapping_version == mapping
    }
}

/// Typed error from neutral selection or immutable calibration validation.
#[derive(Clone, Debug, PartialEq)]
pub enum GnmIdentityCalibrationError {
    /// Candidate selection configuration is invalid.
    InvalidSelectionConfig(&'static str),
    /// Fixed identity dimension differs from the loaded model.
    IdentityDimensionMismatch {
        /// Expected loaded-model dimension.
        expected: usize,
        /// Actual coefficient count.
        actual: usize,
    },
    /// Neutral-expression dimension differs from the loaded model.
    ExpressionDimensionMismatch {
        /// Expected loaded-model dimension.
        expected: usize,
        /// Actual coefficient count.
        actual: usize,
    },
    /// Mapping/model versions differ.
    VersionMismatch {
        /// Model version recorded by mapping/calibration.
        calibration_model: GnmVersion,
        /// Currently loaded model version.
        runtime_model: GnmVersion,
    },
    /// Numerical calibration output is invalid/non-finite.
    InvalidOutput {
        /// Invalid output field.
        field: &'static str,
        /// Validation reason.
        reason: &'static str,
    },
}

impl std::fmt::Display for GnmIdentityCalibrationError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidSelectionConfig(reason) => {
                write!(formatter, "invalid neutral selection config: {reason}")
            }
            Self::IdentityDimensionMismatch { expected, actual } => write!(
                formatter,
                "GNM identity dimension mismatch: expected {expected}, got {actual}"
            ),
            Self::ExpressionDimensionMismatch { expected, actual } => write!(
                formatter,
                "GNM neutral-expression dimension mismatch: expected {expected}, got {actual}"
            ),
            Self::VersionMismatch {
                calibration_model,
                runtime_model,
            } => write!(
                formatter,
                "GNM calibration model {}.{} does not match runtime {}.{}",
                calibration_model.major,
                calibration_model.minor,
                runtime_model.major,
                runtime_model.minor
            ),
            Self::InvalidOutput { field, reason } => {
                write!(
                    formatter,
                    "invalid GNM identity calibration {field}: {reason}"
                )
            }
        }
    }
}

impl std::error::Error for GnmIdentityCalibrationError {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        DenseArray, GNM_HEAD_V3_EXPRESSION_DIM, GNM_HEAD_V3_IDENTITY_DIM, GNM_HEAD_V3_VERSION,
        GnmModelData, GnmVariant,
    };

    fn selection_config() -> NeutralCalibrationSelectionConfig {
        NeutralCalibrationSelectionConfig::new(3, 0.05, 0.25, 0.10, 0.01, 0.75).unwrap()
    }

    fn candidate(seq: u64, timestamp: u64, yaw: f32, pitch: f32) -> NeutralCalibrationCandidate {
        NeutralCalibrationCandidate {
            source_seq: seq,
            captured_at_micros: timestamp,
            coverage: DenseCoverageSummary {
                mapped_points: 120,
                valid_points: 110,
                effective_weight: 100.0,
                status: DenseObservationStatus::Valid,
            },
            reprojection_rms: 0.01,
            expression_activity: None,
            yaw_radians: yaw,
            pitch_radians: pitch,
            tracking_degraded: false,
        }
    }

    fn synthetic_model() -> GnmModel {
        let identity = GNM_HEAD_V3_IDENTITY_DIM;
        let expression = GNM_HEAD_V3_EXPRESSION_DIM;
        GnmModel::from_data(GnmModelData {
            version: GNM_HEAD_V3_VERSION,
            variant: GnmVariant::Head,
            template_vertices: DenseArray::new(
                "vertices",
                vec![3, 3],
                vec![0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0, 0.0],
            )
            .unwrap(),
            template_joints: DenseArray::new("joints", vec![1, 3], vec![0.0; 3]).unwrap(),
            vertex_identity_basis: DenseArray::new(
                "identity",
                vec![identity, 3, 3],
                vec![0.0; identity * 9],
            )
            .unwrap(),
            joint_identity_basis: DenseArray::new(
                "joint_identity",
                vec![identity, 1, 3],
                vec![0.0; identity * 3],
            )
            .unwrap(),
            expression_basis: DenseArray::new(
                "expression",
                vec![expression, 3, 3],
                vec![0.0; expression * 9],
            )
            .unwrap(),
            joint_parent_indices: vec![-1],
            skinning_weights: DenseArray::new("weights", vec![1, 3], vec![1.0; 3]).unwrap(),
            pose_correctives_regressor: None,
        })
        .unwrap()
    }

    fn mapping_version() -> DenseMappingVersion {
        DenseMappingVersion {
            schema_revision: 1,
            model_version: GNM_HEAD_V3_VERSION,
        }
    }

    #[test]
    fn selection_rejects_duplicate_outlier_expression_and_degraded_candidates() {
        let good1 = candidate(1, 1_000, -0.1, 0.0);
        let duplicate = candidate(1, 1_010, -0.05, 0.0);
        let mut residual = candidate(2, 1_020, 0.0, 0.0);
        residual.reprojection_rms = 0.5;
        let mut expressive = candidate(3, 1_030, 0.05, 0.0);
        expressive.expression_activity = Some(0.9);
        let mut degraded = candidate(4, 1_040, 0.10, 0.0);
        degraded.tracking_degraded = true;
        let good2 = candidate(5, 1_050, 0.05, 0.0);
        let good3 = candidate(6, 1_060, 0.15, 0.05);
        let selection = select_neutral_calibration_candidates(
            &[
                good1, duplicate, residual, expressive, degraded, good2, good3,
            ],
            selection_config(),
        );
        assert_eq!(selection.accepted_indices, vec![0, 5, 6]);
        assert_eq!(selection.rejections.len(), 4);
        assert_eq!(
            selection.diagnostics.readiness,
            NeutralCalibrationReadiness::ReadyForIdentitySolve
        );
    }

    #[test]
    fn near_identical_window_is_not_misreported_as_ready() {
        let selection = select_neutral_calibration_candidates(
            &[
                candidate(1, 1_000, 0.0, 0.0),
                candidate(2, 1_010, 0.001, 0.001),
                candidate(3, 1_020, 0.002, 0.002),
                candidate(4, 1_030, 0.003, 0.003),
            ],
            selection_config(),
        );
        assert_eq!(
            selection.diagnostics.readiness,
            NeutralCalibrationReadiness::InsufficientPoseDiversity
        );
        assert!(selection.diagnostics.pose_diversity.near_duplicate_fraction > 0.75);
    }

    #[test]
    fn optional_expression_proxy_absence_is_not_treated_as_zero_authority() {
        let selection = select_neutral_calibration_candidates(
            &[
                candidate(1, 1_000, -0.1, 0.0),
                candidate(2, 1_010, 0.0, 0.0),
                candidate(3, 1_020, 0.1, 0.0),
            ],
            selection_config(),
        );
        assert_eq!(selection.accepted_indices.len(), 3);
    }

    #[test]
    fn fixed_identity_and_calibration_are_version_bound_and_read_only() {
        let model = synthetic_model();
        let fixed = FixedGnmIdentity::new(model.neutral_identity(), &model).unwrap();
        let before = fixed.values().to_vec();
        let calibration = GnmIdentityCalibration::new(
            &model,
            mapping_version(),
            fixed,
            model.neutral_expression(),
            vec![[0.0, 0.0, 0.0], [1.0, 0.0, 0.0]],
            NeutralNormalizationScales {
                inter_ocular: Some(1.0),
                mouth_width: Some(0.5),
                eye_aperture: None,
            },
            IdentityFitDiagnostics {
                accepted_samples: 8,
                rejected_samples: 2,
                reprojection_rms: 0.01,
                active_identity_dimension: 32,
                condition_number: Some(12.0),
                pose_diversity: NeutralPoseDiversity {
                    yaw_span_radians: 0.2,
                    pitch_span_radians: 0.1,
                    near_duplicate_fraction: 0.1,
                },
            },
        )
        .unwrap();
        assert_eq!(calibration.identity().values(), before.as_slice());
        assert!(calibration.matches_runtime(&model, mapping_version()));
    }

    #[test]
    fn mapping_model_mismatch_rejects_old_calibration_contract() {
        let model = synthetic_model();
        let fixed = FixedGnmIdentity::new(model.neutral_identity(), &model).unwrap();
        let mismatched = DenseMappingVersion {
            schema_revision: 1,
            model_version: GnmVersion { major: 9, minor: 0 },
        };
        let result = GnmIdentityCalibration::new(
            &model,
            mismatched,
            fixed,
            model.neutral_expression(),
            vec![[0.0, 0.0, 0.0]],
            NeutralNormalizationScales::default(),
            IdentityFitDiagnostics {
                accepted_samples: 3,
                rejected_samples: 0,
                reprojection_rms: 0.01,
                active_identity_dimension: 1,
                condition_number: None,
                pose_diversity: NeutralPoseDiversity {
                    yaw_span_radians: 0.2,
                    pitch_span_radians: 0.0,
                    near_duplicate_fraction: 0.0,
                },
            },
        );
        assert!(matches!(
            result,
            Err(GnmIdentityCalibrationError::VersionMismatch { .. })
        ));
    }

    #[test]
    fn non_finite_surface_or_normalization_scale_is_rejected() {
        let model = synthetic_model();
        let fixed = FixedGnmIdentity::new(model.neutral_identity(), &model).unwrap();
        let result = GnmIdentityCalibration::new(
            &model,
            mapping_version(),
            fixed,
            model.neutral_expression(),
            vec![[f32::NAN, 0.0, 0.0]],
            NeutralNormalizationScales::default(),
            IdentityFitDiagnostics {
                accepted_samples: 3,
                rejected_samples: 0,
                reprojection_rms: 0.01,
                active_identity_dimension: 1,
                condition_number: None,
                pose_diversity: NeutralPoseDiversity {
                    yaw_span_radians: 0.2,
                    pitch_span_radians: 0.0,
                    near_duplicate_fraction: 0.0,
                },
            },
        );
        assert!(matches!(
            result,
            Err(GnmIdentityCalibrationError::InvalidOutput {
                field: "neutral_surface_reference",
                ..
            })
        ));
    }
}

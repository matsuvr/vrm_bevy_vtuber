//! Regularized GNM-expression to ARKit52 decoder.
//!
//! Training consumes synchronized numeric GNM/teacher samples. Runtime
//! decoding accepts only [`crate::GnmFaceState`]; the MediaPipe teacher is not
//! present in this API, which prevents a direct passthrough shortcut.

use std::fmt::{Display, Formatter};

use vtuber_core::{ARKIT52_CHANNEL_COUNT, Arkit52Coefficients, ArkitBlendshape};

use crate::{GnmFaceState, GnmFaceStatus, GnmVersion};

/// Number of teacher channels in the common MediaPipe/ARKit-like target.
pub const ARKIT52_TEACHER_CHANNEL_COUNT: usize = ARKIT52_CHANNEL_COUNT - 1;
/// Default ridge term for decoder training.
pub const DEFAULT_DECODER_REGULARIZATION: f32 = 1.0e-3;
/// Default number of valid training samples required before a model is built.
pub const DEFAULT_MIN_DECODER_SAMPLES: usize = 8;
/// Default minimum teacher variance for a channel to be reliable.
pub const DEFAULT_MIN_CHANNEL_VARIANCE: f32 = 1.0e-4;
/// Default minimum GNM fit confidence for a training sample.
pub const DEFAULT_MIN_TRAINING_CONFIDENCE: f32 = 0.5;
/// Default maximum GNM reprojection RMS for a training sample.
pub const DEFAULT_MAX_TRAINING_RESIDUAL: f32 = 0.05;
/// Maximum accepted decoder normal-equation condition estimate.
pub const DEFAULT_DECODER_MAX_CONDITION_NUMBER: f32 = 1.0e8;

/// Configuration for a regularized linear GNM-to-ARKit decoder.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GnmDecoderConfig {
    /// Number of leading GNM expression components used by the decoder.
    pub active_expression_dimension: usize,
    /// Ridge regularization for latent weights; bias is unregularized.
    pub regularization: f32,
    /// Minimum accepted sample count.
    pub min_training_samples: usize,
    /// Minimum teacher variance for a reliable channel.
    pub min_channel_variance: f32,
    /// Minimum sample confidence for training inclusion.
    pub min_training_confidence: f32,
    /// Maximum sample reprojection RMS for training inclusion.
    pub max_training_residual: f32,
    /// Maximum condition estimate for the regularized solve.
    pub max_condition_number: f32,
}

impl Default for GnmDecoderConfig {
    fn default() -> Self {
        Self {
            active_expression_dimension: crate::DEFAULT_ACTIVE_EXPRESSION_DIMENSION,
            regularization: DEFAULT_DECODER_REGULARIZATION,
            min_training_samples: DEFAULT_MIN_DECODER_SAMPLES,
            min_channel_variance: DEFAULT_MIN_CHANNEL_VARIANCE,
            min_training_confidence: DEFAULT_MIN_TRAINING_CONFIDENCE,
            max_training_residual: DEFAULT_MAX_TRAINING_RESIDUAL,
            max_condition_number: DEFAULT_DECODER_MAX_CONDITION_NUMBER,
        }
    }
}

/// Numeric synchronized GNM/teacher sample used only during decoder training.
#[derive(Clone, Debug, PartialEq)]
pub struct GnmDecoderTrainingSample {
    /// Source frame sequence shared by the GNM and teacher values.
    pub source_seq: u64,
    /// Active GNM expression coefficients.
    pub active_gnm_expression: Vec<f32>,
    /// Teacher ARKit-like coefficients. TongueOut is ignored by policy.
    pub teacher_arkit52: Arkit52Coefficients,
    /// GNM fitting confidence.
    pub fit_confidence: f32,
    /// GNM reprojection residual.
    pub gnm_reprojection_rms: f32,
}

impl GnmDecoderTrainingSample {
    /// Creates a sample after validating its numeric metadata.
    pub fn new(
        source_seq: u64,
        active_gnm_expression: Vec<f32>,
        teacher_arkit52: Arkit52Coefficients,
        fit_confidence: f32,
        gnm_reprojection_rms: f32,
    ) -> Result<Self, GnmDecoderError> {
        if active_gnm_expression.iter().any(|value| !value.is_finite()) {
            return Err(GnmDecoderError::NonFiniteSample {
                field: "active_gnm_expression",
            });
        }
        if !fit_confidence.is_finite() || !(0.0..=1.0).contains(&fit_confidence) {
            return Err(GnmDecoderError::InvalidSample {
                field: "fit_confidence",
                reason: "must be finite and in [0, 1]",
            });
        }
        if !gnm_reprojection_rms.is_finite() || gnm_reprojection_rms < 0.0 {
            return Err(GnmDecoderError::InvalidSample {
                field: "gnm_reprojection_rms",
                reason: "must be finite and non-negative",
            });
        }
        Ok(Self {
            source_seq,
            active_gnm_expression,
            teacher_arkit52,
            fit_confidence,
            gnm_reprojection_rms,
        })
    }
}

/// Decoder training or runtime failure.
#[derive(Clone, Debug, PartialEq)]
pub enum GnmDecoderError {
    /// Decoder configuration is invalid.
    InvalidConfig {
        /// Configuration field.
        field: &'static str,
        /// Stable reason.
        reason: &'static str,
    },
    /// A sample had an invalid dimension or sequence.
    InvalidSample {
        /// Sample field.
        field: &'static str,
        /// Stable reason.
        reason: &'static str,
    },
    /// A sample contained a non-finite latent value.
    NonFiniteSample {
        /// Sample field.
        field: &'static str,
    },
    /// Too few quality-gated samples were available.
    InsufficientSamples {
        /// Required count.
        required: usize,
        /// Available count.
        actual: usize,
    },
    /// The training normal equation was singular or ill-conditioned.
    IllConditioned {
        /// Estimated condition number.
        condition_number: f32,
    },
    /// A runtime state uses a different active subspace.
    ActiveSubspaceMismatch {
        /// Decoder active dimension.
        expected: usize,
        /// Runtime state active dimension.
        actual: usize,
    },
    /// A runtime GNM state cannot be decoded.
    InvalidRuntimeState {
        /// Stable reason.
        reason: &'static str,
    },
    /// The requested GNM model schema version is not supported.
    UnsupportedModelVersion {
        /// Schema major version.
        major: u16,
        /// Schema minor version.
        minor: u16,
    },
}

impl Display for GnmDecoderError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidConfig { field, reason } => {
                write!(formatter, "invalid decoder config `{field}`: {reason}")
            }
            Self::InvalidSample { field, reason } => {
                write!(formatter, "invalid decoder sample `{field}`: {reason}")
            }
            Self::NonFiniteSample { field } => {
                write!(formatter, "non-finite decoder sample field `{field}`")
            }
            Self::InsufficientSamples { required, actual } => {
                write!(
                    formatter,
                    "insufficient decoder samples: need {required}, got {actual}"
                )
            }
            Self::IllConditioned { condition_number } => {
                write!(
                    formatter,
                    "ill-conditioned decoder solve: {condition_number}"
                )
            }
            Self::ActiveSubspaceMismatch { expected, actual } => {
                write!(
                    formatter,
                    "decoder active subspace mismatch: expected {expected}, got {actual}"
                )
            }
            Self::InvalidRuntimeState { reason } => {
                write!(formatter, "invalid GNM runtime state: {reason}")
            }
            Self::UnsupportedModelVersion { major, minor } => {
                write!(formatter, "unsupported GNM model version {major}.{minor}")
            }
        }
    }
}

impl std::error::Error for GnmDecoderError {}

/// Diagnostics produced with every trained decoder model.
#[derive(Clone, Debug, PartialEq)]
pub struct GnmDecoderDiagnostics {
    /// Number of quality-gated samples used.
    pub sample_count: usize,
    /// Active latent dimension.
    pub active_latent_dimension: usize,
    /// Estimated regularized latent rank.
    pub active_latent_rank: usize,
    /// Estimated condition number.
    pub condition_number: f32,
    /// Per-channel teacher variance; TongueOut is always zero/unobserved.
    pub teacher_variance: [f32; ARKIT52_CHANNEL_COUNT],
    /// Per-channel reliability after coverage gating.
    pub reliable_channels: [bool; ARKIT52_CHANNEL_COUNT],
    /// Weighted training RMS across observed teacher channels.
    pub train_residual: f32,
    /// Whether sample count, rank, and channel coverage meet readiness policy.
    pub ready: bool,
    /// Explicit policy marker for the unobserved TongueOut target.
    pub tongue_out_unobserved: bool,
}

/// Validated frozen GNM-to-ARKit52 linear decoder.
#[derive(Clone, Debug, PartialEq)]
pub struct GnmToArkit52Decoder {
    /// GNM schema version used during training.
    pub model_version: GnmVersion,
    /// Active GNM expression dimension.
    pub active_expression_dimension: usize,
    /// Regularization used during training.
    pub regularization: f32,
    /// Latent-to-target weights for the 52 output channels.
    weights: Vec<Vec<f32>>,
    /// Output bias for the 52 channels.
    bias: [f32; ARKIT52_CHANNEL_COUNT],
    /// Per-channel reliability diagnostics.
    pub reliable_channels: [bool; ARKIT52_CHANNEL_COUNT],
    /// Training diagnostics snapshot.
    pub diagnostics: GnmDecoderDiagnostics,
}

/// Result of one bounded decoder training operation.
#[derive(Clone, Debug, PartialEq)]
pub struct GnmDecoderTrainingResult {
    /// Frozen decoder model.
    pub decoder: GnmToArkit52Decoder,
    /// Coverage and numerical diagnostics.
    pub diagnostics: GnmDecoderDiagnostics,
}

/// Trains one frozen decoder from synchronized numeric samples.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GnmDecoderTrainer {
    model_version: GnmVersion,
    config: GnmDecoderConfig,
}

impl GnmDecoderTrainer {
    /// Creates a trainer for a supported GNM v3 model version.
    pub fn new(
        model_version: GnmVersion,
        config: GnmDecoderConfig,
    ) -> Result<Self, GnmDecoderError> {
        if model_version.major != 3 {
            return Err(GnmDecoderError::UnsupportedModelVersion {
                major: model_version.major,
                minor: model_version.minor,
            });
        }
        validate_config(config)?;
        Ok(Self {
            model_version,
            config,
        })
    }

    /// Returns the trainer configuration.
    #[must_use]
    pub const fn config(&self) -> GnmDecoderConfig {
        self.config
    }

    /// Trains a regularized decoder from synchronized numeric samples.
    ///
    /// Samples failing confidence or reprojection gates are excluded. A model
    /// may be returned with `diagnostics.ready == false` when coverage is
    /// neutral-heavy; callers must use readiness diagnostics before activation.
    pub fn train(
        &self,
        samples: &[GnmDecoderTrainingSample],
    ) -> Result<GnmDecoderTrainingResult, GnmDecoderError> {
        validate_sequence(samples)?;
        let accepted: Vec<&GnmDecoderTrainingSample> = samples
            .iter()
            .filter(|sample| {
                sample.fit_confidence >= self.config.min_training_confidence
                    && sample.gnm_reprojection_rms <= self.config.max_training_residual
            })
            .collect();
        if accepted.len() < self.config.min_training_samples {
            return Err(GnmDecoderError::InsufficientSamples {
                required: self.config.min_training_samples,
                actual: accepted.len(),
            });
        }
        if accepted.iter().any(|sample| {
            sample.active_gnm_expression.len() != self.config.active_expression_dimension
        }) {
            return Err(GnmDecoderError::InvalidSample {
                field: "active_gnm_expression",
                reason: "dimension does not match decoder active subspace",
            });
        }

        let feature_dimension = self.config.active_expression_dimension + 1;
        let mut normal = vec![vec![0.0f64; feature_dimension]; feature_dimension];
        for sample in &accepted {
            let mut features = sample.active_gnm_expression.clone();
            features.push(1.0);
            let weight = f64::from(sample.fit_confidence);
            for left in 0..feature_dimension {
                for right in 0..feature_dimension {
                    normal[left][right] +=
                        weight * f64::from(features[left]) * f64::from(features[right]);
                }
            }
        }
        for (index, row) in normal
            .iter_mut()
            .enumerate()
            .take(self.config.active_expression_dimension)
        {
            row[index] += f64::from(self.config.regularization);
        }
        let (inverse_solve, rank, condition_number) =
            factor_matrix(normal.clone()).ok_or(GnmDecoderError::IllConditioned {
                condition_number: f32::INFINITY,
            })?;
        if condition_number > f64::from(self.config.max_condition_number) {
            return Err(GnmDecoderError::IllConditioned {
                condition_number: condition_number as f32,
            });
        }

        let mut weights =
            vec![vec![0.0; self.config.active_expression_dimension]; ARKIT52_CHANNEL_COUNT];
        let mut bias = [0.0; ARKIT52_CHANNEL_COUNT];
        let mut teacher_variance = [0.0; ARKIT52_CHANNEL_COUNT];
        let mut reliable_channels = [false; ARKIT52_CHANNEL_COUNT];
        for channel in ArkitBlendshape::ALL {
            if channel == ArkitBlendshape::TongueOut {
                continue;
            }
            let values: Vec<f64> = accepted
                .iter()
                .map(|sample| f64::from(sample.teacher_arkit52.get(channel)))
                .collect();
            let mean = values.iter().sum::<f64>() / values.len() as f64;
            let variance = values
                .iter()
                .map(|value| {
                    let delta = value - mean;
                    delta * delta
                })
                .sum::<f64>()
                / values.len() as f64;
            teacher_variance[channel.index()] = variance as f32;
            reliable_channels[channel.index()] =
                variance >= f64::from(self.config.min_channel_variance);

            let mut rhs = vec![0.0; feature_dimension];
            for sample in &accepted {
                let mut features = sample.active_gnm_expression.clone();
                features.push(1.0);
                let target = f64::from(sample.teacher_arkit52.get(channel));
                let sample_weight = f64::from(sample.fit_confidence);
                for index in 0..feature_dimension {
                    rhs[index] += sample_weight * f64::from(features[index]) * target;
                }
            }
            let solved = solve_with_factor(&inverse_solve, rhs);
            for index in 0..self.config.active_expression_dimension {
                weights[channel.index()][index] = solved[index] as f32;
            }
            bias[channel.index()] = solved[self.config.active_expression_dimension] as f32;
        }

        let mut squared_error = 0.0f64;
        let mut error_count = 0usize;
        for sample in &accepted {
            for channel in ArkitBlendshape::ALL {
                if channel == ArkitBlendshape::TongueOut {
                    continue;
                }
                let predicted = predict_channel(
                    &weights[channel.index()],
                    bias[channel.index()],
                    &sample.active_gnm_expression,
                );
                let delta = predicted - sample.teacher_arkit52.get(channel);
                squared_error += f64::from(delta * delta);
                error_count += 1;
            }
        }
        let train_residual = (squared_error / error_count.max(1) as f64).sqrt() as f32;
        let reliable_count = reliable_channels.iter().filter(|value| **value).count();
        let ready = rank >= self.config.active_expression_dimension
            && reliable_count > 0
            && train_residual.is_finite();
        let diagnostics = GnmDecoderDiagnostics {
            sample_count: accepted.len(),
            active_latent_dimension: self.config.active_expression_dimension,
            active_latent_rank: rank,
            condition_number: condition_number as f32,
            teacher_variance,
            reliable_channels,
            train_residual,
            ready,
            tongue_out_unobserved: true,
        };
        let decoder = GnmToArkit52Decoder {
            model_version: self.model_version,
            active_expression_dimension: self.config.active_expression_dimension,
            regularization: self.config.regularization,
            weights,
            bias,
            reliable_channels,
            diagnostics: diagnostics.clone(),
        };
        Ok(GnmDecoderTrainingResult {
            decoder,
            diagnostics,
        })
    }
}

impl GnmToArkit52Decoder {
    /// Decodes an engine-neutral GNM state without accessing any teacher data.
    pub fn decode(&self, state: &GnmFaceState) -> Result<Arkit52Coefficients, GnmDecoderError> {
        if state.active_expression_dimension != self.active_expression_dimension {
            return Err(GnmDecoderError::ActiveSubspaceMismatch {
                expected: self.active_expression_dimension,
                actual: state.active_expression_dimension,
            });
        }
        if state.status == GnmFaceStatus::InvalidFit {
            return Err(GnmDecoderError::InvalidRuntimeState {
                reason: "invalid GNM fit cannot be decoded",
            });
        }
        if state.expression.values().len() < self.active_expression_dimension {
            return Err(GnmDecoderError::ActiveSubspaceMismatch {
                expected: self.active_expression_dimension,
                actual: state.expression.values().len(),
            });
        }
        let mut output = [0.0; ARKIT52_CHANNEL_COUNT];
        for channel in ArkitBlendshape::ALL {
            if channel == ArkitBlendshape::TongueOut {
                output[channel.index()] = 0.0;
                continue;
            }
            let value = predict_channel(
                &self.weights[channel.index()],
                self.bias[channel.index()],
                &state.expression.values()[..self.active_expression_dimension],
            );
            if !value.is_finite() {
                return Err(GnmDecoderError::InvalidRuntimeState {
                    reason: "decoder output is non-finite",
                });
            }
            output[channel.index()] = value.clamp(0.0, 1.0);
        }
        Arkit52Coefficients::try_from_array(output).map_err(|_| {
            GnmDecoderError::InvalidRuntimeState {
                reason: "decoder output failed ARKit52 validation",
            }
        })
    }
}

fn validate_config(config: GnmDecoderConfig) -> Result<(), GnmDecoderError> {
    if config.active_expression_dimension == 0
        || !config.regularization.is_finite()
        || config.regularization <= 0.0
        || config.min_training_samples == 0
        || !config.min_channel_variance.is_finite()
        || config.min_channel_variance < 0.0
        || !config.min_training_confidence.is_finite()
        || !(0.0..=1.0).contains(&config.min_training_confidence)
        || !config.max_training_residual.is_finite()
        || config.max_training_residual <= 0.0
        || !config.max_condition_number.is_finite()
        || config.max_condition_number <= 1.0
    {
        return Err(GnmDecoderError::InvalidConfig {
            field: "decoder settings",
            reason: "settings must be finite and within their bounds",
        });
    }
    Ok(())
}

fn validate_sequence(samples: &[GnmDecoderTrainingSample]) -> Result<(), GnmDecoderError> {
    for pair in samples.windows(2) {
        if pair[1].source_seq <= pair[0].source_seq {
            return Err(GnmDecoderError::InvalidSample {
                field: "source_seq",
                reason: "training samples must be strictly ordered",
            });
        }
    }
    Ok(())
}

fn predict_channel(weights: &[f32], bias: f32, expression: &[f32]) -> f32 {
    weights
        .iter()
        .zip(expression)
        .fold(bias, |sum, (weight, value)| sum + weight * value)
}

fn factor_matrix(mut matrix: Vec<Vec<f64>>) -> Option<(Vec<Vec<f64>>, usize, f64)> {
    let dimension = matrix.len();
    if matrix.iter().any(|row| row.len() != dimension) {
        return None;
    }
    let mut inverse = vec![vec![0.0; dimension]; dimension];
    for (index, row) in inverse.iter_mut().enumerate() {
        row[index] = 1.0;
    }
    let mut pivots = Vec::with_capacity(dimension);
    for pivot in 0..dimension {
        let mut pivot_row = pivot;
        for row in pivot + 1..dimension {
            if matrix[row][pivot].abs() > matrix[pivot_row][pivot].abs() {
                pivot_row = row;
            }
        }
        if matrix[pivot_row][pivot].abs() < 1.0e-12 {
            return None;
        }
        if pivot_row != pivot {
            matrix.swap(pivot, pivot_row);
            inverse.swap(pivot, pivot_row);
        }
        let diagonal = matrix[pivot][pivot];
        pivots.push(diagonal.abs());
        for value in matrix[pivot].iter_mut().skip(pivot) {
            *value /= diagonal;
        }
        for value in inverse[pivot].iter_mut() {
            *value /= diagonal;
        }
        let normalized = matrix[pivot].clone();
        let normalized_inverse = inverse[pivot].clone();
        for row in 0..dimension {
            if row == pivot {
                continue;
            }
            let factor = matrix[row][pivot];
            for (column, value) in matrix[row].iter_mut().enumerate().skip(pivot) {
                *value -= factor * normalized[column];
            }
            for (column, value) in inverse[row].iter_mut().enumerate() {
                *value -= factor * normalized_inverse[column];
            }
        }
    }
    let condition_number = pivots.iter().copied().fold(0.0, f64::max)
        / pivots.iter().copied().fold(f64::INFINITY, f64::min);
    Some((inverse, dimension, condition_number))
}

fn solve_with_factor(factor: &[Vec<f64>], rhs: Vec<f64>) -> Vec<f64> {
    let dimension = factor.len();
    (0..dimension)
        .map(|row| {
            factor[row]
                .iter()
                .zip(&rhs)
                .map(|(coefficient, value)| coefficient * value)
                .sum()
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{GnmExpressionState, GnmIdentityState, GnmProjectionFit, GnmProjectionModel};

    fn teacher(values: &[(ArkitBlendshape, f32)]) -> Arkit52Coefficients {
        let mut output = [0.0; ARKIT52_CHANNEL_COUNT];
        for &(channel, value) in values {
            output[channel.index()] = value;
        }
        Arkit52Coefficients::try_from_array(output).unwrap()
    }

    fn sample(sequence: u64, x: f32, y: f32) -> GnmDecoderTrainingSample {
        GnmDecoderTrainingSample::new(
            sequence,
            vec![x, y],
            teacher(&[
                (
                    ArkitBlendshape::JawOpen,
                    (0.1 + 0.6 * x + 0.2 * y).clamp(0.0, 1.0),
                ),
                (
                    ArkitBlendshape::EyeBlinkLeft,
                    (0.8 - 0.5 * x).clamp(0.0, 1.0),
                ),
            ]),
            1.0,
            0.01,
        )
        .unwrap()
    }

    fn runtime_state(expression: [f32; 2]) -> GnmFaceState {
        let mut values = vec![0.0; 383];
        values[..2].copy_from_slice(&expression);
        GnmFaceState {
            source_seq: 99,
            captured_at_ns: 1,
            identity: GnmIdentityState::neutral(253),
            expression: GnmExpressionState::new(values, 383).unwrap(),
            projection: GnmProjectionFit {
                model: GnmProjectionModel::default(),
                residual_rms: 0.01,
                valid_point_count: 68,
                iterations: 1,
            },
            active_expression_dimension: 2,
            active_expression_rank: 2,
            reprojection_rms: 0.01,
            temporal_delta_rms: 0.0,
            confidence: 1.0,
            status: GnmFaceStatus::Tracking,
        }
    }

    fn trainer() -> GnmDecoderTrainer {
        GnmDecoderTrainer::new(
            GnmVersion { major: 3, minor: 0 },
            GnmDecoderConfig {
                active_expression_dimension: 2,
                min_training_samples: 6,
                ..GnmDecoderConfig::default()
            },
        )
        .unwrap()
    }

    #[test]
    fn known_linear_mapping_is_recovered_without_teacher_at_decode() {
        let samples: Vec<_> = (0..8)
            .map(|index| sample(index + 1, index as f32 / 7.0, (7 - index) as f32 / 7.0))
            .collect();
        let result = trainer().train(&samples).unwrap();
        assert!(result.diagnostics.ready);
        let output = result.decoder.decode(&runtime_state([0.25, 0.75])).unwrap();
        assert!((output.get(ArkitBlendshape::JawOpen) - 0.4).abs() < 0.05);
        assert_eq!(output.get(ArkitBlendshape::TongueOut), 0.0);
    }

    #[test]
    fn neutral_heavy_training_is_diagnosed_not_marked_ready() {
        let samples: Vec<_> = (0..8).map(|index| sample(index + 1, 0.0, 0.0)).collect();
        let result = trainer().train(&samples).unwrap();
        assert!(!result.diagnostics.ready);
        assert!(
            result
                .diagnostics
                .reliable_channels
                .iter()
                .all(|reliable| !reliable)
        );
        assert!(result.diagnostics.tongue_out_unobserved);
    }

    #[test]
    fn quality_gate_sequence_and_version_contracts_are_typed() {
        assert!(matches!(
            GnmDecoderTrainer::new(
                GnmVersion { major: 2, minor: 0 },
                GnmDecoderConfig::default()
            ),
            Err(GnmDecoderError::UnsupportedModelVersion { .. })
        ));
        let mut samples = vec![sample(1, 0.0, 0.0), sample(1, 1.0, 1.0)];
        let result = trainer().train(&samples);
        assert!(matches!(result, Err(GnmDecoderError::InvalidSample { .. })));
        samples[1].source_seq = 2;
        samples[1].fit_confidence = 0.1;
        assert!(matches!(
            trainer().train(&samples),
            Err(GnmDecoderError::InsufficientSamples { .. })
        ));
    }

    #[test]
    fn runtime_output_is_bounded_and_deterministic() {
        let samples: Vec<_> = (0..8)
            .map(|index| sample(index + 1, index as f32 / 7.0, 0.5))
            .collect();
        let decoder = trainer().train(&samples).unwrap().decoder;
        let state = runtime_state([0.5, 0.5]);
        let first = decoder.decode(&state).unwrap();
        let second = decoder.decode(&state).unwrap();
        assert_eq!(first, second);
        assert!(
            first
                .as_array()
                .iter()
                .all(|value| (0.0..=1.0).contains(value))
        );
    }
}

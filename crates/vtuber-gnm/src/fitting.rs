//! Bounded neutral calibration and temporal GNM expression fitting.
//!
//! The first fitting cut intentionally uses a small, ordered active subspace
//! rather than pretending that 68 two-dimensional points can identify all 253
//! identity or 383 expression coefficients.  Model evaluation remains the
//! source of the basis responses; this module only solves the regularized
//! sparse reprojection objective.

use std::fmt::{Display, Formatter};

use crate::{
    DEFAULT_MEDIAPIPE_TO_GNM_MAP, GnmFitError, GnmIdentityState, GnmJointState, GnmModel,
    GnmModelError, GnmProjectionFit, GnmProjectionModel, GnmSparseObservation, GnmSparseVertices,
    SparseLandmarkSet, fit_weak_perspective, project_weak_perspective,
};

/// Default number of ordered identity coefficients exposed to the first-cut
/// calibration solver.
pub const DEFAULT_ACTIVE_IDENTITY_DIMENSION: usize = 8;
/// Default number of ordered expression coefficients exposed to the first-cut
/// per-frame solver.
pub const DEFAULT_ACTIVE_EXPRESSION_DIMENSION: usize = 16;
/// Default bounded number of alternating coefficient/camera updates.
pub const DEFAULT_MAX_ITERATIONS: usize = 4;
/// Minimum number of neutral samples required before identity is fixed.
pub const DEFAULT_MIN_CALIBRATION_SAMPLES: usize = 3;
/// Default ridge term for neutral identity calibration.
pub const DEFAULT_IDENTITY_REGULARIZATION: f32 = 1.0e-3;
/// Default ridge term for the temporal expression solve.
pub const DEFAULT_EXPRESSION_REGULARIZATION: f32 = 1.0e-3;
/// Default temporal prior strength for expression fitting.
pub const DEFAULT_TEMPORAL_REGULARIZATION: f32 = 0.05;
/// Default residual RMS at which a fit becomes degraded.
pub const DEFAULT_RESIDUAL_THRESHOLD: f32 = 0.03;
/// Maximum accepted regularized normal-equation condition estimate.
pub const DEFAULT_MAX_CONDITION_NUMBER: f32 = 1.0e8;
/// Identity coefficient bound used to prevent under-determined blow-up.
pub const DEFAULT_IDENTITY_COEFFICIENT_BOUND: f32 = 3.0;

/// Configuration for the bounded GNM calibration and expression solver.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GnmFitterConfig {
    /// Number of leading identity basis components in the active subspace.
    pub active_identity_dimension: usize,
    /// Number of leading expression basis components in the active subspace.
    pub active_expression_dimension: usize,
    /// Ridge regularization for identity calibration.
    pub identity_regularization: f32,
    /// Ridge regularization for expression fitting.
    pub expression_regularization: f32,
    /// Temporal prior strength for expression fitting.
    pub temporal_regularization: f32,
    /// Maximum alternating solver iterations.
    pub max_iterations: usize,
    /// Minimum neutral samples needed for calibration.
    pub min_calibration_samples: usize,
    /// Residual RMS above which the result is degraded.
    pub residual_threshold: f32,
    /// Maximum accepted regularized normal-equation condition estimate.
    pub max_condition_number: f32,
    /// Absolute bound for identity coefficients.
    pub identity_coefficient_bound: f32,
}

impl Default for GnmFitterConfig {
    fn default() -> Self {
        Self {
            active_identity_dimension: DEFAULT_ACTIVE_IDENTITY_DIMENSION,
            active_expression_dimension: DEFAULT_ACTIVE_EXPRESSION_DIMENSION,
            identity_regularization: DEFAULT_IDENTITY_REGULARIZATION,
            expression_regularization: DEFAULT_EXPRESSION_REGULARIZATION,
            temporal_regularization: DEFAULT_TEMPORAL_REGULARIZATION,
            max_iterations: DEFAULT_MAX_ITERATIONS,
            min_calibration_samples: DEFAULT_MIN_CALIBRATION_SAMPLES,
            residual_threshold: DEFAULT_RESIDUAL_THRESHOLD,
            max_condition_number: DEFAULT_MAX_CONDITION_NUMBER,
            identity_coefficient_bound: DEFAULT_IDENTITY_COEFFICIENT_BOUND,
        }
    }
}

/// A numeric sparse observation associated with one source frame.
#[derive(Clone, Debug, PartialEq)]
pub struct GnmFittingSample {
    /// Source camera frame sequence.
    pub source_seq: u64,
    /// Source capture timestamp in monotonic nanoseconds.
    pub captured_at_ns: u64,
    /// Sparse normalized observation in the correspondence order.
    pub observation: GnmSparseObservation,
    /// Upstream confidence in `[0, 1]`.
    pub confidence: f32,
}

impl GnmFittingSample {
    /// Creates a sample after validating its confidence.
    ///
    /// The observation itself is validated by [`GnmSparseObservation::new`].
    pub fn new(
        source_seq: u64,
        captured_at_ns: u64,
        observation: GnmSparseObservation,
        confidence: f32,
    ) -> Result<Self, GnmFitterError> {
        if !confidence.is_finite() || !(0.0..=1.0).contains(&confidence) {
            return Err(GnmFitterError::InvalidSample {
                field: "confidence",
                reason: "confidence must be finite and in [0, 1]",
            });
        }
        Ok(Self {
            source_seq,
            captured_at_ns,
            observation,
            confidence,
        })
    }
}

/// Failure from configuration, sample validation, or a bounded GNM solve.
#[derive(Clone, Debug, PartialEq)]
pub enum GnmFitterError {
    /// A solver setting is invalid for the selected model.
    InvalidConfig {
        /// Configuration field.
        field: &'static str,
        /// Stable reason.
        reason: &'static str,
    },
    /// Too few neutral samples were supplied.
    InsufficientCalibrationSamples {
        /// Required sample count.
        required: usize,
        /// Supplied sample count.
        actual: usize,
    },
    /// Expression fitting was requested before identity calibration.
    Uncalibrated,
    /// A sample sequence regressed.
    SequenceRegression {
        /// Previously accepted sequence.
        previous: u64,
        /// New sequence.
        current: u64,
    },
    /// A sample violated the fitter's structural contract.
    InvalidSample {
        /// Sample field.
        field: &'static str,
        /// Stable reason.
        reason: &'static str,
    },
    /// The regularized solve was numerically ill-conditioned.
    IllConditioned {
        /// Identity or expression solve.
        kind: &'static str,
        /// Estimated condition number.
        condition_number: f32,
    },
    /// The model evaluator rejected a state.
    Model(GnmModelError),
    /// The projection objective rejected an observation or camera fit.
    Projection(GnmFitError),
}

impl Display for GnmFitterError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidConfig { field, reason } => {
                write!(formatter, "invalid GNM fitter config `{field}`: {reason}")
            }
            Self::InsufficientCalibrationSamples { required, actual } => write!(
                formatter,
                "insufficient neutral calibration samples: need {required}, got {actual}"
            ),
            Self::Uncalibrated => write!(formatter, "GNM identity is not calibrated"),
            Self::SequenceRegression { previous, current } => write!(
                formatter,
                "GNM source sequence regressed from {previous} to {current}"
            ),
            Self::InvalidSample { field, reason } => {
                write!(formatter, "invalid GNM fitting sample `{field}`: {reason}")
            }
            Self::IllConditioned {
                kind,
                condition_number,
            } => write!(
                formatter,
                "ill-conditioned {kind} GNM solve: condition estimate {condition_number}"
            ),
            Self::Model(error) => error.fmt(formatter),
            Self::Projection(error) => error.fmt(formatter),
        }
    }
}

impl std::error::Error for GnmFitterError {}

impl From<GnmModelError> for GnmFitterError {
    fn from(value: GnmModelError) -> Self {
        Self::Model(value)
    }
}

impl From<GnmFitError> for GnmFitterError {
    fn from(value: GnmFitError) -> Self {
        Self::Projection(value)
    }
}

/// Result of neutral identity calibration.
#[derive(Clone, Debug, PartialEq)]
pub struct GnmIdentityCalibration {
    /// Fixed full-dimensional identity state; inactive components are zero.
    pub coefficients: GnmIdentityState,
    /// Weighted mean reprojection RMS across calibration samples.
    pub fit_residual: f32,
    /// Number of accepted neutral samples.
    pub sample_count: usize,
    /// Number of identity components in the active subspace.
    pub active_dimension: usize,
    /// Estimated rank of the regularized identity objective.
    pub estimated_rank: usize,
    /// Estimated condition number of the regularized identity objective.
    pub condition_number: f32,
    /// Whether this calibration passed the configured residual gate.
    pub valid: bool,
}

/// Runtime status of a GNM face state.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum GnmFaceStatus {
    /// Identity calibration has not completed.
    Uncalibrated,
    /// The current fit is within the residual gate.
    Tracking,
    /// A state exists but residual/confidence is reduced.
    Degraded,
    /// The current observation could not produce a valid state.
    InvalidFit,
}

/// Engine-neutral GNM face state produced after identity calibration.
#[derive(Clone, Debug, PartialEq)]
pub struct GnmFaceState {
    /// Source camera frame sequence.
    pub source_seq: u64,
    /// Source capture timestamp in monotonic nanoseconds.
    pub captured_at_ns: u64,
    /// Fixed identity state.
    pub identity: GnmIdentityState,
    /// Current active-subspace expression state.
    pub expression: crate::GnmExpressionState,
    /// Fitted rigid weak-perspective camera/pose.
    pub projection: GnmProjectionFit,
    /// Number of active expression components.
    pub active_expression_dimension: usize,
    /// Estimated rank of the current expression objective.
    pub active_expression_rank: usize,
    /// Reprojection RMS in normalized image coordinates.
    pub reprojection_rms: f32,
    /// RMS change from the temporal expression prior over active components.
    pub temporal_delta_rms: f32,
    /// Bounded confidence derived from sample confidence and residual.
    pub confidence: f32,
    /// State validity class.
    pub status: GnmFaceStatus,
}

/// Stateful, bounded GNM identity/expression fitter.
pub struct GnmFaceFitter<'model> {
    model: &'model GnmModel,
    landmarks: &'model SparseLandmarkSet,
    config: GnmFitterConfig,
    identity: Option<GnmIdentityCalibration>,
    previous_expression: crate::GnmExpressionState,
    previous_source_seq: Option<u64>,
    previous_projection: Option<GnmProjectionModel>,
}

impl<'model> GnmFaceFitter<'model> {
    /// Creates a fitter for one validated GNM model and sparse landmark set.
    ///
    /// The default correspondence map requires exactly 68 sparse points. The
    /// selected active dimensions are deliberately checked against the full
    /// model dimensions at construction time.
    pub fn new(
        model: &'model GnmModel,
        landmarks: &'model SparseLandmarkSet,
        config: GnmFitterConfig,
    ) -> Result<Self, GnmFitterError> {
        validate_config(model, landmarks, config)?;
        Ok(Self {
            model,
            landmarks,
            previous_expression: model.neutral_expression(),
            config,
            identity: None,
            previous_source_seq: None,
            previous_projection: None,
        })
    }

    /// Returns the selected bounded configuration.
    #[must_use]
    pub const fn config(&self) -> GnmFitterConfig {
        self.config
    }

    /// Returns the latest accepted identity calibration, if any.
    #[must_use]
    pub fn calibration(&self) -> Option<&GnmIdentityCalibration> {
        self.identity.as_ref()
    }

    /// Clears the current calibration and all temporal fitting state.
    pub fn clear_calibration(&mut self) {
        self.identity = None;
        self.reset_tracking();
    }

    /// Resets expression/camera state while retaining fixed identity.
    pub fn reset_tracking(&mut self) {
        self.previous_expression = self.model.neutral_expression();
        self.previous_source_seq = None;
        self.previous_projection = None;
    }

    /// Calibrates a fixed identity from a bounded neutral sample window.
    ///
    /// Expressions are held at neutral throughout this operation. The solver
    /// uses the first ordered active identity components, ridge regularization,
    /// and at most `config.max_iterations` camera/identity updates. It does
    /// not claim that inactive components were observed.
    pub fn calibrate_neutral(
        &mut self,
        samples: &[GnmFittingSample],
    ) -> Result<GnmIdentityCalibration, GnmFitterError> {
        validate_sample_sequence(samples)?;
        if samples.len() < self.config.min_calibration_samples {
            return Err(GnmFitterError::InsufficientCalibrationSamples {
                required: self.config.min_calibration_samples,
                actual: samples.len(),
            });
        }

        let mut identity = self.model.neutral_identity();
        let expression = self.model.neutral_expression();
        let mut last_projection = None;
        let mut solve_result = SolveResult::empty(self.config.active_identity_dimension);

        for _ in 0..self.config.max_iterations {
            let mut normal = NormalEquations::new(self.config.active_identity_dimension);
            for sample in samples {
                let vertices = self.evaluate(&identity, &expression)?;
                let projection = fit_weak_perspective(
                    &vertices,
                    &DEFAULT_MEDIAPIPE_TO_GNM_MAP,
                    &sample.observation,
                    last_projection,
                )?;
                last_projection = Some(projection.model);
                let basis = self.basis_deltas(
                    &identity,
                    &expression,
                    projection.model,
                    BasisKind::Identity,
                    self.config.active_identity_dimension,
                    &vertices,
                )?;
                normal.accumulate(&vertices, projection.model, sample, &basis);
            }
            let identity_target: Vec<f32> = identity.values()
                [..self.config.active_identity_dimension]
                .iter()
                .map(|value| -*value)
                .collect();
            solve_result = normal.solve(
                self.config.identity_regularization,
                &identity_target,
                "identity",
                self.config.max_condition_number,
            )?;
            let mut values = identity.values().to_vec();
            let mut delta_norm = 0.0f32;
            for (index, delta) in solve_result.values.iter().enumerate() {
                values[index] = (values[index] + delta).clamp(
                    -self.config.identity_coefficient_bound,
                    self.config.identity_coefficient_bound,
                );
                delta_norm += delta * delta;
            }
            identity = GnmIdentityState::new(values, self.model.identity_dimension())?;
            if delta_norm.sqrt() < 1.0e-5 {
                break;
            }
        }

        let fit_residual = mean_residual(self, samples, &identity, &expression, last_projection)?;
        let valid = fit_residual <= self.config.residual_threshold;
        if !valid {
            return Err(GnmFitterError::InvalidSample {
                field: "neutral calibration residual",
                reason: "residual exceeds the configured calibration gate",
            });
        }
        let calibration = GnmIdentityCalibration {
            coefficients: identity,
            fit_residual,
            sample_count: samples.len(),
            active_dimension: self.config.active_identity_dimension,
            estimated_rank: solve_result.estimated_rank,
            condition_number: solve_result.condition_number,
            valid,
        };
        self.identity = Some(calibration.clone());
        self.reset_tracking();
        Ok(calibration)
    }

    /// Fits one frame's expression state on the fixed calibrated identity.
    ///
    /// A source-sequence gap resets the temporal prior to neutral. The solve
    /// is bounded and emits only the selected active expression components;
    /// all other 383 slots remain zero.
    pub fn fit_expression(
        &mut self,
        sample: &GnmFittingSample,
    ) -> Result<GnmFaceState, GnmFitterError> {
        let identity = self
            .identity
            .as_ref()
            .ok_or(GnmFitterError::Uncalibrated)?
            .coefficients
            .clone();
        if let Some(previous) = self.previous_source_seq {
            if sample.source_seq <= previous {
                return Err(GnmFitterError::SequenceRegression {
                    previous,
                    current: sample.source_seq,
                });
            }
            if sample.source_seq > previous.saturating_add(1) {
                self.previous_expression = self.model.neutral_expression();
            }
        }

        let temporal_reference = self.previous_expression.clone();
        let mut expression = temporal_reference.clone();
        let mut previous_projection = self.previous_projection;
        let mut solve_result = SolveResult::empty(self.config.active_expression_dimension);
        for _ in 0..self.config.max_iterations {
            let vertices = self.evaluate(&identity, &expression)?;
            let projection = fit_weak_perspective(
                &vertices,
                &DEFAULT_MEDIAPIPE_TO_GNM_MAP,
                &sample.observation,
                previous_projection,
            )?;
            previous_projection = Some(projection.model);
            let basis = self.basis_deltas(
                &identity,
                &expression,
                projection.model,
                BasisKind::Expression,
                self.config.active_expression_dimension,
                &vertices,
            )?;
            let target: Vec<f32> = temporal_reference.values()
                [..self.config.active_expression_dimension]
                .iter()
                .zip(&expression.values()[..self.config.active_expression_dimension])
                .map(|(previous, current)| previous - current)
                .collect();
            let mut normal = NormalEquations::new(self.config.active_expression_dimension);
            normal.accumulate(&vertices, projection.model, sample, &basis);
            solve_result = normal.solve(
                self.config.expression_regularization + self.config.temporal_regularization,
                &target,
                "expression",
                self.config.max_condition_number,
            )?;
            let mut values = expression.values().to_vec();
            let mut delta_norm = 0.0f32;
            for (index, delta) in solve_result.values.iter().enumerate() {
                values[index] = (values[index] + delta).clamp(0.0, 1.0);
                delta_norm += delta * delta;
            }
            expression = crate::GnmExpressionState::new(values, self.model.expression_dimension())?;
            if delta_norm.sqrt() < 1.0e-5 {
                break;
            }
        }

        let vertices = self.evaluate(&identity, &expression)?;
        let projection = fit_weak_perspective(
            &vertices,
            &DEFAULT_MEDIAPIPE_TO_GNM_MAP,
            &sample.observation,
            previous_projection,
        )?;
        let reprojection_rms = projection.residual_rms;
        let residual_factor = if self.config.residual_threshold > 0.0 {
            (1.0 - reprojection_rms / self.config.residual_threshold).clamp(0.0, 1.0)
        } else {
            0.0
        };
        let confidence = (sample.confidence * residual_factor).clamp(0.0, 1.0);
        let temporal_delta_rms = expression
            .values()
            .iter()
            .zip(temporal_reference.values())
            .take(self.config.active_expression_dimension)
            .map(|(current, previous)| {
                let delta = current - previous;
                delta * delta
            })
            .sum::<f32>()
            / (self.config.active_expression_dimension as f32).sqrt();
        let status = if reprojection_rms <= self.config.residual_threshold {
            GnmFaceStatus::Tracking
        } else {
            GnmFaceStatus::Degraded
        };
        let state = GnmFaceState {
            source_seq: sample.source_seq,
            captured_at_ns: sample.captured_at_ns,
            identity,
            expression: expression.clone(),
            projection,
            active_expression_dimension: self.config.active_expression_dimension,
            active_expression_rank: solve_result.estimated_rank,
            reprojection_rms,
            temporal_delta_rms,
            confidence,
            status,
        };
        self.previous_expression = expression;
        self.previous_source_seq = Some(sample.source_seq);
        self.previous_projection = Some(projection.model);
        Ok(state)
    }

    fn evaluate(
        &self,
        identity: &GnmIdentityState,
        expression: &crate::GnmExpressionState,
    ) -> Result<GnmSparseVertices, GnmFitterError> {
        let mut output = GnmSparseVertices::with_len(self.landmarks.len());
        self.model.evaluate_sparse(
            identity,
            expression,
            &GnmJointState::neutral(self.model.joint_count()),
            self.landmarks,
            &mut output,
        )?;
        Ok(output)
    }

    fn basis_deltas(
        &self,
        identity: &GnmIdentityState,
        expression: &crate::GnmExpressionState,
        projection: GnmProjectionModel,
        kind: BasisKind,
        dimension: usize,
        base_vertices: &GnmSparseVertices,
    ) -> Result<Vec<Vec<[f32; 2]>>, GnmFitterError> {
        let base_projected: Vec<[f32; 2]> = base_vertices
            .values()
            .iter()
            .map(|&point| project_weak_perspective(point, projection))
            .collect();
        let mut result = Vec::with_capacity(dimension);
        for component in 0..dimension {
            let (candidate_identity, candidate_expression) = match kind {
                BasisKind::Identity => {
                    let mut values = identity.values().to_vec();
                    values[component] += 1.0;
                    (
                        GnmIdentityState::new(values, self.model.identity_dimension())?,
                        expression.clone(),
                    )
                }
                BasisKind::Expression => {
                    let mut values = expression.values().to_vec();
                    values[component] += 1.0;
                    (
                        identity.clone(),
                        crate::GnmExpressionState::new(values, self.model.expression_dimension())?,
                    )
                }
            };
            let candidate_vertices = self.evaluate(&candidate_identity, &candidate_expression)?;
            result.push(
                candidate_vertices
                    .values()
                    .iter()
                    .zip(&base_projected)
                    .map(|(&point, &base)| {
                        let projected = project_weak_perspective(point, projection);
                        [projected[0] - base[0], projected[1] - base[1]]
                    })
                    .collect(),
            );
        }
        Ok(result)
    }
}

#[derive(Clone, Copy)]
enum BasisKind {
    Identity,
    Expression,
}

fn validate_config(
    model: &GnmModel,
    landmarks: &SparseLandmarkSet,
    config: GnmFitterConfig,
) -> Result<(), GnmFitterError> {
    if landmarks.len() != DEFAULT_MEDIAPIPE_TO_GNM_MAP.len() {
        return Err(GnmFitterError::InvalidConfig {
            field: "landmarks",
            reason: "the first-cut correspondence requires 68 sparse landmarks",
        });
    }
    if config.active_identity_dimension == 0
        || config.active_identity_dimension > model.identity_dimension()
    {
        return Err(GnmFitterError::InvalidConfig {
            field: "active_identity_dimension",
            reason: "must be between 1 and the model identity dimension",
        });
    }
    if config.active_expression_dimension == 0
        || config.active_expression_dimension > model.expression_dimension()
    {
        return Err(GnmFitterError::InvalidConfig {
            field: "active_expression_dimension",
            reason: "must be between 1 and the model expression dimension",
        });
    }
    if config.max_iterations == 0
        || config.min_calibration_samples == 0
        || !config.identity_regularization.is_finite()
        || config.identity_regularization <= 0.0
        || !config.expression_regularization.is_finite()
        || config.expression_regularization < 0.0
        || !config.temporal_regularization.is_finite()
        || config.temporal_regularization < 0.0
        || !config.residual_threshold.is_finite()
        || config.residual_threshold <= 0.0
        || !config.max_condition_number.is_finite()
        || config.max_condition_number <= 1.0
        || !config.identity_coefficient_bound.is_finite()
        || config.identity_coefficient_bound <= 0.0
    {
        return Err(GnmFitterError::InvalidConfig {
            field: "solver settings",
            reason: "settings must be finite and within their positive bounds",
        });
    }
    Ok(())
}

fn validate_sample_sequence(samples: &[GnmFittingSample]) -> Result<(), GnmFitterError> {
    for pair in samples.windows(2) {
        if pair[1].source_seq <= pair[0].source_seq {
            return Err(GnmFitterError::SequenceRegression {
                previous: pair[0].source_seq,
                current: pair[1].source_seq,
            });
        }
    }
    Ok(())
}

fn mean_residual(
    fitter: &GnmFaceFitter<'_>,
    samples: &[GnmFittingSample],
    identity: &GnmIdentityState,
    expression: &crate::GnmExpressionState,
    initial: Option<GnmProjectionModel>,
) -> Result<f32, GnmFitterError> {
    let mut weighted_error = 0.0f64;
    let mut confidence_sum = 0.0f64;
    let mut projection = initial;
    for sample in samples {
        let vertices = fitter.evaluate(identity, expression)?;
        let fit = fit_weak_perspective(
            &vertices,
            &DEFAULT_MEDIAPIPE_TO_GNM_MAP,
            &sample.observation,
            projection,
        )?;
        projection = Some(fit.model);
        weighted_error += f64::from(fit.residual_rms) * f64::from(sample.confidence);
        confidence_sum += f64::from(sample.confidence);
    }
    if confidence_sum <= f64::EPSILON {
        return Err(GnmFitterError::InvalidSample {
            field: "confidence",
            reason: "all calibration samples have zero confidence",
        });
    }
    Ok((weighted_error / confidence_sum) as f32)
}

struct NormalEquations {
    matrix: Vec<Vec<f64>>,
    rhs: Vec<f64>,
}

impl NormalEquations {
    fn new(dimension: usize) -> Self {
        Self {
            matrix: vec![vec![0.0; dimension]; dimension],
            rhs: vec![0.0; dimension],
        }
    }

    fn accumulate(
        &mut self,
        base_vertices: &GnmSparseVertices,
        projection: GnmProjectionModel,
        sample: &GnmFittingSample,
        basis: &[Vec<[f32; 2]>],
    ) {
        for (point_index, row) in DEFAULT_MEDIAPIPE_TO_GNM_MAP.iter().enumerate() {
            let weight =
                f64::from(sample.observation.weights()[point_index]) * f64::from(sample.confidence);
            if weight <= 0.0 {
                continue;
            }
            let projected =
                project_weak_perspective(base_vertices.values()[row.gnm_sparse_index], projection);
            for axis in 0..2 {
                let residual = f64::from(sample.observation.normalized_xy()[point_index][axis])
                    - f64::from(projected[axis]);
                for left in 0..basis.len() {
                    let left_value = f64::from(basis[left][row.gnm_sparse_index][axis]);
                    self.rhs[left] += weight * left_value * residual;
                    for (right, right_basis) in basis.iter().enumerate() {
                        self.matrix[left][right] += weight
                            * left_value
                            * f64::from(right_basis[row.gnm_sparse_index][axis]);
                    }
                }
            }
        }
    }

    fn solve(
        mut self,
        regularization: f32,
        target: &[f32],
        kind: &'static str,
        max_condition_number: f32,
    ) -> Result<SolveResult, GnmFitterError> {
        if target.len() != self.rhs.len() {
            return Err(GnmFitterError::InvalidSample {
                field: "regularization target",
                reason: "target dimension does not match active subspace",
            });
        }
        for (index, target_value) in target.iter().enumerate() {
            let lambda = f64::from(regularization);
            self.matrix[index][index] += lambda;
            self.rhs[index] += lambda * f64::from(*target_value);
        }
        let (values, estimated_rank, condition_number) = solve_linear(self.matrix, self.rhs)
            .ok_or(GnmFitterError::IllConditioned {
                kind,
                condition_number: f32::INFINITY,
            })?;
        if !condition_number.is_finite() || condition_number > f64::from(max_condition_number) {
            return Err(GnmFitterError::IllConditioned {
                kind,
                condition_number: condition_number as f32,
            });
        }
        Ok(SolveResult {
            values: values.into_iter().map(|value| value as f32).collect(),
            estimated_rank,
            condition_number: condition_number as f32,
        })
    }
}

struct SolveResult {
    values: Vec<f32>,
    estimated_rank: usize,
    condition_number: f32,
}

impl SolveResult {
    fn empty(dimension: usize) -> Self {
        Self {
            values: vec![0.0; dimension],
            estimated_rank: 0,
            condition_number: 1.0,
        }
    }
}

fn solve_linear(mut matrix: Vec<Vec<f64>>, mut rhs: Vec<f64>) -> Option<(Vec<f64>, usize, f64)> {
    let dimension = rhs.len();
    if matrix.len() != dimension || matrix.iter().any(|row| row.len() != dimension) {
        return None;
    }
    let mut pivots = Vec::with_capacity(dimension);
    let mut rank = 0;
    let scale = matrix
        .iter()
        .flat_map(|row| row.iter())
        .map(|value| value.abs())
        .fold(0.0, f64::max)
        .max(1.0);
    for pivot in 0..dimension {
        let mut pivot_row = pivot;
        for row in pivot + 1..dimension {
            if matrix[row][pivot].abs() > matrix[pivot_row][pivot].abs() {
                pivot_row = row;
            }
        }
        let pivot_value = matrix[pivot_row][pivot].abs();
        if pivot_value <= scale * 1.0e-12 {
            continue;
        }
        if pivot_row != pivot {
            matrix.swap(pivot, pivot_row);
            rhs.swap(pivot, pivot_row);
        }
        let diagonal = matrix[pivot][pivot];
        pivots.push(diagonal.abs());
        rank += 1;
        for value in matrix[pivot].iter_mut().skip(pivot) {
            *value /= diagonal;
        }
        rhs[pivot] /= diagonal;
        let normalized = matrix[pivot].clone();
        for row in 0..dimension {
            if row == pivot {
                continue;
            }
            let factor = matrix[row][pivot];
            for (column, value) in matrix[row].iter_mut().enumerate().skip(pivot) {
                *value -= factor * normalized[column];
            }
            rhs[row] -= factor * rhs[pivot];
        }
    }
    if rank != dimension || rhs.iter().any(|value| !value.is_finite()) {
        return None;
    }
    let condition_number = pivots.iter().copied().fold(0.0, f64::max)
        / pivots.iter().copied().fold(f64::INFINITY, f64::min);
    Some((rhs, rank, condition_number))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{DenseArray, GnmModelData, GnmVariant, GnmVersion, SparseLandmark};

    fn synthetic_model() -> (GnmModel, SparseLandmarkSet) {
        let vertex_count = 68;
        let mut template = Vec::with_capacity(vertex_count * 3);
        for index in 0..vertex_count {
            let angle = index as f32 * 0.19;
            template.extend_from_slice(&[
                0.35 * angle.cos(),
                0.25 * angle.sin(),
                0.08 * (index as f32 * 0.31).sin(),
            ]);
        }
        let mut identity = vec![0.0; 253 * vertex_count * 3];
        let mut expression = vec![0.0; 383 * vertex_count * 3];
        for index in 0..vertex_count {
            identity[index * 3] = 0.06 * (index as f32 * 0.17).sin();
            identity[index * 3 + 1] = 0.04 * (index as f32 * 0.23).cos();
            expression[index * 3] = 0.07 * (index as f32 * 0.13).cos();
            expression[index * 3 + 1] = 0.05 * (index as f32 * 0.29).sin();
        }
        let model = GnmModel::from_data(GnmModelData {
            version: GnmVersion { major: 3, minor: 0 },
            variant: GnmVariant::Head,
            template_vertices: DenseArray::new("vertices", vec![vertex_count, 3], template)
                .unwrap(),
            template_joints: DenseArray::new("joints", vec![1, 3], vec![0.0; 3]).unwrap(),
            vertex_identity_basis: DenseArray::new(
                "identity",
                vec![253, vertex_count, 3],
                identity,
            )
            .unwrap(),
            joint_identity_basis: DenseArray::new(
                "joint_identity",
                vec![253, 1, 3],
                vec![0.0; 253 * 3],
            )
            .unwrap(),
            expression_basis: DenseArray::new("expression", vec![383, vertex_count, 3], expression)
                .unwrap(),
            joint_parent_indices: vec![-1],
            skinning_weights: DenseArray::new(
                "weights",
                vec![1, vertex_count],
                vec![1.0; vertex_count],
            )
            .unwrap(),
            pose_correctives_regressor: None,
        })
        .unwrap();
        let landmarks = SparseLandmarkSet::new(
            (0..vertex_count)
                .map(|index| SparseLandmark::new([index, index, index], [1.0, 0.0, 0.0]).unwrap())
                .collect(),
        )
        .unwrap();
        (model, landmarks)
    }

    fn sample(
        model: &GnmModel,
        landmarks: &SparseLandmarkSet,
        identity_value: f32,
        expression_value: f32,
        source_seq: u64,
    ) -> GnmFittingSample {
        let mut identity = model.neutral_identity();
        let mut identity_values = identity.values().to_vec();
        identity_values[0] = identity_value;
        identity = GnmIdentityState::new(identity_values, model.identity_dimension()).unwrap();
        let mut expression = model.neutral_expression();
        let mut expression_values = expression.values().to_vec();
        expression_values[0] = expression_value;
        expression =
            crate::GnmExpressionState::new(expression_values, model.expression_dimension())
                .unwrap();
        let mut vertices = GnmSparseVertices::with_len(landmarks.len());
        model
            .evaluate_sparse(
                &identity,
                &expression,
                &GnmJointState::neutral(model.joint_count()),
                landmarks,
                &mut vertices,
            )
            .unwrap();
        let projection = GnmProjectionModel {
            yaw: 0.04,
            pitch: -0.02,
            roll: 0.03,
            scale: 0.9,
            translation: [0.5, 0.52],
        };
        let points = DEFAULT_MEDIAPIPE_TO_GNM_MAP
            .iter()
            .map(|row| {
                project_weak_perspective(vertices.values()[row.gnm_sparse_index], projection)
            })
            .collect();
        GnmFittingSample::new(
            source_seq,
            source_seq * 33_333_333,
            GnmSparseObservation::new(points, vec![1.0; 68]).unwrap(),
            1.0,
        )
        .unwrap()
    }

    fn fitter<'a>(model: &'a GnmModel, landmarks: &'a SparseLandmarkSet) -> GnmFaceFitter<'a> {
        GnmFaceFitter::new(
            model,
            landmarks,
            GnmFitterConfig {
                active_identity_dimension: 1,
                active_expression_dimension: 1,
                max_iterations: 4,
                ..GnmFitterConfig::default()
            },
        )
        .unwrap()
    }

    #[test]
    fn insufficient_calibration_is_typed_and_identity_is_fixed() {
        let (model, landmarks) = synthetic_model();
        let mut fitter = fitter(&model, &landmarks);
        let short = [sample(&model, &landmarks, 0.2, 0.0, 1)];
        assert!(matches!(
            fitter.calibrate_neutral(&short),
            Err(GnmFitterError::InsufficientCalibrationSamples { .. })
        ));
        let samples = [
            sample(&model, &landmarks, 0.2, 0.0, 1),
            sample(&model, &landmarks, 0.2, 0.0, 2),
            sample(&model, &landmarks, 0.2, 0.0, 3),
        ];
        let calibration = fitter.calibrate_neutral(&samples).unwrap();
        assert!(calibration.valid);
        assert_eq!(calibration.active_dimension, 1);
        let state = fitter
            .fit_expression(&sample(&model, &landmarks, 0.2, 0.7, 4))
            .unwrap();
        assert_eq!(state.identity, calibration.coefficients);
        assert!(state.expression.values()[0] > 0.0);
        assert!(
            state.expression.values()[1..]
                .iter()
                .all(|value| *value == 0.0)
        );
    }

    #[test]
    fn expression_fit_is_temporal_and_sequence_gap_resets_prior() {
        let (model, landmarks) = synthetic_model();
        let mut fitter = fitter(&model, &landmarks);
        let calibration_samples = [
            sample(&model, &landmarks, 0.0, 0.0, 1),
            sample(&model, &landmarks, 0.0, 0.0, 2),
            sample(&model, &landmarks, 0.0, 0.0, 3),
        ];
        fitter.calibrate_neutral(&calibration_samples).unwrap();
        let first = fitter
            .fit_expression(&sample(&model, &landmarks, 0.0, 0.1, 4))
            .unwrap();
        let second = fitter
            .fit_expression(&sample(&model, &landmarks, 0.0, 0.9, 5))
            .unwrap();
        assert!(second.expression.values()[0] > first.expression.values()[0]);
        let gap = fitter
            .fit_expression(&sample(&model, &landmarks, 0.0, 0.0, 8))
            .unwrap();
        assert!(gap.expression.values()[0] < second.expression.values()[0]);
        assert!(gap.reprojection_rms.is_finite());
    }

    #[test]
    fn uncalibrated_and_regressed_inputs_are_rejected() {
        let (model, landmarks) = synthetic_model();
        let mut fitter = fitter(&model, &landmarks);
        assert!(matches!(
            fitter.fit_expression(&sample(&model, &landmarks, 0.0, 0.0, 1)),
            Err(GnmFitterError::Uncalibrated)
        ));
        let calibration_samples = [
            sample(&model, &landmarks, 0.0, 0.0, 1),
            sample(&model, &landmarks, 0.0, 0.0, 2),
            sample(&model, &landmarks, 0.0, 0.0, 3),
        ];
        fitter.calibrate_neutral(&calibration_samples).unwrap();
        fitter
            .fit_expression(&sample(&model, &landmarks, 0.0, 0.0, 4))
            .unwrap();
        assert!(matches!(
            fitter.fit_expression(&sample(&model, &landmarks, 0.0, 0.0, 4)),
            Err(GnmFitterError::SequenceRegression { .. })
        ));
    }
}

//! Pure motion/confidence-adaptive temporal-weight policy.
//!
//! The policy produces normalized temporal strengths for coarse parameter groups.
//! It does not know about a specific solver or GNM latent dimension. A later GNM
//! fitter can map these normalized strengths onto validated per-group lambda
//! ranges without scattering policy branches through the solver loop.

/// Normalized temporal-prior strengths for coarse face-state parameter groups.
///
/// `0.0` means weakest temporal prior and `1.0` means strongest temporal prior.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TemporalGroupWeights {
    /// Expression-latent temporal strength.
    pub expression: f64,
    /// Eye/jaw or other articulated-joint temporal strength.
    pub joints: f64,
    /// Rigid head-pose temporal strength.
    pub head_pose: f64,
    /// Root/camera-related translation temporal strength.
    pub translation: f64,
}

impl TemporalGroupWeights {
    /// Creates a group weight vector after validating every component in `[0, 1]`.
    pub fn new(
        expression: f64,
        joints: f64,
        head_pose: f64,
        translation: f64,
    ) -> Result<Self, AdaptiveTemporalError> {
        let weights = Self {
            expression,
            joints,
            head_pose,
            translation,
        };
        weights.validate("temporal group weights")?;
        Ok(weights)
    }

    fn validate(self, field: &'static str) -> Result<(), AdaptiveTemporalError> {
        for (group, value) in [
            ("expression", self.expression),
            ("joints", self.joints),
            ("head_pose", self.head_pose),
            ("translation", self.translation),
        ] {
            if !value.is_finite() || !(0.0..=1.0).contains(&value) {
                return Err(AdaptiveTemporalError::InvalidConfig {
                    field,
                    reason: format!("{group} must be finite and within [0, 1]"),
                });
            }
        }
        Ok(())
    }

    fn lerp(self, other: Self, amount: f64) -> Self {
        Self {
            expression: lerp(self.expression, other.expression, amount),
            joints: lerp(self.joints, other.joints, amount),
            head_pose: lerp(self.head_pose, other.head_pose, amount),
            translation: lerp(self.translation, other.translation, amount),
        }
    }
}

/// Observation-health hint supplied by the tracking lifecycle.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TemporalObservationHealth {
    /// No lifecycle-level degradation is known.
    Nominal,
    /// Observation is known to be degraded even if a scalar quality score is absent.
    Degraded,
}

/// One frame of normalized policy inputs.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AdaptiveTemporalInput {
    /// Monotonic frame delta in seconds.
    pub dt_seconds: f64,
    /// Non-negative normalized non-rigid/expression motion score.
    pub expression_motion: f64,
    /// Non-negative normalized rigid head/root motion score.
    pub rigid_motion: f64,
    /// Optional normalized observation quality in `[0, 1]`, where one is best.
    /// Absence means unavailable, not zero and not one.
    pub observation_quality: Option<f64>,
    /// Lifecycle-level observation health.
    pub observation_health: TemporalObservationHealth,
}

/// Tunable, validated policy parameters.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AdaptiveTemporalConfig {
    still_weights: TemporalGroupWeights,
    active_weights: TemporalGroupWeights,
    degraded_weights: TemporalGroupWeights,
    expression_motion_start: f64,
    expression_motion_full: f64,
    rigid_motion_start: f64,
    rigid_motion_full: f64,
    quality_degraded_start: f64,
    quality_degraded_full: f64,
    max_strengthen_per_second: f64,
    max_weaken_per_second: f64,
    max_dt_seconds: f64,
}

impl AdaptiveTemporalConfig {
    /// Creates an adaptive policy configuration.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        still_weights: TemporalGroupWeights,
        active_weights: TemporalGroupWeights,
        degraded_weights: TemporalGroupWeights,
        expression_motion_start: f64,
        expression_motion_full: f64,
        rigid_motion_start: f64,
        rigid_motion_full: f64,
        quality_degraded_start: f64,
        quality_degraded_full: f64,
        max_strengthen_per_second: f64,
        max_weaken_per_second: f64,
        max_dt_seconds: f64,
    ) -> Result<Self, AdaptiveTemporalError> {
        still_weights.validate("still_weights")?;
        active_weights.validate("active_weights")?;
        degraded_weights.validate("degraded_weights")?;
        validate_motion_range(
            "expression_motion",
            expression_motion_start,
            expression_motion_full,
        )?;
        validate_motion_range("rigid_motion", rigid_motion_start, rigid_motion_full)?;
        if !quality_degraded_start.is_finite()
            || !quality_degraded_full.is_finite()
            || !(0.0..=1.0).contains(&quality_degraded_start)
            || !(0.0..=1.0).contains(&quality_degraded_full)
            || quality_degraded_full >= quality_degraded_start
        {
            return Err(AdaptiveTemporalError::InvalidConfig {
                field: "quality_degraded_range",
                reason:
                    "full-degraded quality must be lower than degraded-start quality within [0, 1]"
                        .to_owned(),
            });
        }
        for (field, value) in [
            ("max_strengthen_per_second", max_strengthen_per_second),
            ("max_weaken_per_second", max_weaken_per_second),
            ("max_dt_seconds", max_dt_seconds),
        ] {
            if !value.is_finite() || value <= 0.0 {
                return Err(AdaptiveTemporalError::InvalidConfig {
                    field,
                    reason: "must be finite and positive".to_owned(),
                });
            }
        }
        Ok(Self {
            still_weights,
            active_weights,
            degraded_weights,
            expression_motion_start,
            expression_motion_full,
            rigid_motion_start,
            rigid_motion_full,
            quality_degraded_start,
            quality_degraded_full,
            max_strengthen_per_second,
            max_weaken_per_second,
            max_dt_seconds,
        })
    }

    /// Returns the configured strong/still weights.
    pub fn still_weights(self) -> TemporalGroupWeights {
        self.still_weights
    }

    /// Returns the configured fast-motion weights.
    pub fn active_weights(self) -> TemporalGroupWeights {
        self.active_weights
    }

    /// Returns the configured degraded-observation weights.
    pub fn degraded_weights(self) -> TemporalGroupWeights {
        self.degraded_weights
    }
}

/// Diagnostic regime for one adaptive policy result.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AdaptiveTemporalRegime {
    /// Low motion with no known observation degradation.
    Still,
    /// Genuine motion is strong enough to weaken at least one temporal group.
    Active,
    /// Low quality or lifecycle degradation is strengthening the prior.
    Degraded,
}

/// Pure policy state carried between frames for bounded rate-of-change.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AdaptiveTemporalState {
    /// Applied normalized weights after rate limiting.
    pub weights: TemporalGroupWeights,
    /// Diagnostic regime for the current input.
    pub regime: AdaptiveTemporalRegime,
    /// Continuous non-rigid motion activation in `[0, 1]`.
    pub expression_motion_factor: f64,
    /// Continuous rigid-motion activation in `[0, 1]`.
    pub rigid_motion_factor: f64,
    /// Continuous degradation activation in `[0, 1]`.
    pub degradation_factor: f64,
    /// Whether a scalar observation-quality signal was actually available.
    pub observation_quality_available: bool,
}

/// Advances the adaptive policy without mutating hidden state.
///
/// Passing the previous returned state provides bounded rate-of-change. Passing
/// `None` initializes directly at the current target and is intended for the
/// first frame after lifecycle initialization/reset.
pub fn advance_adaptive_temporal_policy(
    previous: Option<AdaptiveTemporalState>,
    input: AdaptiveTemporalInput,
    config: AdaptiveTemporalConfig,
) -> Result<AdaptiveTemporalState, AdaptiveTemporalError> {
    validate_input(input, config)?;

    let expression_motion_factor = smoothstep_range(
        input.expression_motion,
        config.expression_motion_start,
        config.expression_motion_full,
    );
    let rigid_motion_factor = smoothstep_range(
        input.rigid_motion,
        config.rigid_motion_start,
        config.rigid_motion_full,
    );

    let quality_degradation = input
        .observation_quality
        .map(|quality| {
            reverse_smoothstep_range(
                quality,
                config.quality_degraded_full,
                config.quality_degraded_start,
            )
        })
        .unwrap_or(0.0);
    let lifecycle_degradation = match input.observation_health {
        TemporalObservationHealth::Nominal => 0.0,
        TemporalObservationHealth::Degraded => 1.0,
    };
    let degradation_factor = quality_degradation.max(lifecycle_degradation);

    let motion_target = TemporalGroupWeights {
        expression: lerp(
            config.still_weights.expression,
            config.active_weights.expression,
            expression_motion_factor,
        ),
        joints: lerp(
            config.still_weights.joints,
            config.active_weights.joints,
            expression_motion_factor,
        ),
        head_pose: lerp(
            config.still_weights.head_pose,
            config.active_weights.head_pose,
            rigid_motion_factor,
        ),
        translation: lerp(
            config.still_weights.translation,
            config.active_weights.translation,
            rigid_motion_factor,
        ),
    };
    let target = motion_target.lerp(config.degraded_weights, degradation_factor);

    let weights = match previous {
        Some(previous) => rate_limit_weights(
            previous.weights,
            target,
            input.dt_seconds,
            config.max_strengthen_per_second,
            config.max_weaken_per_second,
        ),
        None => target,
    };

    let regime = if degradation_factor >= 0.5 {
        AdaptiveTemporalRegime::Degraded
    } else if expression_motion_factor.max(rigid_motion_factor) >= 0.5 {
        AdaptiveTemporalRegime::Active
    } else {
        AdaptiveTemporalRegime::Still
    };

    Ok(AdaptiveTemporalState {
        weights,
        regime,
        expression_motion_factor,
        rigid_motion_factor,
        degradation_factor,
        observation_quality_available: input.observation_quality.is_some(),
    })
}

fn validate_input(
    input: AdaptiveTemporalInput,
    config: AdaptiveTemporalConfig,
) -> Result<(), AdaptiveTemporalError> {
    if !input.dt_seconds.is_finite()
        || input.dt_seconds <= 0.0
        || input.dt_seconds > config.max_dt_seconds
    {
        return Err(AdaptiveTemporalError::InvalidInput {
            field: "dt_seconds",
            reason: format!(
                "must be finite, positive, and no greater than {}",
                config.max_dt_seconds
            ),
        });
    }
    for (field, value) in [
        ("expression_motion", input.expression_motion),
        ("rigid_motion", input.rigid_motion),
    ] {
        if !value.is_finite() || value < 0.0 {
            return Err(AdaptiveTemporalError::InvalidInput {
                field,
                reason: "must be finite and non-negative".to_owned(),
            });
        }
    }
    if let Some(quality) = input.observation_quality
        && (!quality.is_finite() || !(0.0..=1.0).contains(&quality))
    {
        return Err(AdaptiveTemporalError::InvalidInput {
            field: "observation_quality",
            reason: "must be finite and within [0, 1] when available".to_owned(),
        });
    }
    Ok(())
}

fn validate_motion_range(
    field: &'static str,
    start: f64,
    full: f64,
) -> Result<(), AdaptiveTemporalError> {
    if !start.is_finite() || !full.is_finite() || start < 0.0 || full <= start {
        return Err(AdaptiveTemporalError::InvalidConfig {
            field,
            reason: "motion start/full must be finite, non-negative, and full must exceed start"
                .to_owned(),
        });
    }
    Ok(())
}

fn smoothstep_range(value: f64, start: f64, full: f64) -> f64 {
    let normalized = ((value - start) / (full - start)).clamp(0.0, 1.0);
    normalized * normalized * (3.0 - 2.0 * normalized)
}

fn reverse_smoothstep_range(value: f64, full: f64, start: f64) -> f64 {
    1.0 - smoothstep_range(value, full, start)
}

fn rate_limit_weights(
    previous: TemporalGroupWeights,
    target: TemporalGroupWeights,
    dt_seconds: f64,
    strengthen_per_second: f64,
    weaken_per_second: f64,
) -> TemporalGroupWeights {
    TemporalGroupWeights {
        expression: rate_limit_scalar(
            previous.expression,
            target.expression,
            dt_seconds,
            strengthen_per_second,
            weaken_per_second,
        ),
        joints: rate_limit_scalar(
            previous.joints,
            target.joints,
            dt_seconds,
            strengthen_per_second,
            weaken_per_second,
        ),
        head_pose: rate_limit_scalar(
            previous.head_pose,
            target.head_pose,
            dt_seconds,
            strengthen_per_second,
            weaken_per_second,
        ),
        translation: rate_limit_scalar(
            previous.translation,
            target.translation,
            dt_seconds,
            strengthen_per_second,
            weaken_per_second,
        ),
    }
}

fn rate_limit_scalar(
    previous: f64,
    target: f64,
    dt_seconds: f64,
    strengthen_per_second: f64,
    weaken_per_second: f64,
) -> f64 {
    let delta = target - previous;
    let max_delta = if delta >= 0.0 {
        strengthen_per_second * dt_seconds
    } else {
        weaken_per_second * dt_seconds
    };
    previous + delta.clamp(-max_delta, max_delta)
}

fn lerp(left: f64, right: f64, amount: f64) -> f64 {
    left + (right - left) * amount
}

/// Typed validation error for the adaptive temporal policy.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum AdaptiveTemporalError {
    /// Configuration is invalid.
    InvalidConfig {
        /// Invalid field or field group.
        field: &'static str,
        /// Validation reason.
        reason: String,
    },
    /// Per-frame input is invalid.
    InvalidInput {
        /// Invalid input field.
        field: &'static str,
        /// Validation reason.
        reason: String,
    },
}

impl std::fmt::Display for AdaptiveTemporalError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidConfig { field, reason } => {
                write!(
                    formatter,
                    "invalid adaptive temporal config {field}: {reason}"
                )
            }
            Self::InvalidInput { field, reason } => {
                write!(
                    formatter,
                    "invalid adaptive temporal input {field}: {reason}"
                )
            }
        }
    }
}

impl std::error::Error for AdaptiveTemporalError {}

#[cfg(test)]
mod tests {
    use super::*;

    fn weights(value: f64) -> TemporalGroupWeights {
        TemporalGroupWeights::new(value, value, value, value).unwrap()
    }

    fn config() -> AdaptiveTemporalConfig {
        AdaptiveTemporalConfig::new(
            weights(0.9),
            weights(0.2),
            weights(1.0),
            0.10,
            0.60,
            0.10,
            0.60,
            0.70,
            0.30,
            2.0,
            8.0,
            0.20,
        )
        .unwrap()
    }

    fn input(expression_motion: f64, rigid_motion: f64) -> AdaptiveTemporalInput {
        AdaptiveTemporalInput {
            dt_seconds: 1.0 / 60.0,
            expression_motion,
            rigid_motion,
            observation_quality: Some(1.0),
            observation_health: TemporalObservationHealth::Nominal,
        }
    }

    fn close(actual: f64, expected: f64) {
        assert!(
            (actual - expected).abs() < 1.0e-9,
            "expected {expected}, got {actual}"
        );
    }

    #[test]
    fn still_good_observation_uses_strong_weights() {
        let state = advance_adaptive_temporal_policy(None, input(0.0, 0.0), config()).unwrap();
        assert_eq!(state.regime, AdaptiveTemporalRegime::Still);
        close(state.weights.expression, 0.9);
        close(state.weights.head_pose, 0.9);
        close(state.degradation_factor, 0.0);
    }

    #[test]
    fn expression_and_rigid_motion_affect_separate_groups() {
        let expression = advance_adaptive_temporal_policy(None, input(1.0, 0.0), config()).unwrap();
        close(expression.weights.expression, 0.2);
        close(expression.weights.joints, 0.2);
        close(expression.weights.head_pose, 0.9);
        close(expression.weights.translation, 0.9);

        let rigid = advance_adaptive_temporal_policy(None, input(0.0, 1.0), config()).unwrap();
        close(rigid.weights.expression, 0.9);
        close(rigid.weights.joints, 0.9);
        close(rigid.weights.head_pose, 0.2);
        close(rigid.weights.translation, 0.2);
    }

    #[test]
    fn low_quality_strengthens_prior_even_during_motion() {
        let mut degraded = input(1.0, 1.0);
        degraded.observation_quality = Some(0.0);
        let state = advance_adaptive_temporal_policy(None, degraded, config()).unwrap();
        assert_eq!(state.regime, AdaptiveTemporalRegime::Degraded);
        close(state.degradation_factor, 1.0);
        close(state.weights.expression, 1.0);
        close(state.weights.head_pose, 1.0);
    }

    #[test]
    fn unavailable_quality_is_not_reported_as_observed_zero_or_one() {
        let mut no_quality = input(0.0, 0.0);
        no_quality.observation_quality = None;
        let state = advance_adaptive_temporal_policy(None, no_quality, config()).unwrap();
        assert!(!state.observation_quality_available);
        close(state.degradation_factor, 0.0);
        close(state.weights.expression, 0.9);
    }

    #[test]
    fn lifecycle_degraded_hint_works_without_scalar_quality() {
        let mut degraded = input(0.0, 0.0);
        degraded.observation_quality = None;
        degraded.observation_health = TemporalObservationHealth::Degraded;
        let state = advance_adaptive_temporal_policy(None, degraded, config()).unwrap();
        assert_eq!(state.regime, AdaptiveTemporalRegime::Degraded);
        close(state.weights.expression, 1.0);
    }

    #[test]
    fn fast_motion_can_weaken_more_quickly_than_stillness_strengthens() {
        let still = advance_adaptive_temporal_policy(None, input(0.0, 0.0), config()).unwrap();
        let active =
            advance_adaptive_temporal_policy(Some(still), input(1.0, 1.0), config()).unwrap();
        assert!(active.weights.expression < 0.8);

        let recovering =
            advance_adaptive_temporal_policy(Some(active), input(0.0, 0.0), config()).unwrap();
        assert!(recovering.weights.expression > active.weights.expression);
        assert!(recovering.weights.expression - active.weights.expression < 0.04);
    }

    #[test]
    fn continuous_mapping_and_rate_limit_prevent_threshold_thrashing() {
        let below = advance_adaptive_temporal_policy(None, input(0.099, 0.0), config()).unwrap();
        let just_above =
            advance_adaptive_temporal_policy(Some(below), input(0.101, 0.0), config()).unwrap();
        assert!((just_above.weights.expression - below.weights.expression).abs() < 0.01);
    }

    #[test]
    fn equivalent_elapsed_time_is_nearly_frame_rate_independent() {
        fn simulate(fps: usize) -> f64 {
            let configuration = config();
            let mut active_input = input(1.0, 1.0);
            active_input.dt_seconds = 1.0 / fps as f64;
            let mut state =
                advance_adaptive_temporal_policy(None, active_input, configuration).unwrap();
            let mut still_input = input(0.0, 0.0);
            still_input.dt_seconds = 1.0 / fps as f64;
            for _ in 0..fps / 2 {
                state = advance_adaptive_temporal_policy(Some(state), still_input, configuration)
                    .unwrap();
            }
            state.weights.expression
        }

        let at_30 = simulate(30);
        let at_60 = simulate(60);
        let at_120 = simulate(120);
        assert!((at_30 - at_60).abs() < 1.0e-9);
        assert!((at_60 - at_120).abs() < 1.0e-9);
    }

    #[test]
    fn invalid_or_huge_dt_is_rejected_for_lifecycle_reset() {
        let mut invalid = input(0.0, 0.0);
        invalid.dt_seconds = 0.0;
        assert!(matches!(
            advance_adaptive_temporal_policy(None, invalid, config()),
            Err(AdaptiveTemporalError::InvalidInput {
                field: "dt_seconds",
                ..
            })
        ));

        invalid.dt_seconds = 1.0;
        assert!(advance_adaptive_temporal_policy(None, invalid, config()).is_err());
    }

    #[test]
    fn same_input_and_state_are_deterministic_and_bounded() {
        let previous = advance_adaptive_temporal_policy(None, input(0.0, 0.0), config()).unwrap();
        let next_a =
            advance_adaptive_temporal_policy(Some(previous), input(0.4, 0.5), config()).unwrap();
        let next_b =
            advance_adaptive_temporal_policy(Some(previous), input(0.4, 0.5), config()).unwrap();
        assert_eq!(next_a, next_b);
        for value in [
            next_a.weights.expression,
            next_a.weights.joints,
            next_a.weights.head_pose,
            next_a.weights.translation,
        ] {
            assert!(value.is_finite());
            assert!((0.0..=1.0).contains(&value));
        }
    }
}

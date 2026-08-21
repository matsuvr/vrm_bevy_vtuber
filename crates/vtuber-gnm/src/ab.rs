//! Deterministic numeric A/B evaluation for Direct MediaPipe and GNM output.
//!
//! This module compares already-produced numeric frames only. It never reads
//! camera pixels, declares that one path is better, or runs either inference
//! implementation. That keeps the evaluator suitable for synthetic sequence
//! tests and bounded local diagnostics.

use std::fmt::{Display, Formatter};

use vtuber_core::{ARKIT52_CHANNEL_COUNT, Arkit52Coefficients};

/// One synchronized numeric frame for an A/B comparison.
#[derive(Clone, Debug, PartialEq)]
pub struct GnmAbSample {
    /// Source frame sequence shared by both paths.
    pub source_seq: u64,
    /// Coefficients produced by the existing Direct MediaPipe path.
    pub direct: Arkit52Coefficients,
    /// Coefficients produced by GNM, when the experimental path produced a
    /// valid frame for this source sequence.
    pub gnm: Option<Arkit52Coefficients>,
    /// GNM reprojection RMS for this frame, if GNM produced a frame.
    pub gnm_fit_residual: Option<f32>,
    /// GNM decoder confidence for this frame, if GNM produced a frame.
    pub gnm_decoder_confidence: Option<f32>,
    /// Direct path end-to-end latency in monotonic nanoseconds.
    pub direct_latency_ns: u64,
    /// GNM path end-to-end latency in monotonic nanoseconds, if available.
    pub gnm_latency_ns: Option<u64>,
}

impl GnmAbSample {
    /// Creates one sample after validating optional GNM diagnostics.
    pub fn new(
        source_seq: u64,
        direct: Arkit52Coefficients,
        gnm: Option<Arkit52Coefficients>,
        gnm_fit_residual: Option<f32>,
        gnm_decoder_confidence: Option<f32>,
        direct_latency_ns: u64,
        gnm_latency_ns: Option<u64>,
    ) -> Result<Self, GnmAbError> {
        if source_seq == 0 {
            return Err(GnmAbError::InvalidSample {
                field: "source_seq",
                reason: "source sequence must be non-zero",
            });
        }
        if gnm.is_none()
            && (gnm_fit_residual.is_some()
                || gnm_decoder_confidence.is_some()
                || gnm_latency_ns.is_some())
        {
            return Err(GnmAbError::InvalidSample {
                field: "gnm diagnostics",
                reason: "GNM diagnostics require a GNM output",
            });
        }
        if let Some(residual) = gnm_fit_residual
            && (!residual.is_finite() || residual < 0.0)
        {
            return Err(GnmAbError::InvalidSample {
                field: "gnm_fit_residual",
                reason: "residual must be finite and non-negative",
            });
        }
        if let Some(confidence) = gnm_decoder_confidence
            && (!confidence.is_finite() || !(0.0..=1.0).contains(&confidence))
        {
            return Err(GnmAbError::InvalidSample {
                field: "gnm_decoder_confidence",
                reason: "confidence must be finite and in [0, 1]",
            });
        }
        Ok(Self {
            source_seq,
            direct,
            gnm,
            gnm_fit_residual,
            gnm_decoder_confidence,
            direct_latency_ns,
            gnm_latency_ns,
        })
    }
}

/// Error returned when an A/B input sequence violates its numeric contract.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum GnmAbError {
    /// No samples were supplied.
    Empty,
    /// A sample or optional diagnostic was invalid.
    InvalidSample {
        /// Invalid field name.
        field: &'static str,
        /// Stable reason.
        reason: &'static str,
    },
    /// Source sequences are not strictly increasing.
    SequenceRegression {
        /// Previously accepted sequence.
        previous: u64,
        /// Regressing or duplicate sequence.
        current: u64,
    },
}

impl Display for GnmAbError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Empty => write!(formatter, "A/B evaluation requires at least one sample"),
            Self::InvalidSample { field, reason } => {
                write!(formatter, "invalid A/B sample `{field}`: {reason}")
            }
            Self::SequenceRegression { previous, current } => write!(
                formatter,
                "A/B source sequence is not strictly increasing: {previous} then {current}"
            ),
        }
    }
}

impl std::error::Error for GnmAbError {}

/// Bounded metrics produced by [`GnmAbEvaluator`].
#[derive(Clone, Debug, PartialEq)]
pub struct GnmAbReport {
    /// Number of accepted source frames.
    pub sample_count: usize,
    /// Number of source frames for which both paths produced output.
    pub compared_sample_count: usize,
    /// Per-channel absolute error between Direct and GNM outputs.
    pub per_channel_mae: [f32; ARKIT52_CHANNEL_COUNT],
    /// Per-channel Direct output variance.
    pub direct_variance: [f32; ARKIT52_CHANNEL_COUNT],
    /// Per-channel GNM output variance over compared frames.
    pub gnm_variance: [f32; ARKIT52_CHANNEL_COUNT],
    /// Mean squared first difference energy for Direct output.
    pub direct_first_difference_energy: f32,
    /// Mean squared first difference energy for GNM output.
    pub gnm_first_difference_energy: Option<f32>,
    /// Mean squared second difference energy for Direct output.
    pub direct_second_difference_energy: f32,
    /// Mean squared second difference energy for GNM output.
    pub gnm_second_difference_energy: Option<f32>,
    /// Mean GNM fitting residual over compared frames with a residual.
    pub mean_gnm_fit_residual: Option<f32>,
    /// Mean GNM decoder confidence over compared frames with confidence.
    pub mean_gnm_decoder_confidence: Option<f32>,
    /// Direct latency p50 in nanoseconds.
    pub direct_latency_p50_ns: u64,
    /// Direct latency p95 in nanoseconds.
    pub direct_latency_p95_ns: u64,
    /// GNM latency p50 in nanoseconds, if any GNM frame was compared.
    pub gnm_latency_p50_ns: Option<u64>,
    /// GNM latency p95 in nanoseconds, if any GNM frame was compared.
    pub gnm_latency_p95_ns: Option<u64>,
}

impl GnmAbReport {
    /// Returns false if any generated floating-point metric is non-finite.
    #[must_use]
    pub fn is_finite(&self) -> bool {
        self.per_channel_mae.iter().all(|value| value.is_finite())
            && self.direct_variance.iter().all(|value| value.is_finite())
            && self.gnm_variance.iter().all(|value| value.is_finite())
            && self.direct_first_difference_energy.is_finite()
            && self.gnm_first_difference_energy.is_none_or(f32::is_finite)
            && self.direct_second_difference_energy.is_finite()
            && self.gnm_second_difference_energy.is_none_or(f32::is_finite)
            && self.mean_gnm_fit_residual.is_none_or(f32::is_finite)
            && self.mean_gnm_decoder_confidence.is_none_or(f32::is_finite)
    }
}

/// Stateless evaluator for synchronized Direct/GNM numeric sequences.
#[derive(Clone, Copy, Debug, Default)]
pub struct GnmAbEvaluator;

impl GnmAbEvaluator {
    /// Evaluates a bounded sequence without quality-ranking either path.
    pub fn evaluate(samples: &[GnmAbSample]) -> Result<GnmAbReport, GnmAbError> {
        if samples.is_empty() {
            return Err(GnmAbError::Empty);
        }
        for pair in samples.windows(2) {
            if pair[1].source_seq <= pair[0].source_seq {
                return Err(GnmAbError::SequenceRegression {
                    previous: pair[0].source_seq,
                    current: pair[1].source_seq,
                });
            }
        }

        let direct: Vec<[f32; ARKIT52_CHANNEL_COUNT]> = samples
            .iter()
            .map(|sample| *sample.direct.as_array())
            .collect();
        let gnm: Vec<[f32; ARKIT52_CHANNEL_COUNT]> = samples
            .iter()
            .filter_map(|sample| sample.gnm.map(|value| *value.as_array()))
            .collect();
        let compared_sample_count = gnm.len();
        let mut per_channel_mae = [0.0; ARKIT52_CHANNEL_COUNT];
        for sample in samples {
            if let Some(gnm) = sample.gnm {
                for (index, (direct, gnm)) in sample
                    .direct
                    .as_array()
                    .iter()
                    .zip(gnm.as_array())
                    .enumerate()
                {
                    per_channel_mae[index] += (direct - gnm).abs();
                }
            }
        }
        if compared_sample_count > 0 {
            for value in &mut per_channel_mae {
                *value /= compared_sample_count as f32;
            }
        }

        let (mean_gnm_fit_residual, _) =
            optional_mean(samples.iter().filter_map(|sample| sample.gnm_fit_residual));
        let (mean_gnm_decoder_confidence, _) = optional_mean(
            samples
                .iter()
                .filter_map(|sample| sample.gnm_decoder_confidence),
        );

        let (direct_variance, _) = variance(&direct);
        let (gnm_variance, _) = variance(&gnm);
        let (direct_first_difference_energy, _) = difference_energy(&direct, 1);
        let (direct_second_difference_energy, _) = difference_energy(&direct, 2);
        let gnm_first_difference_energy = nonempty_metric(difference_energy(&gnm, 1));
        let gnm_second_difference_energy = nonempty_metric(difference_energy(&gnm, 2));

        let direct_latencies: Vec<u64> = samples
            .iter()
            .map(|sample| sample.direct_latency_ns)
            .collect();
        let gnm_latencies: Vec<u64> = samples
            .iter()
            .filter_map(|sample| sample.gnm_latency_ns)
            .collect();

        Ok(GnmAbReport {
            sample_count: samples.len(),
            compared_sample_count,
            per_channel_mae,
            direct_variance,
            gnm_variance,
            direct_first_difference_energy,
            gnm_first_difference_energy,
            direct_second_difference_energy,
            gnm_second_difference_energy,
            mean_gnm_fit_residual,
            mean_gnm_decoder_confidence,
            direct_latency_p50_ns: percentile(&direct_latencies, 0.50),
            direct_latency_p95_ns: percentile(&direct_latencies, 0.95),
            gnm_latency_p50_ns: (!gnm_latencies.is_empty())
                .then(|| percentile(&gnm_latencies, 0.50)),
            gnm_latency_p95_ns: (!gnm_latencies.is_empty())
                .then(|| percentile(&gnm_latencies, 0.95)),
        })
    }

    /// Constructs a report from an iterator without allocating an unbounded
    /// producer-side queue. The caller controls the bounded input slice.
    pub fn evaluate_iter<I>(samples: I) -> Result<GnmAbReport, GnmAbError>
    where
        I: IntoIterator<Item = GnmAbSample>,
    {
        let samples: Vec<_> = samples.into_iter().collect();
        Self::evaluate(&samples)
    }
}

fn optional_mean<I>(values: I) -> (Option<f32>, usize)
where
    I: IntoIterator<Item = f32>,
{
    let mut sum = 0.0f64;
    let mut count = 0usize;
    for value in values {
        sum += f64::from(value);
        count += 1;
    }
    (count > 0)
        .then(|| (sum / count as f64) as f32)
        .map_or((None, 0), |value| (Some(value), count))
}

fn variance(values: &[[f32; ARKIT52_CHANNEL_COUNT]]) -> ([f32; ARKIT52_CHANNEL_COUNT], usize) {
    if values.is_empty() {
        return ([0.0; ARKIT52_CHANNEL_COUNT], 0);
    }
    let mut mean = [0.0f64; ARKIT52_CHANNEL_COUNT];
    for value in values {
        for (index, component) in value.iter().enumerate() {
            mean[index] += f64::from(*component);
        }
    }
    for component in &mut mean {
        *component /= values.len() as f64;
    }
    let mut result = [0.0f32; ARKIT52_CHANNEL_COUNT];
    for value in values {
        for (index, component) in value.iter().enumerate() {
            let delta = f64::from(*component) - mean[index];
            result[index] += (delta * delta) as f32;
        }
    }
    for component in &mut result {
        *component /= values.len() as f32;
    }
    (result, values.len())
}

fn difference_energy(values: &[[f32; ARKIT52_CHANNEL_COUNT]], order: usize) -> (f32, usize) {
    if order == 0 || values.len() <= order {
        return (0.0, 0);
    }
    let mut sum = 0.0f64;
    let mut count = 0usize;
    for index in order..values.len() {
        if order == 1 {
            for (current, previous) in values[index].iter().zip(&values[index - 1]) {
                let value = current - previous;
                sum += f64::from(value * value);
                count += 1;
            }
        } else {
            for ((current, previous), older) in values[index]
                .iter()
                .zip(&values[index - 1])
                .zip(&values[index - 2])
            {
                let value = current - 2.0 * previous + older;
                sum += f64::from(value * value);
                count += 1;
            }
        }
    }
    ((sum / count as f64) as f32, count)
}

fn nonempty_metric((value, count): (f32, usize)) -> Option<f32> {
    (count > 0).then_some(value)
}

fn percentile(values: &[u64], quantile: f64) -> u64 {
    let mut sorted = values.to_vec();
    sorted.sort_unstable();
    let index = ((sorted.len() as f64 * quantile).ceil() as usize)
        .saturating_sub(1)
        .min(sorted.len().saturating_sub(1));
    sorted[index]
}

#[cfg(test)]
mod tests {
    use super::*;

    fn coefficients(value: f32) -> Arkit52Coefficients {
        Arkit52Coefficients::try_from_array([value; ARKIT52_CHANNEL_COUNT]).unwrap()
    }

    fn sample(sequence: u64, direct: f32, gnm: f32) -> GnmAbSample {
        GnmAbSample::new(
            sequence,
            coefficients(direct),
            Some(coefficients(gnm)),
            Some(0.01 * sequence as f32),
            Some(0.5),
            sequence * 10,
            Some(sequence * 20),
        )
        .unwrap()
    }

    #[test]
    fn deterministic_metrics_are_finite_and_include_jerk_and_latency() {
        let samples = vec![
            sample(1, 0.1, 0.2),
            sample(2, 0.2, 0.3),
            sample(3, 0.4, 0.5),
        ];
        let first = GnmAbEvaluator::evaluate(&samples).unwrap();
        let second = GnmAbEvaluator::evaluate(&samples).unwrap();
        assert_eq!(first, second);
        assert!(first.is_finite());
        assert_eq!(first.compared_sample_count, 3);
        assert!(first.per_channel_mae[0] > 0.0);
        assert!(first.direct_first_difference_energy > 0.0);
        assert!(first.direct_second_difference_energy > 0.0);
        assert_eq!(first.direct_latency_p50_ns, 20);
        assert_eq!(first.direct_latency_p95_ns, 30);
        assert_eq!(first.gnm_latency_p95_ns, Some(60));
    }

    #[test]
    fn direct_only_samples_keep_gnm_metrics_unavailable() {
        let samples = vec![
            GnmAbSample::new(1, coefficients(0.1), None, None, None, 11, None).unwrap(),
            GnmAbSample::new(2, coefficients(0.2), None, None, None, 12, None).unwrap(),
        ];
        let report = GnmAbEvaluator::evaluate(&samples).unwrap();
        assert_eq!(report.compared_sample_count, 0);
        assert_eq!(report.mean_gnm_fit_residual, None);
        assert_eq!(report.gnm_first_difference_energy, None);
        assert_eq!(report.gnm_latency_p95_ns, None);
    }

    #[test]
    fn sequence_and_optional_diagnostic_contracts_are_typed() {
        let duplicate = vec![sample(1, 0.1, 0.2), sample(1, 0.2, 0.3)];
        assert!(matches!(
            GnmAbEvaluator::evaluate(&duplicate),
            Err(GnmAbError::SequenceRegression { .. })
        ));
        assert!(matches!(
            GnmAbSample::new(1, coefficients(0.1), None, Some(0.1), None, 1, None),
            Err(GnmAbError::InvalidSample {
                field: "gnm diagnostics",
                ..
            })
        ));
        assert!(matches!(
            GnmAbSample::new(
                1,
                coefficients(0.1),
                Some(coefficients(0.2)),
                None,
                Some(2.0),
                1,
                Some(1)
            ),
            Err(GnmAbError::InvalidSample {
                field: "gnm_decoder_confidence",
                ..
            })
        ));
    }
}

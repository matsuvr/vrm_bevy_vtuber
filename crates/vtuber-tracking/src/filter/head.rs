//! Quaternion-centered exponential smoothing for head rotation.
//!
//! The filter operates directly on [`UnitQuaternion`] values in the canonical
//! tracking basis described in `DESIGN.md` §11.6. Smoothing in quaternion
//! space avoids Euler-angle wrapping, gimbal-lock singularities, and
//! independent per-axis low-pass artefacts.

use nalgebra::{Quaternion, UnitQuaternion};
use vtuber_core::types::MonoTimeNs;

/// Parameters for the head rotation filter.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct HeadFilterParams {
    /// Smoothing time constant in seconds.
    ///
    /// Smaller values make the filter follow input changes faster. Values
    /// must be positive and finite; non-positive values are clamped to
    /// [`f32::EPSILON`] when the filter runs.
    pub time_constant_sec: f32,
    /// Maximum allowed delta-time in seconds.
    ///
    /// Larger gaps are clamped to this value so that a stale observation
    /// cannot fully snap the output.
    pub max_dt_sec: f32,
}

impl Default for HeadFilterParams {
    fn default() -> Self {
        Self {
            time_constant_sec: 0.05,
            max_dt_sec: 0.5,
        }
    }
}

impl HeadFilterParams {
    /// Returns parameters with the given smoothing time constant and the
    /// default maximum delta-time.
    #[must_use]
    pub fn with_time_constant(time_constant_sec: f32) -> Self {
        Self {
            time_constant_sec,
            ..Self::default()
        }
    }
}

#[derive(Clone, Copy, Debug)]
struct FilterState {
    quat: UnitQuaternion<f32>,
    last_time: MonoTimeNs,
}

/// Quaternion-centered exponential smoothing filter for head rotation.
///
/// The filter maintains an internal quaternion state. On each update it
/// blends the current state toward the new target using spherical linear
/// interpolation (slerp). The blend factor is derived from the elapsed time
/// since the last update and a time constant, making the smoothing
/// independent of the input frame rate.
///
/// The filter handles quaternion sign ambiguity by choosing the sign of the
/// target quaternion that yields the shortest arc from the current state.
/// Switching between `q` and `-q` for the same physical rotation therefore
/// does not produce a discontinuity.
#[derive(Clone, Debug)]
pub struct HeadRotationFilter {
    params: HeadFilterParams,
    state: Option<FilterState>,
}

impl HeadRotationFilter {
    /// Creates a new filter with the given parameters.
    #[must_use]
    pub fn new(params: HeadFilterParams) -> Self {
        Self {
            params,
            state: None,
        }
    }

    /// Returns `true` if the filter has received at least one observation.
    #[must_use]
    pub fn is_initialized(&self) -> bool {
        self.state.is_some()
    }

    /// Resets the filter, discarding all state.
    ///
    /// The next call to [`update`](Self::update) initializes the filter with
    /// that observation.
    pub fn reset(&mut self) {
        self.state = None;
    }

    /// Reacquires tracking, discarding the previous smoothed state.
    ///
    /// For this exponential smoothing filter this is equivalent to
    /// [`reset`](Self::reset). The next observation becomes the new initial
    /// state.
    pub fn reacquire(&mut self) {
        self.reset();
    }

    /// Updates the filter with a new target rotation.
    ///
    /// `timestamp` is expected to be monotonically non-decreasing. If it is
    /// older than the previous observation the elapsed time is treated as
    /// zero, so the output is the current state (or the input if the filter
    /// was just reset).
    ///
    /// # Arguments
    ///
    /// * `target` - Desired rotation in the canonical tracking basis.
    /// * `timestamp` - Monotonic timestamp of the observation.
    ///
    /// # Returns
    ///
    /// The smoothed rotation. This equals `target` on the first update after
    /// a [`reset`](Self::reset) or [`reacquire`](Self::reacquire).
    #[must_use]
    pub fn update(
        &mut self,
        target: UnitQuaternion<f32>,
        timestamp: MonoTimeNs,
    ) -> UnitQuaternion<f32> {
        let Some(state) = self.state else {
            self.state = Some(FilterState {
                quat: target,
                last_time: timestamp,
            });
            return target;
        };

        // Compute elapsed seconds. `saturating_sub` clamps backwards
        // timestamps to zero and avoids overflow for very large differences.
        let dt_ns = timestamp.0.saturating_sub(state.last_time.0);
        let dt_sec = (dt_ns as f32) / 1_000_000_000.0;
        let dt_sec = dt_sec.min(self.params.max_dt_sec).max(0.0);

        // If dt is zero (same timestamp or backwards), keep the current
        // state. This also covers the zero/negative dt acceptance cases.
        if dt_sec <= 0.0 {
            return state.quat;
        }

        // Exponential smoothing coefficient: alpha = 1 - exp(-dt / tau).
        // Clamp tau to avoid division by zero and alpha to [0, 1] for
        // robustness against non-finite parameters.
        let tau = self.params.time_constant_sec.max(f32::EPSILON);
        let alpha = (1.0_f32 - (-dt_sec / tau).exp()).clamp(0.0, 1.0);

        // Choose the quaternion sign that gives the shortest arc.
        let signed_target = choose_shortest_arc(state.quat, target);

        // Slerp toward the signed target.
        let smoothed = state.quat.slerp(&signed_target, alpha);

        self.state = Some(FilterState {
            quat: smoothed,
            last_time: timestamp,
        });

        smoothed
    }

    /// Returns the current smoothed rotation without advancing the filter.
    ///
    /// Returns `None` if the filter has not been initialized.
    #[must_use]
    pub fn current(&self) -> Option<UnitQuaternion<f32>> {
        self.state.map(|s| s.quat)
    }
}

/// Returns `target` or `-target`, whichever is closer to `current`.
#[must_use]
fn choose_shortest_arc(
    current: UnitQuaternion<f32>,
    target: UnitQuaternion<f32>,
) -> UnitQuaternion<f32> {
    let c = current.quaternion();
    let t = target.quaternion();
    let dot = c.w * t.w + c.i * t.i + c.j * t.j + c.k * t.k;
    if dot < 0.0 { negate(target) } else { target }
}

/// Explicitly negates a unit quaternion, preserving unit norm.
#[must_use]
fn negate(q: UnitQuaternion<f32>) -> UnitQuaternion<f32> {
    let inner = q.quaternion();
    UnitQuaternion::from_quaternion(Quaternion::new(-inner.w, -inner.i, -inner.j, -inner.k))
}

#[cfg(test)]
mod tests {
    use super::*;
    use approx::assert_relative_eq;
    use nalgebra::Vector3;

    fn ts(ns: u64) -> MonoTimeNs {
        MonoTimeNs(ns)
    }

    #[test]
    fn first_update_initializes_state() {
        let mut filter = HeadRotationFilter::new(HeadFilterParams::default());
        assert!(!filter.is_initialized());
        let q = UnitQuaternion::from_axis_angle(&Vector3::y_axis(), 0.5);
        let out = filter.update(q, ts(16_666_667));
        assert!(filter.is_initialized());
        assert_relative_eq!(out.quaternion().w, q.quaternion().w, epsilon = 1e-6);
        assert_relative_eq!(out.quaternion().i, q.quaternion().i, epsilon = 1e-6);
        assert_relative_eq!(out.quaternion().j, q.quaternion().j, epsilon = 1e-6);
        assert_relative_eq!(out.quaternion().k, q.quaternion().k, epsilon = 1e-6);
    }

    #[test]
    fn zero_dt_returns_current_state() {
        let mut filter = HeadRotationFilter::new(HeadFilterParams::default());
        let q = UnitQuaternion::from_axis_angle(&Vector3::y_axis(), 0.5);
        let out1 = filter.update(q, ts(1_000_000_000));
        let out2 = filter.update(
            UnitQuaternion::from_axis_angle(&Vector3::y_axis(), -0.5),
            ts(1_000_000_000),
        );
        assert_relative_eq!(out1.quaternion().w, out2.quaternion().w, epsilon = 1e-6);
        assert_relative_eq!(out1.quaternion().i, out2.quaternion().i, epsilon = 1e-6);
        assert_relative_eq!(out1.quaternion().j, out2.quaternion().j, epsilon = 1e-6);
        assert_relative_eq!(out1.quaternion().k, out2.quaternion().k, epsilon = 1e-6);
    }

    #[test]
    fn backwards_timestamp_returns_current_state() {
        let mut filter = HeadRotationFilter::new(HeadFilterParams::default());
        let q = UnitQuaternion::from_axis_angle(&Vector3::y_axis(), 0.5);
        let out1 = filter.update(q, ts(2_000_000_000));
        let out2 = filter.update(
            UnitQuaternion::from_axis_angle(&Vector3::y_axis(), -0.5),
            ts(1_000_000_000),
        );
        assert_relative_eq!(out1.quaternion().w, out2.quaternion().w, epsilon = 1e-6);
        assert_relative_eq!(out1.quaternion().i, out2.quaternion().i, epsilon = 1e-6);
        assert_relative_eq!(out1.quaternion().j, out2.quaternion().j, epsilon = 1e-6);
        assert_relative_eq!(out1.quaternion().k, out2.quaternion().k, epsilon = 1e-6);
    }
}

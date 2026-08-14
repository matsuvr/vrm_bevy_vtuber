//! Always-on procedural idle breathing for the active avatar.
//!
//! Every avatar in the
//! [Ready`](crate::lifecycle::AvatarLifecycleState::Ready) lifecycle state
//! receives a subtle, time-driven breathing motion as an additive translation
//! of the required hips bone. The motion is a smooth non-negative sin-squared
//! envelope with an explicit cycle period; it starts from and returns exactly
//! to the authored/animated base pose at every cycle boundary.
//!
//! # Design contract
//!
//! - Breathing runs after Bevy animation and before direct-pose
//!   bevy_vrm1 BodyTracking, gaze, expressions, node constraints, and
//!   SpringBone (see ADR-004).
//! - The breathing system owns only additive hips translation. It never
//!   writes bone rotation, scale, the scene root, arms, eyes, or the camera.
//! - The waveform is frame-rate independent: an f64 phase accumulator is
//!   advanced by Time::delta_secs and only the final bounded envelope value
//!   is converted to f32.
//! - Breathing is independent of camera availability, control frames,
//!   BodyTrackingPoseInput.active, and tracking confidence.
//! - `RestGlobalTransform` is a global/world-space affine. Binding removes the
//!   immutable avatar-root rest affine before measuring hips height or mapping
//!   the semantic model-space `+Y`/`+Z` vectors into the hips parent local space.
//!   The steady-state update is allocation-free and performs one scalar
//!   evaluation and one small transform composition.
//!
//! # Reference behavior
//!
//! The waveform follows the issue #20 contract (a true period_seconds cycle):
//!
//! ```text
//! phase_01 = (elapsed_seconds / period_seconds) mod 1
//! breath_01 = sin(PI * phase_01)^2
//! ```
//! At peak inhale the semantic model-space offset is
//! +Y * vertical_amplitude + +Z * forward_amplitude (VRM model space: up
//! +Y, forward +Z per ADR-004). That model-space vector is converted
//! through the hips parent's actual rest orientation so non-humanoid
//! intermediate nodes in the ChildOf path are handled exactly. The root rest
//! affine is removed first, so root rotation, translation, and scale do not
//! change the model-space amplitudes or semantic direction.

use bevy::math::{Affine3A, Mat3A};
use bevy::prelude::*;

use crate::binding::AvatarBinding;
use crate::lifecycle::{ActiveAvatar, AvatarGeneration, AvatarLifecycle, AvatarLifecycleState};

/// Default breathing cycle period in seconds.
pub const DEFAULT_BREATHING_PERIOD_SECONDS: f64 = 5.0;
/// Default vertical (model-space +Y) amplitude as a fraction of rest hips height.
pub const DEFAULT_VERTICAL_HEIGHT_FACTOR: f32 = 0.010;
/// Default forward (model-space +Z) amplitude as a fraction of rest hips height.
pub const DEFAULT_FORWARD_HEIGHT_FACTOR: f32 = 0.008;
/// Lower clamp of the vertical amplitude in model-space meters.
pub const VERTICAL_AMPLITUDE_MIN_METERS: f32 = 0.006;
/// Upper clamp of the vertical amplitude in model-space meters.
pub const VERTICAL_AMPLITUDE_MAX_METERS: f32 = 0.0125;
/// Lower clamp of the forward amplitude in model-space meters.
pub const FORWARD_AMPLITUDE_MIN_METERS: f32 = 0.004;
/// Upper clamp of the forward amplitude in model-space meters.
pub const FORWARD_AMPLITUDE_MAX_METERS: f32 = 0.010;

/// Maximum ChildOf depth walked when resolving the hips ancestor path.
///
/// Real VRM hierarchies are far shallower; this is a defensive cycle guard.
const MAX_HIPS_ANCESTOR_DEPTH: usize = 64;

/// Translation distance (meters) below which the current hips translation is
/// considered identical to the previously composed output.
const TRANSLATION_MATCH_EPSILON: f32 = 1.0e-5;

/// Bounded, validated tuning profile for the breathing motion.
///
/// The profile is attached to the active avatar root when binding succeeds so
/// defaults, clamps, tests, and future tuning stay explicit instead of being
/// scattered constants. There is deliberately no user-facing enable/disable
/// toggle in this feature.
#[derive(Component, Clone, Copy, Debug, PartialEq)]
pub struct BreathingProfile {
    /// True cycle period in seconds. Must be finite and positive.
    pub period_seconds: f64,
    /// Vertical amplitude per meter of rest hips height (dimensionless).
    pub vertical_height_factor: f32,
    /// Minimum vertical amplitude in model-space meters.
    pub vertical_min_meters: f32,
    /// Maximum vertical amplitude in model-space meters.
    pub vertical_max_meters: f32,
    /// Forward amplitude per meter of rest hips height (dimensionless).
    pub forward_height_factor: f32,
    /// Minimum forward amplitude in model-space meters.
    pub forward_min_meters: f32,
    /// Maximum forward amplitude in model-space meters.
    pub forward_max_meters: f32,
}

impl Default for BreathingProfile {
    fn default() -> Self {
        Self {
            period_seconds: DEFAULT_BREATHING_PERIOD_SECONDS,
            vertical_height_factor: DEFAULT_VERTICAL_HEIGHT_FACTOR,
            vertical_min_meters: VERTICAL_AMPLITUDE_MIN_METERS,
            vertical_max_meters: VERTICAL_AMPLITUDE_MAX_METERS,
            forward_height_factor: DEFAULT_FORWARD_HEIGHT_FACTOR,
            forward_min_meters: FORWARD_AMPLITUDE_MIN_METERS,
            forward_max_meters: FORWARD_AMPLITUDE_MAX_METERS,
        }
    }
}

impl BreathingProfile {
    /// Validates every field of the profile.
    ///
    /// # Errors
    ///
    /// Returns the first invalid field as a BreathingProfileError.
    pub fn validate(&self) -> Result<(), BreathingProfileError> {
        if !self.period_seconds.is_finite() || self.period_seconds <= 0.0 {
            return Err(BreathingProfileError::InvalidPeriod);
        }
        if !self.vertical_height_factor.is_finite() || self.vertical_height_factor < 0.0 {
            return Err(BreathingProfileError::InvalidVerticalFactor);
        }
        if !valid_bounds(self.vertical_min_meters, self.vertical_max_meters) {
            return Err(BreathingProfileError::InvalidVerticalBounds);
        }
        if !self.forward_height_factor.is_finite() || self.forward_height_factor < 0.0 {
            return Err(BreathingProfileError::InvalidForwardFactor);
        }
        if !valid_bounds(self.forward_min_meters, self.forward_max_meters) {
            return Err(BreathingProfileError::InvalidForwardBounds);
        }
        Ok(())
    }
}

/// Returns true when min/max are finite, non-negative, and ordered.
fn valid_bounds(min: f32, max: f32) -> bool {
    min.is_finite() && max.is_finite() && min >= 0.0 && min <= max
}

/// Errors produced by BreathingProfile::validate.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BreathingProfileError {
    /// The cycle period is not finite or not positive.
    InvalidPeriod,
    /// The vertical height factor is not finite or negative.
    InvalidVerticalFactor,
    /// The vertical amplitude bounds are not finite, negative, or ordered.
    InvalidVerticalBounds,
    /// The forward height factor is not finite or negative.
    InvalidForwardFactor,
    /// The forward amplitude bounds are not finite, negative, or ordered.
    InvalidForwardBounds,
}

impl std::fmt::Display for BreathingProfileError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidPeriod => f.write_str("breathing period must be finite and positive"),
            Self::InvalidVerticalFactor => {
                f.write_str("vertical height factor must be finite and non-negative")
            }
            Self::InvalidVerticalBounds => {
                f.write_str("vertical amplitude bounds must be finite and ordered")
            }
            Self::InvalidForwardFactor => {
                f.write_str("forward height factor must be finite and non-negative")
            }
            Self::InvalidForwardBounds => {
                f.write_str("forward amplitude bounds must be finite and ordered")
            }
        }
    }
}

impl std::error::Error for BreathingProfileError {}

/// Computes the normalized cycle phase for an elapsed time and period.
///
/// The result is (elapsed / period) mod 1 and always lies in [0, 1).
/// Invalid input (non-finite values, non-positive period, negative elapsed
/// time, or a quotient that overflows) yields a neutral 0.0 phase.
#[must_use]
pub fn breathing_phase(elapsed_seconds: f64, period_seconds: f64) -> f64 {
    if !elapsed_seconds.is_finite()
        || elapsed_seconds < 0.0
        || !period_seconds.is_finite()
        || period_seconds <= 0.0
    {
        return 0.0;
    }
    let quotient = elapsed_seconds / period_seconds;
    if !quotient.is_finite() {
        return 0.0;
    }
    quotient.rem_euclid(1.0)
}

/// Evaluates the breathing envelope sin(PI * phase) squared.
///
/// The output is exactly 0.0 at phase 0 (cycle wrap), exactly 1.0 at
/// phase 0.5 (peak inhale), continuous with a continuous first derivative
/// at the wrap and peak, and clamped to [0, 1] for every finite input.
/// Non-finite input yields a neutral 0.0.
#[must_use]
pub fn breathing_envelope(phase_01: f64) -> f64 {
    if !phase_01.is_finite() {
        return 0.0;
    }
    let phase = phase_01.rem_euclid(1.0);
    if phase == 0.0 {
        return 0.0;
    }
    if phase == 0.5 {
        return 1.0;
    }
    let sine = (std::f64::consts::PI * phase).sin();
    (sine * sine).clamp(0.0, 1.0)
}

/// Resolves the bounded vertical and forward amplitudes for a rest hips height.
///
/// Amplitudes scale with the positive hips height resolved in immutable VRM
/// model/rest space, then clamp into the profile's bounds. Returns None for
/// non-finite or non-positive geometry so the caller disables breathing as a
/// bounded safe no-op.
#[must_use]
pub fn resolve_breathing_amplitudes(
    profile: &BreathingProfile,
    rest_hips_height: f32,
) -> Option<(f32, f32)> {
    profile.validate().ok()?;
    if !rest_hips_height.is_finite() || rest_hips_height <= 0.0 {
        return None;
    }
    let vertical = (profile.vertical_height_factor * rest_hips_height)
        .clamp(profile.vertical_min_meters, profile.vertical_max_meters);
    let forward = (profile.forward_height_factor * rest_hips_height)
        .clamp(profile.forward_min_meters, profile.forward_max_meters);
    if !vertical.is_finite() || !forward.is_finite() {
        return None;
    }
    Some((vertical, forward))
}

/// Immutable per-avatar geometry resolved once when binding becomes ready.
///
/// This component is inserted on the active avatar root together with the
/// validated BreathingProfile and a fresh BreathingState. It caches the
/// hips entity, the ChildOf ancestor path up to (excluding) the avatar root,
/// and the model-space-to-parent-local conversion so the steady-state update
/// performs no hierarchy discovery and no allocation.
#[derive(Component, Debug, Clone, PartialEq)]
pub struct BreathingBinding {
    /// Avatar generation this geometry belongs to.
    pub generation: AvatarGeneration,
    /// Hips bone entity receiving the additive translation.
    pub hips: Entity,
    /// Immutable avatar-root global/rest affine captured at binding time.
    ///
    /// This is the `RestGlobalTransform` of the root when available, or the
    /// root's binding-time `GlobalTransform` fallback. It is never replaced by
    /// an animated/current root transform.
    pub root_rest_global: GlobalTransform,
    /// Ancestors from the hips parent up to (excluding) the avatar root,
    /// nearest first. Resolved once from the real ChildOf path.
    pub ancestors: Vec<Entity>,
    /// Hips-parent-local direction whose rest-space global motion is exactly
    /// model-space up (+Y). Includes parent scale compensation.
    pub up_local: Vec3,
    /// Hips-parent-local direction whose rest-space global motion is exactly
    /// model-space forward (+Z). Includes parent scale compensation.
    pub forward_local: Vec3,
    /// Positive hips height measured in VRM model/root space, not world space.
    pub rest_hips_height: f32,
    /// Bounded vertical amplitude in model-space meters.
    pub vertical_amplitude: f32,
    /// Bounded forward amplitude in model-space meters.
    pub forward_amplitude: f32,
}

/// Per-avatar runtime state for the breathing motion.
///
/// The state lives on the avatar root so unloading or replacing an avatar
/// despawns it with the entity: a replacement always starts at neutral phase
/// 0 with no inherited base or delta.
#[derive(Component, Debug, Clone, Copy, PartialEq)]
pub struct BreathingState {
    /// Accumulated phase time in seconds (f64 for long-run precision).
    elapsed_seconds: f64,
    /// The detected animation base hips translation.
    base: Option<Vec3>,
    /// The additive delta written in the previous update.
    last_delta: Vec3,
    /// Whether a first finite base has been captured.
    initialized: bool,
}

impl Default for BreathingState {
    fn default() -> Self {
        Self {
            elapsed_seconds: 0.0,
            base: None,
            last_delta: Vec3::ZERO,
            initialized: false,
        }
    }
}

impl BreathingState {
    /// Current accumulated phase time in seconds.
    #[must_use]
    pub fn elapsed_seconds(&self) -> f64 {
        self.elapsed_seconds
    }

    /// Current detected animation base translation, if a finite value has been
    /// captured yet.
    #[must_use]
    pub fn base(&self) -> Option<Vec3> {
        self.base
    }
}

/// Resolves the immutable breathing geometry for one avatar generation.
///
/// # Arguments
///
/// - root_rest_global: the avatar root's immutable binding-time rest/global
///   affine. This must be the root `RestGlobalTransform`, or the binding-time
///   root `GlobalTransform` fallback when the root has no rest component.
/// - hips_rest_local / hips_rest_global: the hips bone's immutable
///   RestTransform / RestGlobalTransform values. `RestGlobalTransform` is in
///   global/world space and is converted back to model/root space through the
///   cached root affine.
/// - ancestors: the ChildOf path from the hips parent up to (excluding)
///   the avatar root, nearest first (see collect_hips_ancestor_path).
///
/// Returns None when rest data is missing, the model-space hips height is not
/// positive and finite, a rest affine is non-finite or non-invertible, or the
/// converted directions degenerate. The caller then simply does not insert
/// the breathing components, which disables the motion for that avatar as a
/// safe no-op.
#[must_use]
pub fn resolve_breathing_binding(
    generation: AvatarGeneration,
    hips: Entity,
    profile: &BreathingProfile,
    root_rest_global: Option<GlobalTransform>,
    hips_rest_local: Option<Transform>,
    hips_rest_global: Option<GlobalTransform>,
    ancestors: Vec<Entity>,
) -> Option<BreathingBinding> {
    profile.validate().ok()?;
    let root_rest_global = root_rest_global?;
    let rest_local = hips_rest_local?;
    let rest_global = hips_rest_global?;
    let root_affine = root_rest_global.affine();
    let hips_affine = rest_global.affine();
    let local_affine = rest_local.compute_affine();
    if !root_affine.is_finite() || !hips_affine.is_finite() || !local_affine.is_finite() {
        return None;
    }
    let root_inverse = finite_affine_inverse(root_affine)?;
    let parent_global = hips_affine * finite_affine_inverse(local_affine)?;
    if !parent_global.is_finite() {
        return None;
    }
    let parent_in_model = root_inverse * parent_global;
    if !parent_in_model.is_finite() {
        return None;
    }
    let hips_global_position = hips_affine.transform_point3(Vec3::ZERO);
    let hips_model_position = root_inverse.transform_point3(hips_global_position);
    if !hips_model_position.is_finite() {
        return None;
    }
    let rest_hips_height = hips_model_position.y;
    let (vertical_amplitude, forward_amplitude) =
        resolve_breathing_amplitudes(profile, rest_hips_height)?;
    let linear_inverse = finite_linear_inverse(parent_in_model.matrix3)?;
    let up_local = finite_nonzero(linear_inverse.mul_vec3(Vec3::Y))?;
    let forward_local = finite_nonzero(linear_inverse.mul_vec3(Vec3::Z))?;
    Some(BreathingBinding {
        generation,
        hips,
        root_rest_global,
        ancestors,
        up_local,
        forward_local,
        rest_hips_height,
        vertical_amplitude,
        forward_amplitude,
    })
}

/// Returns a finite inverse for a finite affine, treating non-invertible
/// values as a safe no-op.
fn finite_affine_inverse(value: Affine3A) -> Option<Affine3A> {
    if !value.is_finite() {
        return None;
    }
    let inverse = value.inverse();
    inverse.is_finite().then_some(inverse)
}

/// Returns a finite inverse for a finite linear matrix, treating
/// non-invertible values as a safe no-op.
fn finite_linear_inverse(value: Mat3A) -> Option<Mat3A> {
    if !value.is_finite() {
        return None;
    }
    let inverse = value.inverse();
    inverse.is_finite().then_some(inverse)
}

/// Returns the vector when it is finite and non-degenerate.
fn finite_nonzero(value: Vec3) -> Option<Vec3> {
    if value.is_finite() && value.length_squared() > f32::EPSILON {
        Some(value)
    } else {
        None
    }
}

/// Collects the ChildOf ancestors from the hips parent up to (excluding)
/// the avatar root, nearest first.
///
/// The parent lookup is injected so the walk is testable without constructing
/// an ECS Query. Returns None when the chain leaves the world without
/// reaching the root or exceeds MAX_HIPS_ANCESTOR_DEPTH (a defensive cycle
/// guard).
#[must_use]
pub fn collect_hips_ancestor_path(
    hips: Entity,
    root: Entity,
    parent_of: impl Fn(Entity) -> Option<Entity>,
) -> Option<Vec<Entity>> {
    let mut ancestors = Vec::new();
    let mut cursor = hips;
    for _ in 0..MAX_HIPS_ANCESTOR_DEPTH {
        let parent = parent_of(cursor)?;
        if parent == root {
            return Some(ancestors);
        }
        ancestors.push(parent);
        cursor = parent;
    }
    None
}

/// Applies the additive breathing translation to the active avatar's hips.
///
/// # Schedule
///
/// Runs in PostUpdate after Bevy AnimationSystems and before bevy_vrm1's
/// direct-pose BodyTracking (ADR-004).
///
/// # Ownership
///
/// This system is the sole writer of additive hips translation. It composes
/// the current animation base with the current breathing delta
/// (output = base + delta) and never accumulates its own previous output.
/// A translation written by animation or another legitimate upstream owner is
/// captured as the new base. It refreshes the hips GlobalTransform along
/// the cached ancestor path so downstream same-frame consumers (body tracking,
/// constraints, SpringBone) observe the hips result; the full hierarchy is not
/// traversed.
#[allow(clippy::type_complexity)]
pub fn apply_breathing_hips_translation(
    lifecycle: Res<AvatarLifecycle>,
    roots: Query<
        (Entity, &AvatarBinding, &BreathingBinding, &BreathingProfile),
        With<ActiveAvatar>,
    >,
    mut states: Query<&mut BreathingState>,
    mut transforms: Query<(&mut Transform, &mut GlobalTransform)>,
    time: Res<Time>,
) {
    if lifecycle.state() != AvatarLifecycleState::Ready {
        return;
    }

    for (root, binding, geometry, profile) in roots.iter() {
        if geometry.generation != binding.generation {
            continue;
        }
        let Ok(mut state) = states.get_mut(root) else {
            continue;
        };

        // The phase is evaluated BEFORE advancing the accumulator, so the
        // first frame after binding starts exactly at phase 0 (neutral) and
        // produces no position pop. Accumulation is frame-rate independent;
        // invalid or non-monotonic deltas leave the phase unchanged so the
        // same phase always produces the same output.
        let phase = breathing_phase(state.elapsed_seconds, profile.period_seconds);
        let breath = breathing_envelope(phase) as f32;
        let delta_seconds = time.delta_secs() as f64;
        if delta_seconds.is_finite() && delta_seconds >= 0.0 {
            state.elapsed_seconds += delta_seconds;
        }
        let delta = geometry.up_local * (geometry.vertical_amplitude * breath)
            + geometry.forward_local * (geometry.forward_amplitude * breath);

        // Non-finite hips translations are a bounded safe no-op: nothing is
        // written and the base detection is not advanced, so the next finite
        // translation is captured as a fresh base.
        let current = match transforms.get(geometry.hips) {
            Ok((hips_transform, _)) => hips_transform.translation,
            Err(_) => continue,
        };
        if !current.is_finite() {
            continue;
        }

        let base = if state.initialized {
            let previous_base = state.base.unwrap_or(current);
            let expected = previous_base + state.last_delta;
            if (current - expected).length_squared()
                <= TRANSLATION_MATCH_EPSILON * TRANSLATION_MATCH_EPSILON
            {
                previous_base
            } else {
                current
            }
        } else {
            current
        };
        let output = base + delta;
        if !output.is_finite() {
            continue;
        }

        if let Ok((mut hips_transform, _)) = transforms.get_mut(geometry.hips) {
            hips_transform.translation = output;
        }
        state.base = Some(base);
        state.last_delta = delta;
        state.initialized = true;

        // Refresh the hips GlobalTransform along the cached ancestor path so
        // downstream systems consume the same-frame hips result. The root's own
        // global transform is never written by this system.
        let root_global = transforms
            .get(root)
            .map(|(_, global)| *global)
            .unwrap_or(GlobalTransform::IDENTITY);
        let mut parent_global = root_global;
        let mut chain_fresh = true;
        // The cached path is nearest-first (hips parent toward the root), so
        // compose it in the opposite direction from root_global toward hips.
        for &ancestor in geometry.ancestors.iter().rev() {
            let Ok((ancestor_transform, mut ancestor_global)) = transforms.get_mut(ancestor) else {
                chain_fresh = false;
                break;
            };
            *ancestor_global = parent_global.mul_transform(*ancestor_transform);
            parent_global = *ancestor_global;
        }
        if chain_fresh
            && let Ok((hips_transform, mut hips_global)) = transforms.get_mut(geometry.hips)
        {
            *hips_global = parent_global.mul_transform(*hips_transform);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const EPSILON: f64 = 1.0e-9;
    const F32_EPSILON: f32 = 1.0e-6;

    fn entity(id: u32) -> Entity {
        Entity::from_raw_u32(id).expect("test entity index is valid")
    }

    // --- waveform ---

    #[test]
    fn waveform_is_exactly_neutral_at_cycle_boundaries() {
        assert_eq!(breathing_envelope(0.0), 0.0);
        assert_eq!(breathing_envelope(1.0), 0.0);
        assert_eq!(breathing_envelope(2.0), 0.0);
    }

    #[test]
    fn waveform_peaks_exactly_at_half_phase() {
        assert_eq!(breathing_envelope(0.5), 1.0);
        assert_eq!(breathing_envelope(1.5), 1.0);
    }

    #[test]
    fn waveform_is_bounded_and_finite_for_invalid_input() {
        for phase in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY, -0.25, 3.75] {
            let value = breathing_envelope(phase);
            assert!(value.is_finite(), "phase {phase} produced {value}");
            assert!(
                (0.0..=1.0).contains(&value),
                "phase {phase} produced out-of-range {value}"
            );
        }
    }

    #[test]
    fn waveform_matches_sin_squared_reference() {
        for i in 0..=100 {
            let phase = i as f64 / 100.0;
            let value = breathing_envelope(phase);
            let expected = (std::f64::consts::PI * phase).sin().powi(2);
            assert!(
                (value - expected).abs() < EPSILON,
                "phase {phase}: {value} vs {expected}"
            );
        }
    }

    #[test]
    fn waveform_is_continuous_across_wrap() {
        let before = breathing_envelope(1.0 - 1.0e-7);
        let after = breathing_envelope(1.0e-7);
        assert!(before < 1.0e-12, "wrap predecessor {before}");
        assert!(after < 1.0e-12, "wrap successor {after}");
    }

    #[test]
    fn waveform_is_continuous_across_peak() {
        let before = breathing_envelope(0.5 - 1.0e-7);
        let after = breathing_envelope(0.5 + 1.0e-7);
        assert!((1.0 - before).abs() < 1.0e-12, "peak predecessor {before}");
        assert!((1.0 - after).abs() < 1.0e-12, "peak successor {after}");
    }

    #[test]
    fn waveform_inhales_then_exhales() {
        // Increasing on [0, 0.5] (inhale phase), decreasing on [0.5, 1] (exhale).
        let mut previous = 0.0;
        for i in 1..=50 {
            let value = breathing_envelope(i as f64 / 100.0);
            assert!(value >= previous, "inhale should be monotonic at {i}");
            previous = value;
        }
        let mut previous = 1.0;
        for i in 51..=100 {
            let value = breathing_envelope(i as f64 / 100.0);
            assert!(value <= previous, "exhale should be monotonic at {i}");
            previous = value;
        }
    }

    #[test]
    fn phase_uses_true_five_second_default_cycle() {
        let period = DEFAULT_BREATHING_PERIOD_SECONDS;
        assert_eq!(breathing_phase(0.0, period), 0.0);
        assert_eq!(breathing_phase(1.25, period), 0.25);
        assert_eq!(breathing_phase(2.5, period), 0.5);
        assert_eq!(breathing_phase(5.0, period), 0.0);
        assert_eq!(breathing_phase(6.25, period), 0.25);
    }

    #[test]
    fn phase_is_safe_for_invalid_input() {
        assert_eq!(breathing_phase(f64::NAN, 5.0), 0.0);
        assert_eq!(breathing_phase(1.0, 0.0), 0.0);
        assert_eq!(breathing_phase(1.0, -2.0), 0.0);
        assert_eq!(breathing_phase(-1.0, 5.0), 0.0);
        assert_eq!(breathing_phase(f64::INFINITY, 5.0), 0.0);
        // Quotient overflow must not panic or produce NaN.
        assert_eq!(breathing_phase(1.0e308, 1.0e-308), 0.0);
    }

    #[test]
    fn fps_variants_produce_materially_equivalent_values() {
        // Accumulate elapsed time at 30/60/120 fps for the same duration and
        // compare the envelope at the end.
        let simulate = |fps: f64| {
            let dt = 1.0 / fps;
            let frames = (1.2345 * fps).round() as usize;
            let mut elapsed = 0.0;
            let mut value = 0.0;
            for _ in 0..frames {
                elapsed += dt;
                value = breathing_envelope(breathing_phase(elapsed, 5.0));
            }
            value
        };
        let at_30 = simulate(30.0);
        let at_60 = simulate(60.0);
        let at_120 = simulate(120.0);
        assert!(
            (at_30 - at_60).abs() < 0.03,
            "30fps {at_30} vs 60fps {at_60}"
        );
        assert!(
            (at_30 - at_120).abs() < 0.03,
            "30fps {at_30} vs 120fps {at_120}"
        );
        assert!(
            (at_60 - at_120).abs() < 0.03,
            "60fps {at_60} vs 120fps {at_120}"
        );
    }

    // --- profile validation ---

    #[test]
    fn default_profile_is_valid() {
        assert!(BreathingProfile::default().validate().is_ok());
    }

    #[test]
    fn profile_rejects_invalid_period() {
        for period in [0.0, -1.0, f64::NAN, f64::INFINITY] {
            let profile = BreathingProfile {
                period_seconds: period,
                ..BreathingProfile::default()
            };
            assert_eq!(
                profile.validate(),
                Err(BreathingProfileError::InvalidPeriod),
                "period {period}"
            );
        }
    }

    #[test]
    fn profile_rejects_invalid_factors_and_bounds() {
        let profile = BreathingProfile {
            vertical_height_factor: -0.1,
            ..BreathingProfile::default()
        };
        assert_eq!(
            profile.validate(),
            Err(BreathingProfileError::InvalidVerticalFactor)
        );

        let profile = BreathingProfile {
            vertical_min_meters: 1.0,
            ..BreathingProfile::default()
        };
        assert_eq!(
            profile.validate(),
            Err(BreathingProfileError::InvalidVerticalBounds)
        );

        let profile = BreathingProfile {
            vertical_min_meters: f32::NAN,
            ..BreathingProfile::default()
        };
        assert_eq!(
            profile.validate(),
            Err(BreathingProfileError::InvalidVerticalBounds)
        );

        let profile = BreathingProfile {
            forward_height_factor: f32::INFINITY,
            ..BreathingProfile::default()
        };
        assert_eq!(
            profile.validate(),
            Err(BreathingProfileError::InvalidForwardFactor)
        );

        let profile = BreathingProfile {
            forward_max_meters: -1.0,
            ..BreathingProfile::default()
        };
        assert_eq!(
            profile.validate(),
            Err(BreathingProfileError::InvalidForwardBounds)
        );
    }

    // --- amplitude resolution ---

    #[test]
    fn amplitudes_scale_and_clamp_for_small_typical_and_large_rigs() {
        let profile = BreathingProfile::default();

        // Small rig: below the minimum clamp.
        let (small_vertical, small_forward) =
            resolve_breathing_amplitudes(&profile, 0.2).expect("small rig is valid");
        assert!((small_vertical - VERTICAL_AMPLITUDE_MIN_METERS).abs() < F32_EPSILON);
        assert!((small_forward - FORWARD_AMPLITUDE_MIN_METERS).abs() < F32_EPSILON);

        // Typical VRoid-style rig.
        let (typical_vertical, typical_forward) =
            resolve_breathing_amplitudes(&profile, 0.95).expect("typical rig is valid");
        assert!((typical_vertical - 0.0095).abs() < F32_EPSILON);
        assert!((typical_forward - 0.0076).abs() < F32_EPSILON);

        // Very large rig: above the maximum clamp.
        let (large_vertical, large_forward) =
            resolve_breathing_amplitudes(&profile, 5.0).expect("large rig is valid");
        assert!((large_vertical - VERTICAL_AMPLITUDE_MAX_METERS).abs() < F32_EPSILON);
        assert!((large_forward - FORWARD_AMPLITUDE_MAX_METERS).abs() < F32_EPSILON);
    }

    #[test]
    fn amplitudes_reject_invalid_geometry() {
        let profile = BreathingProfile::default();
        for height in [f32::NAN, f32::INFINITY, f32::NEG_INFINITY, 0.0, -1.0] {
            assert!(
                resolve_breathing_amplitudes(&profile, height).is_none(),
                "height {height} must be rejected"
            );
        }
    }

    // --- binding resolution ---

    #[test]
    fn identity_hierarchy_maps_model_axes_to_identity_local_axes() {
        let profile = BreathingProfile::default();
        let hips = entity(1);
        let rest_local = Transform::from_translation(Vec3::new(0.0, 1.0, 0.0));
        let rest_global = GlobalTransform::from(rest_local);
        let binding = resolve_breathing_binding(
            AvatarGeneration(1),
            hips,
            &profile,
            Some(GlobalTransform::IDENTITY),
            Some(rest_local),
            Some(rest_global),
            Vec::new(),
        )
        .expect("identity rest data resolves");
        assert!(binding.up_local.abs_diff_eq(Vec3::Y, F32_EPSILON));
        assert!(binding.forward_local.abs_diff_eq(Vec3::Z, F32_EPSILON));
    }

    #[test]
    fn rotated_root_preserves_model_axes_in_parent_local_space() {
        let profile = BreathingProfile::default();
        let hips = entity(1);
        // Root rest rotation +90 degrees about X changes world coordinates,
        // but model/root-space +Y and +Z remain the semantic axes.
        let root_rotation = Quat::from_rotation_x(std::f32::consts::FRAC_PI_2);
        let rest_local = Transform::from_translation(Vec3::new(0.0, 1.0, 0.0));
        let rest_global = GlobalTransform::from_rotation(root_rotation).mul_transform(rest_local);
        let binding = resolve_breathing_binding(
            AvatarGeneration(1),
            hips,
            &profile,
            Some(GlobalTransform::from_rotation(root_rotation)),
            Some(rest_local),
            Some(rest_global),
            Vec::new(),
        )
        .expect("rotated rest data resolves");

        assert!(
            binding.up_local.abs_diff_eq(Vec3::Y, F32_EPSILON),
            "up_local {0}",
            binding.up_local
        );
        assert!(
            binding.forward_local.abs_diff_eq(Vec3::Z, F32_EPSILON),
            "forward_local {0}",
            binding.forward_local
        );
    }

    #[test]
    fn non_uniform_parent_scale_is_compensated() {
        let profile = BreathingProfile::default();
        let hips = entity(1);
        let rest_local = Transform {
            translation: Vec3::new(0.0, 1.0, 0.0),
            ..Transform::IDENTITY
        };
        let parent_rest = Transform {
            scale: Vec3::new(2.0, 1.0, 0.5),
            translation: Vec3::ZERO,
            ..Transform::IDENTITY
        };
        let rest_global = GlobalTransform::from(parent_rest).mul_transform(rest_local);
        let binding = resolve_breathing_binding(
            AvatarGeneration(1),
            hips,
            &profile,
            Some(GlobalTransform::IDENTITY),
            Some(rest_local),
            Some(rest_global),
            Vec::new(),
        )
        .expect("scaled rest data resolves");

        // Local +Y passes through scale 1 unchanged; local +Z must double to
        // compensate the 0.5 parent scale.
        assert!(binding.up_local.abs_diff_eq(Vec3::Y, F32_EPSILON));
        assert!(
            binding
                .forward_local
                .abs_diff_eq(Vec3::new(0.0, 0.0, 2.0), F32_EPSILON)
        );
    }

    #[test]
    fn degenerate_rest_data_disables_breathing() {
        let profile = BreathingProfile::default();
        let hips = entity(1);
        let valid_local = Transform::from_translation(Vec3::new(0.0, 1.0, 0.0));

        // Missing rest data.
        assert!(
            resolve_breathing_binding(
                AvatarGeneration(1),
                hips,
                &profile,
                Some(GlobalTransform::IDENTITY),
                None,
                Some(GlobalTransform::from(valid_local)),
                Vec::new(),
            )
            .is_none()
        );

        // Non-invertible local scale.
        let zero_scale = Transform {
            scale: Vec3::ZERO,
            translation: Vec3::new(0.0, 1.0, 0.0),
            ..Transform::IDENTITY
        };
        assert!(
            resolve_breathing_binding(
                AvatarGeneration(1),
                hips,
                &profile,
                Some(GlobalTransform::IDENTITY),
                Some(zero_scale),
                Some(GlobalTransform::from(zero_scale)),
                Vec::new(),
            )
            .is_none()
        );

        // Non-positive rest hips height.
        let underground = Transform::from_translation(Vec3::new(0.0, -0.5, 0.0));
        assert!(
            resolve_breathing_binding(
                AvatarGeneration(1),
                hips,
                &profile,
                Some(GlobalTransform::IDENTITY),
                Some(underground),
                Some(GlobalTransform::from(underground)),
                Vec::new(),
            )
            .is_none()
        );

        // Non-finite rest global.
        assert!(
            resolve_breathing_binding(
                AvatarGeneration(1),
                hips,
                &profile,
                Some(GlobalTransform::IDENTITY),
                Some(valid_local),
                Some(GlobalTransform::from(Transform {
                    translation: Vec3::new(f32::NAN, 1.0, 0.0),
                    ..Transform::IDENTITY
                })),
                Vec::new(),
            )
            .is_none()
        );

        // A non-invertible root rest affine must disable breathing without
        // producing NaN or panicking.
        let invalid_root = GlobalTransform::from(Transform {
            scale: Vec3::ZERO,
            ..Transform::IDENTITY
        });
        assert!(
            resolve_breathing_binding(
                AvatarGeneration(1),
                hips,
                &profile,
                Some(invalid_root),
                Some(valid_local),
                Some(GlobalTransform::from(valid_local)),
                Vec::new(),
            )
            .is_none()
        );

        let non_finite_root = GlobalTransform::from(Transform {
            translation: Vec3::new(f32::NAN, 0.0, 0.0),
            ..Transform::IDENTITY
        });
        assert!(
            resolve_breathing_binding(
                AvatarGeneration(1),
                hips,
                &profile,
                Some(non_finite_root),
                Some(valid_local),
                Some(GlobalTransform::from(valid_local)),
                Vec::new(),
            )
            .is_none()
        );
    }

    // --- ancestor path resolution ---

    fn ancestor_world() -> (World, Entity, Entity, Entity, Entity) {
        let mut world = World::new();
        let root = world.spawn_empty().id();
        let a = world.spawn(ChildOf(root)).id();
        let b = world.spawn(ChildOf(a)).id();
        let hips = world.spawn(ChildOf(b)).id();
        (world, root, a, b, hips)
    }

    #[test]
    fn ancestor_path_collects_multiple_intermediate_nodes_nearest_first() {
        let (world, root, a, b, hips) = ancestor_world();
        let path = collect_hips_ancestor_path(hips, root, |entity| {
            world.get::<ChildOf>(entity).map(ChildOf::parent)
        })
        .expect("path reaches root");
        assert_eq!(path, vec![b, a]);
    }

    #[test]
    fn ancestor_path_rejects_cycles_and_detached_chains() {
        let mut world = World::new();
        let root = world.spawn_empty().id();
        let a = world.spawn_empty().id();
        let b = world.spawn_empty().id();
        world.entity_mut(a).insert(ChildOf(b));
        world.entity_mut(b).insert(ChildOf(a));
        assert!(
            collect_hips_ancestor_path(a, root, |entity| {
                world.get::<ChildOf>(entity).map(ChildOf::parent)
            })
            .is_none()
        );

        // Detached hips with no ChildOf.
        let detached = world.spawn_empty().id();
        assert!(
            collect_hips_ancestor_path(detached, root, |entity| {
                world.get::<ChildOf>(entity).map(ChildOf::parent)
            })
            .is_none()
        );
    }
}

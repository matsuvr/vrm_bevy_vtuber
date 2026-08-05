//! VRM 1.0 compatibility gate for `bevy_vrm1`.
//!
//! This module introspects a loaded VRM model and records which runtime
//! capabilities are present. It is used by the compatibility runner and by
//! unit tests to guard against upstream behaviour changes at the pinned
//! revision.

use bevy::prelude::*;
use bevy_vrm1::prelude::*;

/// Plugin that installs compatibility-report systems.
#[derive(Default)]
pub struct VrmCompatibilityPlugin;

impl Plugin for VrmCompatibilityPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, inspect_initialized_vrm);
    }
}

/// Bone capability tuple used when querying a freshly-initialized VRM.
type InitializedVrmBones<'w, 's> = (
    Entity,
    Option<&'static HeadBoneEntity>,
    Option<&'static NeckBoneEntity>,
    Option<&'static LeftEyeBoneEntity>,
    Option<&'static RightEyeBoneEntity>,
    Option<&'static ExpressionEntityMap>,
    Option<&'static LookAt>,
    Option<&'static BodyTracking>,
);

/// Resource populated once a VRM has been inspected.
#[derive(Resource, Debug, Clone, Default, PartialEq)]
pub struct VrmCompatibilityReport {
    /// Whether a `Vrm` component was observed.
    pub vrm_loaded: bool,
    /// Whether the `Initialized` marker was observed.
    pub initialized: bool,
    /// Whether the head bone entity was found.
    pub has_head: bool,
    /// Whether the neck bone entity was found.
    pub has_neck: bool,
    /// Whether the left eye bone entity was found.
    pub has_left_eye: bool,
    /// Whether the right eye bone entity was found.
    pub has_right_eye: bool,
    /// Expression preset names discovered on the model.
    pub expressions: Vec<String>,
    /// Whether a `LookAt` component is present on the root.
    pub has_look_at_component: bool,
    /// Whether a `BodyTracking` component is present on the root.
    pub has_body_tracking_component: bool,
    /// Number of `SpringRoot` components found (proxy for SpringBone presence).
    pub spring_root_count: usize,
}

impl VrmCompatibilityReport {
    /// Returns `true` if the model is usable for the VTuber MVP.
    ///
    /// MVP requires at least a head bone and either per-eye blink or a
    /// combined blink expression. It does **not** require `LookAt` or
    /// `BodyTracking`, which are intentionally avoided per ADR-002.
    #[must_use]
    pub fn is_mvp_capable(&self) -> bool {
        self.vrm_loaded && self.has_head && self.has_any_blink()
    }

    /// Returns `true` if either combined or per-eye blink is available.
    #[must_use]
    pub fn has_any_blink(&self) -> bool {
        self.has_expression("blink")
            || (self.has_expression("blinkLeft") && self.has_expression("blinkRight"))
    }

    /// Returns `true` if the named expression preset is available.
    #[must_use]
    pub fn has_expression(&self, name: &str) -> bool {
        self.expressions.iter().any(|e| e.eq_ignore_ascii_case(name))
    }
}

fn inspect_initialized_vrm(
    mut report: ResMut<VrmCompatibilityReport>,
    vrms: Query<InitializedVrmBones, (With<Vrm>, Added<Initialized>)>,
    spring_roots: Query<&SpringRoot>,
) {
    for (
        _entity,
        head,
        neck,
        left_eye,
        right_eye,
        expression_map,
        look_at,
        body_tracking,
    ) in vrms.iter()
    {
        report.vrm_loaded = true;
        report.initialized = true;
        report.has_head = head.is_some();
        report.has_neck = neck.is_some();
        report.has_left_eye = left_eye.is_some();
        report.has_right_eye = right_eye.is_some();
        report.has_look_at_component = look_at.is_some();
        report.has_body_tracking_component = body_tracking.is_some();
        report.spring_root_count = spring_roots.iter().count();

        if let Some(map) = expression_map {
            report.expressions = map
                .0
                .keys()
                .map(|expr| expr.to_string())
                .collect::<Vec<_>>();
            report.expressions.sort();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mvp_capable_requires_head_and_blink() {
        let mut report = VrmCompatibilityReport::default();
        assert!(!report.is_mvp_capable());

        report.vrm_loaded = true;
        report.has_head = true;
        assert!(!report.is_mvp_capable());

        report.expressions.push("blink".into());
        assert!(report.is_mvp_capable());
    }

    #[test]
    fn per_eye_blink_satisfies_blink() {
        let report = VrmCompatibilityReport {
            vrm_loaded: true,
            has_head: true,
            expressions: vec!["blinkLeft".into(), "blinkRight".into()],
            ..Default::default()
        };
        assert!(report.is_mvp_capable());
    }
}

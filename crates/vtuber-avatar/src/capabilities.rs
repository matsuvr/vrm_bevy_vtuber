//! VRM expression and avatar capability discovery.
//!
//! This module inspects the expression map that `bevy_vrm1` builds during VRM
//! initialization and classifies the available presets into the categories
//! used by the VTuber adapter. It does not touch morph targets directly and
//! does not apply expression weights.
//!
//! The public [`AvatarCapabilities`] snapshot is engine-neutral: it contains
//! no Bevy `Entity` IDs and no `bevy_vrm1` component types. It is built once
//! when humanoid bone binding completes and is stored in the lifecycle for
//! UI consumption.

use bevy::prelude::*;
use bevy_vrm1::prelude::ExpressionEntityMap;
use std::collections::BTreeSet;

const BLINK_LEFT: &str = "blinkLeft";
const BLINK_RIGHT: &str = "blinkRight";
const BLINK: &str = "blink";

const MOUTH_PRESETS: [&str; 5] = ["aa", "ih", "ou", "ee", "oh"];

/// How the model exposes eye-blink expressions.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum BlinkMode {
    /// No blink expression is available.
    #[default]
    None,
    /// Both `blinkLeft` and `blinkRight` are available.
    PerEye,
    /// A single combined `blink` expression is available.
    Combined,
}

/// How the model exposes mouth-shape expressions.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum MouthMode {
    /// No mouth expression is available.
    #[default]
    None,
    /// Only the `aa` mouth expression is available.
    AaOnly,
    /// All five vowel expressions (`aa`, `ih`, `ou`, `ee`, `oh`) are available.
    Full,
}

/// Which look-direction expressions the model exposes.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct LookDirectionSet {
    /// `lookLeft` is present.
    pub left: bool,
    /// `lookRight` is present.
    pub right: bool,
    /// `lookUp` is present.
    pub up: bool,
    /// `lookDown` is present.
    pub down: bool,
}

impl LookDirectionSet {
    /// Returns `true` if at least one look-direction expression is available.
    #[must_use]
    pub const fn any(&self) -> bool {
        self.left || self.right || self.up || self.down
    }
}

/// Which emotion expressions the model exposes.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct EmotionSet {
    /// `happy` is present.
    pub happy: bool,
    /// `angry` is present.
    pub angry: bool,
    /// `sad` is present.
    pub sad: bool,
    /// `relaxed` is present.
    pub relaxed: bool,
    /// `surprised` is present.
    pub surprised: bool,
}

impl EmotionSet {
    /// Returns `true` if at least one emotion expression is available.
    #[must_use]
    pub const fn any(&self) -> bool {
        self.happy || self.angry || self.sad || self.relaxed || self.surprised
    }
}

/// Engine-neutral presence of humanoid bones relevant to tracking output.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct BonePresence {
    /// `head` bone is present (always true for a bound avatar).
    pub head: bool,
    /// `neck` bone is present.
    pub neck: bool,
    /// `leftEye` bone is present.
    pub left_eye: bool,
    /// `rightEye` bone is present.
    pub right_eye: bool,
    /// `upperChest` bone is present.
    pub upper_chest: bool,
    /// `chest` bone is present.
    pub chest: bool,
    /// `spine` bone is present.
    pub spine: bool,
}

/// Engine-neutral gaze control strategy for a model.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum GazeMode {
    /// No gaze control is available.
    #[default]
    None,
    /// Look-direction expressions are available.
    Expression,
    /// Left and right eye bones are available for direct rotation.
    EyeBones,
    /// Both look-direction expressions and eye bones are available.
    ExpressionAndEyeBones,
}

/// Discovered expression capabilities for a single VRM model.
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct ExpressionCapabilities {
    /// Available blink mode, if any.
    pub blink: BlinkMode,
    /// Available mouth mode, if any.
    pub mouth: MouthMode,
    /// Available look-direction expressions.
    pub look: LookDirectionSet,
    /// Available emotion expressions.
    pub emotions: EmotionSet,
    /// Custom or unknown expression names that are not part of the MVP preset
    /// set. They are retained for diagnostics but are not auto-mapped.
    pub unknown: Vec<String>,
}

impl ExpressionCapabilities {
    /// Builds capabilities from the `bevy_vrm1` expression map.
    ///
    /// Passing `None` produces an empty capability set, which is the correct
    /// treatment for models that do not expose an expression map.
    #[must_use]
    pub fn from_map(map: Option<&ExpressionEntityMap>) -> Self {
        let names: BTreeSet<String> = map
            .map(|m| m.0.keys().map(|expr| expr.0.clone()).collect())
            .unwrap_or_default();
        Self::from_names(&names)
    }

    /// Builds capabilities from a set of expression names.
    ///
    /// The input order does not affect the result. Names are matched against
    /// the VRM 1.0 expression preset names exactly.
    #[must_use]
    pub fn from_names(names: &BTreeSet<String>) -> Self {
        let mut caps = Self::default();
        let mut unknown = Vec::new();

        for name in names {
            let known = classify_known(name, &mut caps);
            if !known {
                unknown.push(name.clone());
            }
        }

        // Blink priority: per-eye beats combined.
        if names.contains(BLINK_LEFT) && names.contains(BLINK_RIGHT) {
            caps.blink = BlinkMode::PerEye;
        } else if names.contains(BLINK) {
            caps.blink = BlinkMode::Combined;
        }

        // Mouth priority: full five-vowel set beats aa-only.
        if MOUTH_PRESETS.iter().all(|preset| names.contains(*preset)) {
            caps.mouth = MouthMode::Full;
        } else if names.contains(MOUTH_PRESETS[0]) {
            caps.mouth = MouthMode::AaOnly;
        }

        caps.unknown = unknown;
        caps
    }

    /// Returns `true` if any blink expression is available.
    #[must_use]
    pub const fn has_blink(&self) -> bool {
        !matches!(self.blink, BlinkMode::None)
    }

    /// Returns `true` if any mouth expression is available.
    #[must_use]
    pub const fn has_mouth(&self) -> bool {
        !matches!(self.mouth, MouthMode::None)
    }
}

/// Public, engine-neutral snapshot of avatar capabilities for UI consumption.
///
/// This struct contains no Bevy `Entity` IDs and no `bevy_vrm1` component
/// types. It is built once when binding completes and stored in the active
/// lifecycle. The UI can display it without querying Bevy internals.
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct AvatarCapabilities {
    /// Presence of tracked humanoid bones.
    pub bones: BonePresence,
    /// Available blink mode.
    pub blink: BlinkMode,
    /// Available mouth mode.
    pub mouth: MouthMode,
    /// Resolved gaze control strategy.
    pub gaze: GazeMode,
    /// Available look-direction expression candidates.
    pub look_directions: LookDirectionSet,
    /// Whether the model has SpringBone chains.
    pub spring_bone: bool,
    /// Custom or unknown expression names not part of the MVP preset set.
    pub unknown_expressions: Vec<String>,
}

impl AvatarCapabilities {
    /// Builds a capability snapshot from engine-neutral bone presence and
    /// already-classified expression capabilities.
    ///
    /// `has_spring_bone` is supplied by the caller because detecting it
    /// requires a scene traversal that is specific to the binding system.
    #[must_use]
    pub(crate) fn from_bones_and_expression_capabilities(
        bones: BonePresence,
        expressions: &ExpressionCapabilities,
        has_spring_bone: bool,
    ) -> Self {
        Self {
            bones,
            blink: expressions.blink,
            mouth: expressions.mouth,
            gaze: determine_gaze_mode(&expressions.look, &bones),
            look_directions: expressions.look,
            spring_bone: has_spring_bone,
            unknown_expressions: expressions.unknown.clone(),
        }
    }

    /// Returns `true` if any unknown or unsupported expression preset was found.
    #[must_use]
    pub fn has_unknown(&self) -> bool {
        !self.unknown_expressions.is_empty()
    }

    /// Returns `true` if the model supports the full MVP tracking surface:
    /// head, blink, mouth `aa`, and at least one gaze method.
    #[must_use]
    pub fn is_fully_supported(&self) -> bool {
        self.bones.head
            && self.has_blink()
            && self.mouth != MouthMode::None
            && self.gaze != GazeMode::None
    }

    /// Human-readable summary for UI display.
    ///
    /// This is derived from the machine-readable fields and does not require
    /// a Bevy query.
    #[must_use]
    pub fn summary(&self) -> String {
        let mut parts = Vec::with_capacity(6);

        parts.push(format!("Bones: {}", self.bone_summary()));

        parts.push(format!(
            "Blink: {}",
            match self.blink {
                BlinkMode::None => "none",
                BlinkMode::PerEye => "per-eye",
                BlinkMode::Combined => "combined",
            }
        ));

        parts.push(format!(
            "Mouth: {}",
            match self.mouth {
                MouthMode::None => "none",
                MouthMode::AaOnly => "aa only",
                MouthMode::Full => "full vowels",
            }
        ));

        parts.push(format!(
            "Gaze: {}",
            match self.gaze {
                GazeMode::None => "none",
                GazeMode::Expression => "expression",
                GazeMode::EyeBones => "eye bones",
                GazeMode::ExpressionAndEyeBones => "expression + eye bones",
            }
        ));

        parts.push(format!(
            "SpringBone: {}",
            if self.spring_bone { "yes" } else { "no" }
        ));

        if self.has_unknown() {
            parts.push(format!(
                "{} unknown expression(s)",
                self.unknown_expressions.len()
            ));
        }

        parts.join("; ")
    }

    fn bone_summary(&self) -> String {
        let mut bones = Vec::new();
        if self.bones.head {
            bones.push("head");
        }
        if self.bones.neck {
            bones.push("neck");
        }
        if self.bones.left_eye && self.bones.right_eye {
            bones.push("eyes");
        } else {
            if self.bones.left_eye {
                bones.push("left eye");
            }
            if self.bones.right_eye {
                bones.push("right eye");
            }
        }
        if self.bones.upper_chest {
            bones.push("upper chest");
        }
        if self.bones.chest {
            bones.push("chest");
        }
        if self.bones.spine {
            bones.push("spine");
        }

        if bones.is_empty() {
            "none".to_string()
        } else {
            bones.join(", ")
        }
    }

    /// Returns `true` if any blink expression is available.
    #[must_use]
    pub const fn has_blink(&self) -> bool {
        !matches!(self.blink, BlinkMode::None)
    }
}

fn determine_gaze_mode(look: &LookDirectionSet, bones: &BonePresence) -> GazeMode {
    let has_expression = look.any();
    let has_eyes = bones.left_eye && bones.right_eye;
    match (has_expression, has_eyes) {
        (true, true) => GazeMode::ExpressionAndEyeBones,
        (true, false) => GazeMode::Expression,
        (false, true) => GazeMode::EyeBones,
        (false, false) => GazeMode::None,
    }
}

fn classify_known(name: &str, caps: &mut ExpressionCapabilities) -> bool {
    match name {
        BLINK_LEFT | BLINK_RIGHT | BLINK => true,
        "lookLeft" => {
            caps.look.left = true;
            true
        }
        "lookRight" => {
            caps.look.right = true;
            true
        }
        "lookUp" => {
            caps.look.up = true;
            true
        }
        "lookDown" => {
            caps.look.down = true;
            true
        }
        "happy" => {
            caps.emotions.happy = true;
            true
        }
        "angry" => {
            caps.emotions.angry = true;
            true
        }
        "sad" => {
            caps.emotions.sad = true;
            true
        }
        "relaxed" => {
            caps.emotions.relaxed = true;
            true
        }
        "surprised" => {
            caps.emotions.surprised = true;
            true
        }
        _ => MOUTH_PRESETS.contains(&name),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy_vrm1::prelude::VrmExpression;

    fn make_map(names: &[&str]) -> ExpressionEntityMap {
        let mut world = World::new();
        let mut map = bevy::platform::collections::HashMap::default();
        for name in names {
            let entity = world.spawn_empty().id();
            map.insert(VrmExpression::from(*name), entity);
        }
        ExpressionEntityMap(map)
    }

    #[test]
    fn expression_capabilities_empty_when_map_missing() {
        let caps = ExpressionCapabilities::from_map(None);
        assert_eq!(caps.blink, BlinkMode::None);
        assert_eq!(caps.mouth, MouthMode::None);
        assert!(!caps.look.any());
        assert!(!caps.emotions.any());
        assert!(caps.unknown.is_empty());
    }

    #[test]
    fn expression_capabilities_empty_when_no_expressions() {
        let map = make_map(&[]);
        let caps = ExpressionCapabilities::from_map(Some(&map));
        assert_eq!(caps.blink, BlinkMode::None);
        assert_eq!(caps.mouth, MouthMode::None);
        assert!(caps.unknown.is_empty());
    }

    #[test]
    fn expression_capabilities_per_eye_blink() {
        let map = make_map(&["blinkLeft", "blinkRight"]);
        let caps = ExpressionCapabilities::from_map(Some(&map));
        assert_eq!(caps.blink, BlinkMode::PerEye);
        assert!(caps.has_blink());
    }

    #[test]
    fn expression_capabilities_combined_blink() {
        let map = make_map(&["blink"]);
        let caps = ExpressionCapabilities::from_map(Some(&map));
        assert_eq!(caps.blink, BlinkMode::Combined);
    }

    #[test]
    fn expression_capabilities_per_eye_beats_combined_blink() {
        let map = make_map(&["blink", "blinkLeft", "blinkRight"]);
        let caps = ExpressionCapabilities::from_map(Some(&map));
        assert_eq!(caps.blink, BlinkMode::PerEye);
    }

    #[test]
    fn expression_capabilities_full_mouth() {
        let map = make_map(&["aa", "ih", "ou", "ee", "oh"]);
        let caps = ExpressionCapabilities::from_map(Some(&map));
        assert_eq!(caps.mouth, MouthMode::Full);
    }

    #[test]
    fn expression_capabilities_aa_only_mouth() {
        let map = make_map(&["aa"]);
        let caps = ExpressionCapabilities::from_map(Some(&map));
        assert_eq!(caps.mouth, MouthMode::AaOnly);
    }

    #[test]
    fn expression_capabilities_order_independent() {
        let a = make_map(&["oh", "aa", "ee", "ih", "ou"]);
        let b = make_map(&["aa", "ih", "ou", "ee", "oh"]);
        assert_eq!(
            ExpressionCapabilities::from_map(Some(&a)),
            ExpressionCapabilities::from_map(Some(&b))
        );
    }

    #[test]
    fn expression_capabilities_custom_is_unknown() {
        let map = make_map(&["customExpression", "aa"]);
        let caps = ExpressionCapabilities::from_map(Some(&map));
        assert_eq!(caps.mouth, MouthMode::AaOnly);
        assert_eq!(caps.unknown, vec!["customExpression"]);
    }

    #[test]
    fn avatar_capability_snapshot_minimal() {
        let expressions = ExpressionCapabilities::default();
        let caps = AvatarCapabilities::from_bones_and_expression_capabilities(
            BonePresence {
                head: true,
                ..BonePresence::default()
            },
            &expressions,
            false,
        );

        assert!(caps.bones.head);
        assert!(!caps.bones.neck);
        assert!(!caps.bones.left_eye);
        assert!(!caps.bones.right_eye);
        assert_eq!(caps.blink, BlinkMode::None);
        assert_eq!(caps.mouth, MouthMode::None);
        assert_eq!(caps.gaze, GazeMode::None);
        assert!(!caps.spring_bone);
        assert!(caps.unknown_expressions.is_empty());
        assert!(!caps.is_fully_supported());
    }

    #[test]
    fn avatar_capability_snapshot_full() {
        let map = make_map(&[
            "blinkLeft",
            "blinkRight",
            "aa",
            "ih",
            "ou",
            "ee",
            "oh",
            "lookLeft",
            "lookRight",
            "lookUp",
            "lookDown",
        ]);
        let expressions = ExpressionCapabilities::from_map(Some(&map));
        let bones = BonePresence {
            head: true,
            neck: true,
            left_eye: true,
            right_eye: true,
            upper_chest: true,
            chest: false,
            spine: false,
        };
        let caps =
            AvatarCapabilities::from_bones_and_expression_capabilities(bones, &expressions, true);

        assert!(caps.bones.head);
        assert!(caps.bones.neck);
        assert!(caps.bones.left_eye);
        assert!(caps.bones.right_eye);
        assert!(!caps.bones.chest);
        assert_eq!(caps.blink, BlinkMode::PerEye);
        assert_eq!(caps.mouth, MouthMode::Full);
        assert_eq!(caps.gaze, GazeMode::ExpressionAndEyeBones);
        assert!(caps.look_directions.left);
        assert!(caps.spring_bone);
        assert!(caps.is_fully_supported());
    }

    #[test]
    fn avatar_capability_snapshot_unknown() {
        let map = make_map(&["customA", "aa", "customB"]);
        let expressions = ExpressionCapabilities::from_map(Some(&map));
        let bones = BonePresence {
            head: true,
            ..BonePresence::default()
        };
        let caps =
            AvatarCapabilities::from_bones_and_expression_capabilities(bones, &expressions, false);

        assert_eq!(caps.mouth, MouthMode::AaOnly);
        assert!(caps.has_unknown());
        assert_eq!(caps.unknown_expressions, vec!["customA", "customB"]);
    }

    #[test]
    fn avatar_capability_snapshot_summary() {
        let map = make_map(&["blinkLeft", "blinkRight", "aa", "lookLeft"]);
        let expressions = ExpressionCapabilities::from_map(Some(&map));
        let bones = BonePresence {
            head: true,
            neck: true,
            left_eye: true,
            right_eye: true,
            upper_chest: false,
            chest: false,
            spine: false,
        };
        let caps =
            AvatarCapabilities::from_bones_and_expression_capabilities(bones, &expressions, true);

        let summary = caps.summary();
        assert!(summary.contains("Bones: head, neck, eyes"));
        assert!(summary.contains("Blink: per-eye"));
        assert!(summary.contains("Mouth: aa only"));
        assert!(summary.contains("Gaze: expression"));
        assert!(summary.contains("SpringBone: yes"));
    }

    #[test]
    fn avatar_capability_snapshot_gaze_modes() {
        let no_look = ExpressionCapabilities::default();
        assert_eq!(
            AvatarCapabilities::from_bones_and_expression_capabilities(
                BonePresence {
                    head: true,
                    ..BonePresence::default()
                },
                &no_look,
                false
            )
            .gaze,
            GazeMode::None
        );

        let look = LookDirectionSet {
            left: true,
            ..LookDirectionSet::default()
        };
        let expr_caps = ExpressionCapabilities {
            look,
            ..ExpressionCapabilities::default()
        };
        assert_eq!(
            AvatarCapabilities::from_bones_and_expression_capabilities(
                BonePresence {
                    head: true,
                    ..BonePresence::default()
                },
                &expr_caps,
                false
            )
            .gaze,
            GazeMode::Expression
        );

        let bones = BonePresence {
            head: true,
            left_eye: true,
            right_eye: true,
            ..BonePresence::default()
        };
        assert_eq!(
            AvatarCapabilities::from_bones_and_expression_capabilities(bones, &expr_caps, false)
                .gaze,
            GazeMode::ExpressionAndEyeBones
        );

        let no_look = ExpressionCapabilities::default();
        assert_eq!(
            AvatarCapabilities::from_bones_and_expression_capabilities(bones, &no_look, false).gaze,
            GazeMode::EyeBones
        );
    }
}

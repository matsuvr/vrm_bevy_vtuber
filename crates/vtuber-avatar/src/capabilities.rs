//! VRM expression capability discovery.
//!
//! This module inspects the expression map that `bevy_vrm1` builds during VRM
//! initialization and classifies the available presets into the categories
//! used by the VTuber adapter. It does not touch morph targets directly and
//! does not apply expression weights.

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
}

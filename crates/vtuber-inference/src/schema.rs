//! Landmark schema definitions and index mapping for the PeppaPig-98 model.

use vtuber_core::types::LandmarkSchemaId;

/// Schema ID for the Peppa_Pig_Face_Landmark student 256x256 model.
pub const SCHEMA_PEPPAPIG_98: LandmarkSchemaId = LandmarkSchemaId("peppapig-98");

/// Indices for the 98-point landmark set used for basic expressions.
pub mod indices {
    /// Indices for the left eye.
    pub const LEFT_EYE: &[usize] = &[
        // Approximate indices for a 98-point set; these are placeholders
        // until validated against the actual model output.
        33, 160, 158, 133, 153, 144,
    ];
    /// Indices for the right eye.
    pub const RIGHT_EYE: &[usize] = &[263, 388, 385, 362, 382, 373];
    /// Indices for the mouth (inner/outer).
    pub const MOUTH: &[usize] = &[0, 17, 37, 267, 291];
}

/// Basic blink/mouth observation derived from landmarks.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct BasicObservation {
    /// Left eye closed coefficient in `[0, 1]`.
    pub blink_left: f32,
    /// Right eye closed coefficient in `[0, 1]`.
    pub blink_right: f32,
    /// Mouth open coefficient in `[0, 1]`.
    pub mouth_open: f32,
}

impl BasicObservation {
    /// Calculates basic coefficients from raw landmarks using a simple distance heuristic.
    pub fn from_landmarks(landmarks: &[vtuber_core::types::Landmark3]) -> Self {
        // For M1-02, we use a simple ratio: dist(vertical) / dist(horizontal)
        // compared to a presumed neutral. Since we don't have calibration yet,
        // we use a hardcoded threshold for this prototype.

        let blink_left = calculate_blink(landmarks, indices::LEFT_EYE);
        let blink_right = calculate_blink(landmarks, indices::RIGHT_EYE);
        let mouth_open = calculate_mouth(landmarks, indices::MOUTH);

        Self {
            blink_left,
            blink_right,
            mouth_open,
        }
    }
}

fn calculate_blink(landmarks: &[vtuber_core::types::Landmark3], idx: &[usize]) -> f32 {
    if landmarks.len() < idx.iter().max().unwrap_or(&0) + 1 {
        return 0.0;
    }
    // Simple vertical distance heuristic: mid-top vs mid-bottom
    // This is a placeholder for real blink detection.
    let top = landmarks[idx[0]];
    let bot = landmarks[idx[1]];
    let dist = (top.y - bot.y).abs();
    (1.0 - dist * 10.0).clamp(0.0, 1.0)
}

fn calculate_mouth(landmarks: &[vtuber_core::types::Landmark3], idx: &[usize]) -> f32 {
    if landmarks.len() < idx.iter().max().unwrap_or(&0) + 1 {
        return 0.0;
    }
    let top = landmarks[idx[0]];
    let bot = landmarks[idx[1]];
    let dist = (top.y - bot.y).abs();
    (dist * 5.0).clamp(0.0, 1.0)
}

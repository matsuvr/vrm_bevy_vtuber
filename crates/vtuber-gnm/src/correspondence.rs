//! MediaPipe-to-GNM sparse correspondence and weak-perspective fitting.

use crate::{GnmModelError, GnmSparseVertices};

/// The number of points in the repository-owned iBUG/FAN-like semantic order.
pub const SPARSE_FACE_LANDMARK_COUNT: usize = 68;
/// The number of normalized MediaPipe Face Landmarker points.
pub const MEDIAPIPE_LANDMARK_COUNT: usize = 478;

/// A stable semantic point in the iBUG/FAN-like 68-point order.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct SparseFaceLandmarkSemantic {
    index: u8,
}

impl SparseFaceLandmarkSemantic {
    /// Creates a semantic point by its canonical zero-based index.
    pub const fn new(index: usize) -> Option<Self> {
        if index < SPARSE_FACE_LANDMARK_COUNT {
            Some(Self { index: index as u8 })
        } else {
            None
        }
    }

    /// Returns the canonical zero-based index.
    pub const fn index(self) -> usize {
        self.index as usize
    }

    /// Returns the stable semantic label.
    pub const fn name(self) -> &'static str {
        SEMANTIC_NAMES[self.index as usize]
    }
}

const SEMANTIC_NAMES: [&str; SPARSE_FACE_LANDMARK_COUNT] = [
    "Jaw0",
    "Jaw1",
    "Jaw2",
    "Jaw3",
    "Jaw4",
    "Jaw5",
    "Jaw6",
    "Jaw7",
    "Jaw8",
    "Jaw9",
    "Jaw10",
    "Jaw11",
    "Jaw12",
    "Jaw13",
    "Jaw14",
    "Jaw15",
    "Jaw16",
    "BrowRight0",
    "BrowRight1",
    "BrowRight2",
    "BrowRight3",
    "BrowRight4",
    "BrowLeft0",
    "BrowLeft1",
    "BrowLeft2",
    "BrowLeft3",
    "BrowLeft4",
    "NoseBridge0",
    "NoseBridge1",
    "NoseBridge2",
    "NoseBridge3",
    "NoseBottom0",
    "NoseBottom1",
    "NoseBottom2",
    "NoseBottom3",
    "NoseBottom4",
    "EyeRight0",
    "EyeRight1",
    "EyeRight2",
    "EyeRight3",
    "EyeRight4",
    "EyeRight5",
    "EyeLeft0",
    "EyeLeft1",
    "EyeLeft2",
    "EyeLeft3",
    "EyeLeft4",
    "EyeLeft5",
    "MouthOuter0",
    "MouthOuter1",
    "MouthOuter2",
    "MouthOuter3",
    "MouthOuter4",
    "MouthOuter5",
    "MouthOuter6",
    "MouthOuter7",
    "MouthOuter8",
    "MouthOuter9",
    "MouthOuter10",
    "MouthOuter11",
    "MouthInner0",
    "MouthInner1",
    "MouthInner2",
    "MouthInner3",
    "MouthInner4",
    "MouthInner5",
    "MouthInner6",
    "MouthInner7",
];

/// One explicit MediaPipe-to-GNM sparse correspondence row.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MediaPipeToGnmSparseMap {
    /// Stable iBUG/FAN-like semantic point.
    pub semantic: SparseFaceLandmarkSemantic,
    /// MediaPipe Face Landmarker index.
    pub mediapipe_index: usize,
    /// Index in the official GNM sparse landmark output.
    pub gnm_sparse_index: usize,
    /// Static objective weight.
    pub weight: f32,
}

const MEDIAPIPE_INDICES: [usize; SPARSE_FACE_LANDMARK_COUNT] = [
    234, 93, 132, 58, 172, 136, 150, 149, 152, 377, 400, 378, 379, 365, 397, 288, 361, 70, 63, 105,
    66, 107, 336, 296, 334, 293, 300, 168, 6, 197, 195, 48, 4, 278, 275, 45, 33, 160, 158, 133,
    153, 144, 362, 385, 387, 263, 373, 380, 61, 40, 37, 0, 267, 270, 291, 321, 314, 17, 84, 91, 78,
    81, 13, 311, 308, 14, 178, 95,
];

const fn default_map() -> [MediaPipeToGnmSparseMap; SPARSE_FACE_LANDMARK_COUNT] {
    let mut result = [MediaPipeToGnmSparseMap {
        semantic: SparseFaceLandmarkSemantic { index: 0 },
        mediapipe_index: 0,
        gnm_sparse_index: 0,
        weight: 1.0,
    }; SPARSE_FACE_LANDMARK_COUNT];
    let mut index = 0;
    while index < SPARSE_FACE_LANDMARK_COUNT {
        result[index] = MediaPipeToGnmSparseMap {
            semantic: SparseFaceLandmarkSemantic { index: index as u8 },
            mediapipe_index: MEDIAPIPE_INDICES[index],
            gnm_sparse_index: index,
            weight: 1.0,
        };
        index += 1;
    }
    result
}

/// Repository-owned first-cut mapping. It is not claimed to be an official
/// Google GNM correspondence; the semantic and source conventions are fixed
/// here so a later provenance-backed replacement is reviewable in one place.
pub static DEFAULT_MEDIAPIPE_TO_GNM_MAP: [MediaPipeToGnmSparseMap; SPARSE_FACE_LANDMARK_COUNT] =
    default_map();

/// One normalized MediaPipe observation after correspondence selection.
#[derive(Clone, Debug, PartialEq)]
pub struct GnmSparseObservation {
    normalized_xy: Vec<[f32; 2]>,
    weights: Vec<f32>,
}

impl GnmSparseObservation {
    /// Validates a mapped observation and its static/objective weights.
    pub fn new(normalized_xy: Vec<[f32; 2]>, weights: Vec<f32>) -> Result<Self, GnmFitError> {
        if normalized_xy.len() != weights.len() {
            return Err(GnmFitError::Shape {
                field: "normalized_xy/weights",
                expected: weights.len(),
                actual: normalized_xy.len(),
            });
        }
        if normalized_xy.is_empty() {
            return Err(GnmFitError::InsufficientPoints { valid: 0 });
        }
        for (index, (point, weight)) in normalized_xy.iter().zip(&weights).enumerate() {
            if point.iter().any(|value| !value.is_finite()) || !weight.is_finite() {
                return Err(GnmFitError::NonFinite { index });
            }
            if point.iter().any(|value| !(0.0..=1.0).contains(value)) {
                return Err(GnmFitError::OutOfRange { index });
            }
            if *weight < 0.0 {
                return Err(GnmFitError::InvalidWeight { index });
            }
        }
        Ok(Self {
            normalized_xy,
            weights,
        })
    }

    /// Returns normalized image points in correspondence order.
    pub fn normalized_xy(&self) -> &[[f32; 2]] {
        &self.normalized_xy
    }

    /// Returns objective weights in correspondence order.
    pub fn weights(&self) -> &[f32] {
        &self.weights
    }

    /// Builds an observation from all 478 normalized MediaPipe `(x, y)` points.
    pub fn from_mediapipe(
        landmarks: &[[f32; 2]],
        map: &[MediaPipeToGnmSparseMap],
    ) -> Result<Self, GnmFitError> {
        if landmarks.len() != MEDIAPIPE_LANDMARK_COUNT {
            return Err(GnmFitError::Shape {
                field: "mediapipe landmarks",
                expected: MEDIAPIPE_LANDMARK_COUNT,
                actual: landmarks.len(),
            });
        }
        validate_map(map)?;
        let mut points = Vec::with_capacity(map.len());
        let mut weights = Vec::with_capacity(map.len());
        for row in map {
            points.push(landmarks[row.mediapipe_index]);
            weights.push(row.weight);
        }
        Self::new(points, weights)
    }
}

/// Weak-perspective camera and rigid pose parameters.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GnmProjectionModel {
    /// Rotation around the image vertical axis, in radians.
    pub yaw: f32,
    /// Rotation around the image horizontal axis, in radians.
    pub pitch: f32,
    /// Clockwise image-plane rotation, in radians.
    pub roll: f32,
    /// Positive normalized-image scale.
    pub scale: f32,
    /// Normalized image translation.
    pub translation: [f32; 2],
}

impl Default for GnmProjectionModel {
    fn default() -> Self {
        Self {
            yaw: 0.0,
            pitch: 0.0,
            roll: 0.0,
            scale: 1.0,
            translation: [0.5, 0.5],
        }
    }
}

/// Quantified weak-perspective fit result.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GnmProjectionFit {
    /// Recovered camera and rigid pose.
    pub model: GnmProjectionModel,
    /// Weighted RMS residual in normalized image coordinates.
    pub residual_rms: f32,
    /// Number of positive-weight points used.
    pub valid_point_count: usize,
    /// Number of Gauss-Newton iterations performed.
    pub iterations: usize,
}

/// Typed failure from correspondence validation or projection fitting.
#[derive(Clone, Debug, PartialEq)]
pub enum GnmFitError {
    /// A vector pair had inconsistent lengths.
    Shape {
        /// Field name.
        field: &'static str,
        /// Expected length.
        expected: usize,
        /// Actual length.
        actual: usize,
    },
    /// A map row or observation contained a non-finite value.
    NonFinite {
        /// Point index containing a non-finite value.
        index: usize,
    },
    /// A normalized point was outside the closed unit square.
    OutOfRange {
        /// Point index outside the normalized image square.
        index: usize,
    },
    /// A weight was negative.
    InvalidWeight {
        /// Point index containing the invalid weight.
        index: usize,
    },
    /// A map was not one-to-one or referenced an invalid index.
    InvalidMap {
        /// Map row index.
        index: usize,
        /// Stable reason code text.
        reason: &'static str,
    },
    /// Too few positive-weight points were available.
    InsufficientPoints {
        /// Number of positive-weight points.
        valid: usize,
    },
    /// The normal equation was singular.
    Singular {
        /// Iteration at which the normal equation was singular.
        iteration: usize,
    },
    /// The fit became non-finite.
    NonFiniteFit {
        /// Iteration at which the fit became non-finite.
        iteration: usize,
    },
    /// The supplied GNM sparse output was too short for the map.
    SparseOutput {
        /// Missing sparse output index.
        index: usize,
    },
    /// A GNM evaluator returned an error.
    Model(GnmModelError),
}

impl std::fmt::Display for GnmFitError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Shape {
                field,
                expected,
                actual,
            } => {
                write!(
                    formatter,
                    "{field} length mismatch: expected {expected}, got {actual}"
                )
            }
            Self::NonFinite { index } => {
                write!(formatter, "non-finite correspondence value at {index}")
            }
            Self::OutOfRange { index } => {
                write!(formatter, "normalized point {index} is outside [0, 1]")
            }
            Self::InvalidWeight { index } => {
                write!(formatter, "negative correspondence weight at {index}")
            }
            Self::InvalidMap { index, reason } => {
                write!(formatter, "invalid correspondence row {index}: {reason}")
            }
            Self::InsufficientPoints { valid } => {
                write!(formatter, "insufficient positive-weight points: {valid}")
            }
            Self::Singular { iteration } => write!(
                formatter,
                "singular projection fit at iteration {iteration}"
            ),
            Self::NonFiniteFit { iteration } => write!(
                formatter,
                "non-finite projection fit at iteration {iteration}"
            ),
            Self::SparseOutput { index } => {
                write!(formatter, "sparse output is missing point {index}")
            }
            Self::Model(error) => error.fmt(formatter),
        }
    }
}

impl std::error::Error for GnmFitError {}

/// Validates the explicit correspondence table without applying it.
pub fn validate_map(map: &[MediaPipeToGnmSparseMap]) -> Result<(), GnmFitError> {
    if map.len() != SPARSE_FACE_LANDMARK_COUNT {
        return Err(GnmFitError::Shape {
            field: "correspondence map",
            expected: SPARSE_FACE_LANDMARK_COUNT,
            actual: map.len(),
        });
    }
    let mut media_seen = [false; MEDIAPIPE_LANDMARK_COUNT];
    let mut gnm_seen = [false; SPARSE_FACE_LANDMARK_COUNT];
    for (index, row) in map.iter().enumerate() {
        if row.mediapipe_index >= MEDIAPIPE_LANDMARK_COUNT {
            return Err(GnmFitError::InvalidMap {
                index,
                reason: "MediaPipe index out of range",
            });
        }
        if row.gnm_sparse_index >= SPARSE_FACE_LANDMARK_COUNT {
            return Err(GnmFitError::InvalidMap {
                index,
                reason: "GNM sparse index out of range",
            });
        }
        if row.semantic.index() != index {
            return Err(GnmFitError::InvalidMap {
                index,
                reason: "semantic order is not canonical",
            });
        }
        if row.weight < 0.0 || !row.weight.is_finite() {
            return Err(GnmFitError::InvalidMap {
                index,
                reason: "weight must be finite and non-negative",
            });
        }
        if media_seen[row.mediapipe_index] || gnm_seen[row.gnm_sparse_index] {
            return Err(GnmFitError::InvalidMap {
                index,
                reason: "duplicate source or sparse target",
            });
        }
        media_seen[row.mediapipe_index] = true;
        gnm_seen[row.gnm_sparse_index] = true;
    }
    Ok(())
}

/// Fits a scaled-orthographic camera and rigid pose to GNM sparse points.
pub fn fit_weak_perspective(
    sparse_vertices: &GnmSparseVertices,
    map: &[MediaPipeToGnmSparseMap],
    observation: &GnmSparseObservation,
    initial: Option<GnmProjectionModel>,
) -> Result<GnmProjectionFit, GnmFitError> {
    validate_map(map)?;
    if observation.normalized_xy.len() != map.len() {
        return Err(GnmFitError::Shape {
            field: "observation/map",
            expected: map.len(),
            actual: observation.normalized_xy.len(),
        });
    }
    let valid = observation
        .weights
        .iter()
        .filter(|weight| **weight > 0.0)
        .count();
    if valid < 6 {
        return Err(GnmFitError::InsufficientPoints { valid });
    }
    let mut model = initial.unwrap_or_default();
    if !model_is_finite(model) || model.scale <= 0.0 {
        return Err(GnmFitError::NonFiniteFit { iteration: 0 });
    }
    let mut iterations = 0;
    for iteration in 0..32 {
        iterations = iteration + 1;
        let mut normal = [[0.0f64; 6]; 6];
        let mut rhs = [0.0f64; 6];
        for (map_index, row) in map.iter().enumerate() {
            let weight = observation.weights[map_index];
            if weight <= 0.0 {
                continue;
            }
            let vertex = sparse_vertices.values().get(row.gnm_sparse_index).ok_or(
                GnmFitError::SparseOutput {
                    index: row.gnm_sparse_index,
                },
            )?;
            let predicted = project_weak_perspective(*vertex, model);
            let residual = [
                observation.normalized_xy[map_index][0] - predicted[0],
                observation.normalized_xy[map_index][1] - predicted[1],
            ];
            for (component, residual_component) in residual.iter().enumerate() {
                let jacobian = numerical_jacobian(*vertex, model, component);
                let weight_sqrt = (weight as f64).sqrt();
                let weighted_jacobian = jacobian.map(|value| weight_sqrt * value);
                let weighted_residual = weight_sqrt * *residual_component as f64;
                for left in 0..6 {
                    rhs[left] += weighted_jacobian[left] * weighted_residual;
                    for right in 0..6 {
                        normal[left][right] += weighted_jacobian[left] * weighted_jacobian[right];
                    }
                }
            }
        }
        let delta = solve_symmetric(normal, rhs).ok_or(GnmFitError::Singular { iteration })?;
        let next = apply_delta(model, delta);
        if !model_is_finite(next) || next.scale <= 0.0 {
            return Err(GnmFitError::NonFiniteFit { iteration });
        }
        model = next;
        if delta.iter().map(|value| value * value).sum::<f64>().sqrt() < 1e-7 {
            break;
        }
    }
    let mut weighted_error = 0.0f64;
    let mut weight_sum = 0.0f64;
    for (map_index, row) in map.iter().enumerate() {
        let weight = observation.weights[map_index];
        if weight <= 0.0 {
            continue;
        }
        let vertex = sparse_vertices.values().get(row.gnm_sparse_index).ok_or(
            GnmFitError::SparseOutput {
                index: row.gnm_sparse_index,
            },
        )?;
        let predicted = project_weak_perspective(*vertex, model);
        let dx = predicted[0] - observation.normalized_xy[map_index][0];
        let dy = predicted[1] - observation.normalized_xy[map_index][1];
        weighted_error += weight as f64 * (dx as f64 * dx as f64 + dy as f64 * dy as f64);
        weight_sum += weight as f64;
    }
    Ok(GnmProjectionFit {
        model,
        residual_rms: (weighted_error / weight_sum).sqrt() as f32,
        valid_point_count: valid,
        iterations,
    })
}

fn model_is_finite(model: GnmProjectionModel) -> bool {
    [
        model.yaw,
        model.pitch,
        model.roll,
        model.scale,
        model.translation[0],
        model.translation[1],
    ]
    .iter()
    .all(|value| value.is_finite())
}

/// Projects one GNM point with the fitted weak-perspective model.
///
/// This is public so later GNM fitting layers can build a linearized sparse
/// objective without duplicating the coordinate, handedness, or image-Y
/// convention owned by this module.
#[must_use]
pub fn project_weak_perspective(vertex: [f32; 3], model: GnmProjectionModel) -> [f32; 2] {
    let rotated = matrix_vector(rotation_matrix(model), vertex);
    [
        model.scale * rotated[0] + model.translation[0],
        model.scale * rotated[1] + model.translation[1],
    ]
}

fn numerical_jacobian(vertex: [f32; 3], model: GnmProjectionModel, component: usize) -> [f64; 6] {
    let mut result = [0.0; 6];
    for (parameter, result_value) in result.iter_mut().enumerate() {
        let step = if parameter == 3 { 1e-5 } else { 1e-4 };
        let mut plus = model;
        let mut minus = model;
        set_parameter(&mut plus, parameter, step);
        set_parameter(&mut minus, parameter, -step);
        *result_value = ((project_weak_perspective(vertex, plus)[component]
            - project_weak_perspective(vertex, minus)[component])
            / (2.0 * step)) as f64;
    }
    result
}

fn apply_delta(mut model: GnmProjectionModel, delta: [f64; 6]) -> GnmProjectionModel {
    for (parameter, value) in delta.iter().enumerate() {
        set_parameter(&mut model, parameter, *value as f32);
    }
    model
}

fn set_parameter(model: &mut GnmProjectionModel, parameter: usize, value: f32) {
    match parameter {
        0 => model.yaw += value,
        1 => model.pitch += value,
        2 => model.roll += value,
        3 => model.scale *= value.exp(),
        4 => model.translation[0] += value,
        5 => model.translation[1] += value,
        _ => {}
    }
}

fn rotation_matrix(model: GnmProjectionModel) -> [[f32; 3]; 3] {
    let (sy, cy) = model.yaw.sin_cos();
    let (sp, cp) = model.pitch.sin_cos();
    let (sr, cr) = model.roll.sin_cos();
    let yaw = [[cy, 0.0, sy], [0.0, 1.0, 0.0], [-sy, 0.0, cy]];
    let pitch = [[1.0, 0.0, 0.0], [0.0, cp, -sp], [0.0, sp, cp]];
    let roll = [[cr, -sr, 0.0], [sr, cr, 0.0], [0.0, 0.0, 1.0]];
    matrix_matrix(matrix_matrix(roll, yaw), pitch)
}

fn matrix_vector(matrix: [[f32; 3]; 3], vector: [f32; 3]) -> [f32; 3] {
    [
        matrix[0][0] * vector[0] + matrix[0][1] * vector[1] + matrix[0][2] * vector[2],
        matrix[1][0] * vector[0] + matrix[1][1] * vector[1] + matrix[1][2] * vector[2],
        matrix[2][0] * vector[0] + matrix[2][1] * vector[1] + matrix[2][2] * vector[2],
    ]
}

fn matrix_matrix(left: [[f32; 3]; 3], right: [[f32; 3]; 3]) -> [[f32; 3]; 3] {
    let mut result = [[0.0; 3]; 3];
    for row in 0..3 {
        for column in 0..3 {
            for index in 0..3 {
                result[row][column] += left[row][index] * right[index][column];
            }
        }
    }
    result
}

fn solve_symmetric(mut matrix: [[f64; 6]; 6], mut rhs: [f64; 6]) -> Option<[f64; 6]> {
    for pivot in 0..6 {
        let mut pivot_row = pivot;
        for row in pivot + 1..6 {
            if matrix[row][pivot].abs() > matrix[pivot_row][pivot].abs() {
                pivot_row = row;
            }
        }
        if matrix[pivot_row][pivot].abs() < 1e-10 {
            return None;
        }
        if pivot_row != pivot {
            matrix.swap(pivot, pivot_row);
            rhs.swap(pivot, pivot_row);
        }
        let diagonal = matrix[pivot][pivot];
        for value in matrix[pivot].iter_mut().skip(pivot) {
            *value /= diagonal;
        }
        rhs[pivot] /= diagonal;
        let normalized_pivot = matrix[pivot];
        for row in 0..6 {
            if row == pivot {
                continue;
            }
            let factor = matrix[row][pivot];
            for (column, value) in matrix[row].iter_mut().enumerate().skip(pivot) {
                *value -= factor * normalized_pivot[column];
            }
            rhs[row] -= factor * rhs[pivot];
        }
    }
    Some(rhs)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn synthetic_vertices() -> GnmSparseVertices {
        let mut output = GnmSparseVertices::with_len(SPARSE_FACE_LANDMARK_COUNT);
        for (index, point) in output.values_mut().iter_mut().enumerate() {
            let angle = index as f32 * 0.17;
            *point = [
                angle.cos() * 0.3,
                angle.sin() * 0.25,
                (index as f32 * 0.03).sin() * 0.1,
            ];
        }
        output
    }

    #[test]
    fn default_map_is_ordered_and_one_to_one() {
        validate_map(&DEFAULT_MEDIAPIPE_TO_GNM_MAP).unwrap();
        assert_eq!(DEFAULT_MEDIAPIPE_TO_GNM_MAP[0].semantic.name(), "Jaw0");
        assert_ne!(
            DEFAULT_MEDIAPIPE_TO_GNM_MAP[0].mediapipe_index,
            DEFAULT_MEDIAPIPE_TO_GNM_MAP[1].mediapipe_index
        );
    }

    #[test]
    fn duplicate_map_rows_are_rejected() {
        let mut map = DEFAULT_MEDIAPIPE_TO_GNM_MAP;
        map[1].mediapipe_index = map[0].mediapipe_index;
        assert!(matches!(
            validate_map(&map),
            Err(GnmFitError::InvalidMap { .. })
        ));
    }

    #[test]
    fn known_weak_perspective_pose_is_recovered() {
        let vertices = synthetic_vertices();
        let truth = GnmProjectionModel {
            yaw: 0.08,
            pitch: -0.05,
            roll: 0.04,
            scale: 0.85,
            translation: [0.48, 0.53],
        };
        let points = DEFAULT_MEDIAPIPE_TO_GNM_MAP
            .iter()
            .map(|row| project_weak_perspective(vertices.values()[row.gnm_sparse_index], truth))
            .collect();
        let observation = GnmSparseObservation::new(points, vec![1.0; 68]).unwrap();
        let fit =
            fit_weak_perspective(&vertices, &DEFAULT_MEDIAPIPE_TO_GNM_MAP, &observation, None)
                .unwrap();
        assert!(fit.residual_rms < 1e-4, "residual={}", fit.residual_rms);
        assert!((fit.model.yaw - truth.yaw).abs() < 2e-3);
        assert!((fit.model.pitch - truth.pitch).abs() < 2e-3);
        assert!((fit.model.roll - truth.roll).abs() < 2e-3);
        assert!((fit.model.scale - truth.scale).abs() < 2e-3);
        assert!((fit.model.translation[0] - truth.translation[0]).abs() < 2e-3);
        assert!((fit.model.translation[1] - truth.translation[1]).abs() < 2e-3);
    }

    #[test]
    fn nonfinite_and_low_confidence_points_are_typed() {
        assert!(matches!(
            GnmSparseObservation::new(vec![[f32::NAN, 0.2]], vec![1.0]),
            Err(GnmFitError::NonFinite { index: 0 })
        ));
        assert!(matches!(
            GnmSparseObservation::new(vec![[0.2, 0.2]], vec![-1.0]),
            Err(GnmFitError::InvalidWeight { index: 0 })
        ));
    }
}

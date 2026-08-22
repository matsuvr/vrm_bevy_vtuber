//! Engine-neutral dense face observation and GNM surface correspondence core.
//!
//! This module deliberately owns no camera, MediaPipe runtime, Bevy, or renderer
//! state. It turns normalized 2D observations into a validated correspondence
//! contract and reuses the existing selected-surface GNM evaluator.

use crate::{
    GnmExpressionState, GnmIdentityState, GnmJointState, GnmModel, GnmModelError,
    GnmSparseVertices, GnmVersion, SparseLandmark, SparseLandmarkSet,
};

/// Number of normalized landmarks emitted by MediaPipe Face Landmarker.
pub const MEDIAPIPE_FACE_LANDMARK_COUNT: usize = 478;
/// The existing sparse bootstrap contains 68 points, so a primary dense mapping
/// must contain strictly more than this many validated points.
pub const SPARSE_BOOTSTRAP_POINT_COUNT: usize = 68;

/// Anatomical side of a face observation.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AnatomicalSide {
    /// Subject's anatomical left side.
    Left,
    /// Subject's anatomical right side.
    Right,
    /// Midline point with no left/right ownership.
    Midline,
}

/// Coarse facial region used for weighting and diagnostics.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FaceRegion {
    /// Face silhouette or jaw contour.
    Contour,
    /// Eyebrow region.
    Brow,
    /// Eyelid or periocular region.
    Eye,
    /// Nose and central rigid-face region.
    Nose,
    /// Lips and mouth region.
    Mouth,
    /// Iris or eye-center region.
    Iris,
    /// Other explicitly validated face surface point.
    Other,
}

/// Provenance class for a repository-owned correspondence row.
///
/// None of these variants claim that Google publishes an official
/// MediaPipe-to-GNM correspondence table.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CorrespondenceProvenance {
    /// A row anchored from the existing 68-point sparse bootstrap.
    SparseBootstrap,
    /// A row derived and validated by this repository.
    RepositoryValidated,
    /// A research-derived row retained with lower evidentiary confidence.
    ResearchDerived,
}

/// Static reliability class for a correspondence row.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CorrespondenceReliability {
    /// Strong semantic correspondence suitable for a primary data term.
    High,
    /// Usable correspondence that may receive a lower region/static weight.
    Medium,
    /// Weak but explicitly retained correspondence.
    Low,
}

/// A point on the GNM template surface.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum GnmSurfacePointRef {
    /// Exact template vertex.
    Vertex {
        /// Template vertex index.
        vertex_index: usize,
    },
    /// Barycentric point over three explicit template vertices.
    ///
    /// Explicit vertex indices are used instead of a triangle index because the
    /// currently pinned GNM runtime boundary does not load render topology.
    Barycentric {
        /// Three template vertex indices defining the surface point.
        vertex_indices: [usize; 3],
        /// Barycentric weights corresponding to `vertex_indices`.
        weights: [f32; 3],
    },
}

impl GnmSurfacePointRef {
    fn to_sparse_landmark(self, vertex_count: usize) -> Result<SparseLandmark, GnmDenseError> {
        let (indices, weights) = match self {
            Self::Vertex { vertex_index } => {
                ([vertex_index, vertex_index, vertex_index], [1.0, 0.0, 0.0])
            }
            Self::Barycentric {
                vertex_indices,
                weights,
            } => (vertex_indices, weights),
        };
        if let Some(vertex_index) = indices.iter().copied().find(|index| *index >= vertex_count) {
            return Err(GnmDenseError::InvalidMapping {
                row: None,
                reason: format!(
                    "surface point references vertex {vertex_index}, but model has {vertex_count} vertices"
                ),
            });
        }
        SparseLandmark::new(indices, weights).map_err(GnmDenseError::Model)
    }
}

/// Version binding for a repository-owned dense mapping.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DenseMappingVersion {
    /// Repository mapping schema revision.
    pub schema_revision: u32,
    /// GNM model schema version for which the mapping was validated.
    pub model_version: GnmVersion,
}

/// One MediaPipe-to-GNM dense correspondence row.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MediaPipeGnmDenseCorrespondence {
    /// MediaPipe Face Landmarker source index.
    pub mediapipe_index: usize,
    /// GNM surface target.
    pub target: GnmSurfacePointRef,
    /// Coarse facial region.
    pub region: FaceRegion,
    /// Subject-relative anatomical side.
    pub anatomical_side: AnatomicalSide,
    /// Static objective weight. Dynamic confidence is intentionally separate.
    pub base_weight: f32,
    /// Evidence class for the mapping row.
    pub provenance: CorrespondenceProvenance,
    /// Static reliability class.
    pub reliability: CorrespondenceReliability,
}

/// Validated, immutable dense correspondence set.
#[derive(Clone, Debug, PartialEq)]
pub struct DenseCorrespondenceSet {
    version: DenseMappingVersion,
    rows: Vec<MediaPipeGnmDenseCorrespondence>,
    surface_landmarks: SparseLandmarkSet,
}

impl DenseCorrespondenceSet {
    /// Validates rows against the supplied GNM model and builds the reusable
    /// selected-surface evaluator contract.
    pub fn new(
        version: DenseMappingVersion,
        rows: Vec<MediaPipeGnmDenseCorrespondence>,
        model: &GnmModel,
    ) -> Result<Self, GnmDenseError> {
        if version.model_version != model.version() {
            return Err(GnmDenseError::ModelVersionMismatch {
                mapping: version.model_version,
                model: model.version(),
            });
        }
        if version.schema_revision == 0 {
            return Err(GnmDenseError::InvalidMapping {
                row: None,
                reason: "mapping schema revision must be non-zero".to_owned(),
            });
        }
        if rows.is_empty() {
            return Err(GnmDenseError::InvalidMapping {
                row: None,
                reason: "mapping must contain at least one row".to_owned(),
            });
        }

        let mut seen_source = [false; MEDIAPIPE_FACE_LANDMARK_COUNT];
        let mut landmarks = Vec::with_capacity(rows.len());
        for (row_index, row) in rows.iter().enumerate() {
            if row.mediapipe_index >= MEDIAPIPE_FACE_LANDMARK_COUNT {
                return Err(GnmDenseError::InvalidMapping {
                    row: Some(row_index),
                    reason: format!(
                        "MediaPipe index {} is outside 0..{}",
                        row.mediapipe_index, MEDIAPIPE_FACE_LANDMARK_COUNT
                    ),
                });
            }
            if seen_source[row.mediapipe_index] {
                return Err(GnmDenseError::InvalidMapping {
                    row: Some(row_index),
                    reason: format!("duplicate MediaPipe index {}", row.mediapipe_index),
                });
            }
            if !row.base_weight.is_finite() || row.base_weight <= 0.0 {
                return Err(GnmDenseError::InvalidMapping {
                    row: Some(row_index),
                    reason: "base weight must be finite and positive".to_owned(),
                });
            }
            if let Some(previous) = rows[..row_index]
                .iter()
                .position(|candidate| candidate.target == row.target)
            {
                return Err(GnmDenseError::InvalidMapping {
                    row: Some(row_index),
                    reason: format!("duplicate GNM surface target already used by row {previous}"),
                });
            }
            seen_source[row.mediapipe_index] = true;
            landmarks.push(row.target.to_sparse_landmark(model.vertex_count()).map_err(
                |error| GnmDenseError::InvalidMapping {
                    row: Some(row_index),
                    reason: error.to_string(),
                },
            )?);
        }

        Ok(Self {
            version,
            rows,
            surface_landmarks: SparseLandmarkSet::new(landmarks).map_err(GnmDenseError::Model)?,
        })
    }

    /// Returns the mapping version binding.
    pub fn version(&self) -> DenseMappingVersion {
        self.version
    }

    /// Returns correspondence rows in stable source order.
    pub fn rows(&self) -> &[MediaPipeGnmDenseCorrespondence] {
        &self.rows
    }

    /// Returns the number of mapped points.
    pub fn len(&self) -> usize {
        self.rows.len()
    }

    /// Returns whether this set has no rows.
    pub fn is_empty(&self) -> bool {
        self.rows.is_empty()
    }

    /// Enforces the Issue #53 rule that the primary observation must be denser
    /// than the existing 68-point bootstrap.
    pub fn validate_as_primary_observation(&self) -> Result<(), GnmDenseError> {
        if self.rows.len() <= SPARSE_BOOTSTRAP_POINT_COUNT {
            return Err(GnmDenseError::InsufficientDensity {
                mapped: self.rows.len(),
                sparse_bootstrap: SPARSE_BOOTSTRAP_POINT_COUNT,
            });
        }
        Ok(())
    }

    /// Verifies that this mapping is still compatible with the loaded model.
    pub fn validate_model(&self, model: &GnmModel) -> Result<(), GnmDenseError> {
        if self.version.model_version != model.version() {
            return Err(GnmDenseError::ModelVersionMismatch {
                mapping: self.version.model_version,
                model: model.version(),
            });
        }
        for (row_index, row) in self.rows.iter().enumerate() {
            row.target
                .to_sparse_landmark(model.vertex_count())
                .map_err(|error| GnmDenseError::InvalidMapping {
                    row: Some(row_index),
                    reason: error.to_string(),
                })?;
        }
        Ok(())
    }

    /// Evaluates only the GNM surface points referenced by this mapping.
    ///
    /// The model reuses its existing selected-vertex scratch buffers; no render
    /// mesh or material dependency is introduced here.
    pub fn evaluate_surface(
        &self,
        model: &GnmModel,
        identity: &GnmIdentityState,
        expression: &GnmExpressionState,
        joints: &GnmJointState,
        output: &mut GnmSparseVertices,
    ) -> Result<(), GnmDenseError> {
        self.validate_model(model)?;
        model
            .evaluate_sparse(
                identity,
                expression,
                joints,
                &self.surface_landmarks,
                output,
            )
            .map_err(GnmDenseError::Model)
    }
}

/// Coverage thresholds for a dense observation.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DenseCoveragePolicy {
    min_valid_points: usize,
    degraded_below_fraction: f32,
}

impl DenseCoveragePolicy {
    /// Creates a finite coverage policy.
    pub fn new(
        min_valid_points: usize,
        degraded_below_fraction: f32,
    ) -> Result<Self, GnmDenseError> {
        if min_valid_points == 0 {
            return Err(GnmDenseError::InvalidCoveragePolicy(
                "min_valid_points must be positive",
            ));
        }
        if !degraded_below_fraction.is_finite()
            || !(0.0..=1.0).contains(&degraded_below_fraction)
        {
            return Err(GnmDenseError::InvalidCoveragePolicy(
                "degraded_below_fraction must be finite and within [0, 1]",
            ));
        }
        Ok(Self {
            min_valid_points,
            degraded_below_fraction,
        })
    }

    /// Returns the minimum valid point count.
    pub fn min_valid_points(self) -> usize {
        self.min_valid_points
    }

    /// Returns the fraction below which an otherwise usable observation is degraded.
    pub fn degraded_below_fraction(self) -> f32 {
        self.degraded_below_fraction
    }
}

/// Typed observation coverage state.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DenseObservationStatus {
    /// Coverage satisfies the configured primary threshold.
    Valid,
    /// Enough points remain to use the observation, but coverage is reduced.
    Degraded,
    /// Too few valid points remain for the configured minimum.
    Insufficient,
}

/// Summary of dense observation coverage.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DenseCoverageSummary {
    /// Number of correspondence rows in the mapping.
    pub mapped_points: usize,
    /// Number of finite in-range points retained from this frame.
    pub valid_points: usize,
    /// Sum of static weights for retained points.
    pub effective_weight: f32,
    /// Typed coverage state.
    pub status: DenseObservationStatus,
}

/// One valid point in an engine-neutral dense observation.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GnmDenseObservationPoint {
    /// Stable row index in the dense correspondence set.
    pub mapping_index: usize,
    /// GNM surface target for this observation.
    pub target: GnmSurfacePointRef,
    /// Canonical normalized image coordinate: x right, y down, both in `[0, 1]`.
    pub normalized_xy: [f32; 2],
    /// Static objective weight before runtime robust/confidence weighting.
    pub weight: f32,
    /// Facial region.
    pub region: FaceRegion,
    /// Subject-relative anatomical side.
    pub anatomical_side: AnatomicalSide,
    /// Original MediaPipe landmark index, retained for diagnostics only.
    pub source_landmark_index: usize,
    /// Per-point tracker confidence when the source actually provides it.
    /// MediaPipe Face Landmarker currently enters this adapter as `None`.
    pub source_confidence: Option<f32>,
}

/// Engine-neutral dense 2D observation for one source frame.
#[derive(Clone, Debug, PartialEq)]
pub struct GnmDenseObservation {
    source_seq: u64,
    captured_at_micros: u64,
    points: Vec<GnmDenseObservationPoint>,
    coverage: DenseCoverageSummary,
}

impl GnmDenseObservation {
    /// Builds a dense observation from all 478 MediaPipe normalized `(x, y)`
    /// points. Non-finite or out-of-range points are excluded rather than
    /// poisoning the entire frame.
    ///
    /// No `z` coordinate and no preview-mirror flag are accepted by this API.
    pub fn from_mediapipe_xy(
        source_seq: u64,
        captured_at_micros: u64,
        landmarks: &[[f32; 2]],
        mapping: &DenseCorrespondenceSet,
        policy: DenseCoveragePolicy,
    ) -> Result<Self, GnmDenseError> {
        if landmarks.len() != MEDIAPIPE_FACE_LANDMARK_COUNT {
            return Err(GnmDenseError::Shape {
                field: "MediaPipe normalized_xy",
                expected: MEDIAPIPE_FACE_LANDMARK_COUNT,
                actual: landmarks.len(),
            });
        }
        if policy.min_valid_points > mapping.len() {
            return Err(GnmDenseError::InvalidCoveragePolicy(
                "min_valid_points exceeds mapped point count",
            ));
        }

        let mut points = Vec::with_capacity(mapping.len());
        let mut effective_weight = 0.0;
        for (mapping_index, row) in mapping.rows.iter().enumerate() {
            let Some(normalized_xy) = canonicalize_mediapipe_xy(landmarks[row.mediapipe_index])
            else {
                continue;
            };
            effective_weight += row.base_weight;
            points.push(GnmDenseObservationPoint {
                mapping_index,
                target: row.target,
                normalized_xy,
                weight: row.base_weight,
                region: row.region,
                anatomical_side: row.anatomical_side,
                source_landmark_index: row.mediapipe_index,
                source_confidence: None,
            });
        }

        let valid_points = points.len();
        let valid_fraction = valid_points as f32 / mapping.len() as f32;
        let status = if valid_points < policy.min_valid_points {
            DenseObservationStatus::Insufficient
        } else if valid_fraction < policy.degraded_below_fraction {
            DenseObservationStatus::Degraded
        } else {
            DenseObservationStatus::Valid
        };

        Ok(Self {
            source_seq,
            captured_at_micros,
            points,
            coverage: DenseCoverageSummary {
                mapped_points: mapping.len(),
                valid_points,
                effective_weight,
                status,
            },
        })
    }

    /// Returns the source frame sequence number.
    pub fn source_seq(&self) -> u64 {
        self.source_seq
    }

    /// Returns the monotonic capture timestamp in microseconds.
    pub fn captured_at_micros(&self) -> u64 {
        self.captured_at_micros
    }

    /// Returns valid points in stable mapping order.
    pub fn points(&self) -> &[GnmDenseObservationPoint] {
        &self.points
    }

    /// Returns coverage diagnostics for this frame.
    pub fn coverage(&self) -> DenseCoverageSummary {
        self.coverage
    }
}

/// Converts one MediaPipe normalized image point into the canonical image space.
///
/// The canonical space intentionally matches MediaPipe image semantics: x grows
/// rightward and y grows downward. Preview mirroring is presentation-only and is
/// therefore absent from this function and from correspondence lookup.
pub fn canonicalize_mediapipe_xy(point: [f32; 2]) -> Option<[f32; 2]> {
    if point
        .iter()
        .all(|value| value.is_finite() && (0.0..=1.0).contains(value))
    {
        Some(point)
    } else {
        None
    }
}

/// Typed failure from dense correspondence or observation validation.
#[derive(Debug)]
pub enum GnmDenseError {
    /// Mapping row validation failed.
    InvalidMapping {
        /// Row index when the failure belongs to a specific row.
        row: Option<usize>,
        /// Stable human-readable validation reason.
        reason: String,
    },
    /// Mapping and loaded model schema versions differ.
    ModelVersionMismatch {
        /// Model version recorded by the mapping.
        mapping: GnmVersion,
        /// Model version currently loaded.
        model: GnmVersion,
    },
    /// Mapping has not yet exceeded the 68-point bootstrap density.
    InsufficientDensity {
        /// Number of mapped points.
        mapped: usize,
        /// Existing sparse bootstrap point count.
        sparse_bootstrap: usize,
    },
    /// Input vector shape is invalid.
    Shape {
        /// Input field name.
        field: &'static str,
        /// Required length.
        expected: usize,
        /// Actual length.
        actual: usize,
    },
    /// Coverage policy is internally inconsistent.
    InvalidCoveragePolicy(&'static str),
    /// Underlying validated GNM model failure.
    Model(GnmModelError),
}

impl std::fmt::Display for GnmDenseError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidMapping { row, reason } => match row {
                Some(row) => write!(formatter, "invalid dense mapping row {row}: {reason}"),
                None => write!(formatter, "invalid dense mapping: {reason}"),
            },
            Self::ModelVersionMismatch { mapping, model } => write!(
                formatter,
                "dense mapping targets GNM {}.{}, but loaded model is {}.{}",
                mapping.major, mapping.minor, model.major, model.minor
            ),
            Self::InsufficientDensity {
                mapped,
                sparse_bootstrap,
            } => write!(
                formatter,
                "dense primary mapping requires more than {sparse_bootstrap} points, got {mapped}"
            ),
            Self::Shape {
                field,
                expected,
                actual,
            } => write!(
                formatter,
                "{field} length mismatch: expected {expected}, got {actual}"
            ),
            Self::InvalidCoveragePolicy(reason) => {
                write!(formatter, "invalid dense coverage policy: {reason}")
            }
            Self::Model(error) => write!(formatter, "{error}"),
        }
    }
}

impl std::error::Error for GnmDenseError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Model(error) => Some(error),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        DenseArray, GNM_HEAD_V3_EXPRESSION_DIM, GNM_HEAD_V3_IDENTITY_DIM, GNM_HEAD_V3_VERSION,
        GnmModelData, GnmVariant,
    };

    fn synthetic_model(vertex_count: usize) -> GnmModel {
        let identity = GNM_HEAD_V3_IDENTITY_DIM;
        let expression = GNM_HEAD_V3_EXPRESSION_DIM;
        let mut vertices = Vec::with_capacity(vertex_count * 3);
        for index in 0..vertex_count {
            vertices.extend_from_slice(&[index as f32, (index % 3) as f32, 0.0]);
        }
        GnmModel::from_data(GnmModelData {
            version: GNM_HEAD_V3_VERSION,
            variant: GnmVariant::Head,
            template_vertices: DenseArray::new("vertices", vec![vertex_count, 3], vertices).unwrap(),
            template_joints: DenseArray::new("joints", vec![1, 3], vec![0.0; 3]).unwrap(),
            vertex_identity_basis: DenseArray::new(
                "identity",
                vec![identity, vertex_count, 3],
                vec![0.0; identity * vertex_count * 3],
            )
            .unwrap(),
            joint_identity_basis: DenseArray::new(
                "joint_identity",
                vec![identity, 1, 3],
                vec![0.0; identity * 3],
            )
            .unwrap(),
            expression_basis: DenseArray::new(
                "expression",
                vec![expression, vertex_count, 3],
                vec![0.0; expression * vertex_count * 3],
            )
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
        .unwrap()
    }

    fn version() -> DenseMappingVersion {
        DenseMappingVersion {
            schema_revision: 1,
            model_version: GNM_HEAD_V3_VERSION,
        }
    }

    fn row(index: usize) -> MediaPipeGnmDenseCorrespondence {
        MediaPipeGnmDenseCorrespondence {
            mediapipe_index: index,
            target: GnmSurfacePointRef::Vertex {
                vertex_index: index,
            },
            region: FaceRegion::Other,
            anatomical_side: AnatomicalSide::Midline,
            base_weight: 1.0,
            provenance: CorrespondenceProvenance::RepositoryValidated,
            reliability: CorrespondenceReliability::High,
        }
    }

    #[test]
    fn primary_density_must_exceed_sparse_68() {
        let model = synthetic_model(69);
        let sparse_sized = DenseCorrespondenceSet::new(
            version(),
            (0..SPARSE_BOOTSTRAP_POINT_COUNT).map(row).collect(),
            &model,
        )
        .unwrap();
        assert!(matches!(
            sparse_sized.validate_as_primary_observation(),
            Err(GnmDenseError::InsufficientDensity { mapped: 68, .. })
        ));

        let dense = DenseCorrespondenceSet::new(version(), (0..69).map(row).collect(), &model)
            .unwrap();
        assert!(dense.validate_as_primary_observation().is_ok());
    }

    #[test]
    fn duplicate_source_and_target_are_rejected() {
        let model = synthetic_model(3);
        let mut duplicate_source = vec![row(0), row(1)];
        duplicate_source[1].mediapipe_index = 0;
        assert!(matches!(
            DenseCorrespondenceSet::new(version(), duplicate_source, &model),
            Err(GnmDenseError::InvalidMapping { row: Some(1), .. })
        ));

        let mut duplicate_target = vec![row(0), row(1)];
        duplicate_target[1].target = duplicate_target[0].target;
        assert!(matches!(
            DenseCorrespondenceSet::new(version(), duplicate_target, &model),
            Err(GnmDenseError::InvalidMapping { row: Some(1), .. })
        ));
    }

    #[test]
    fn barycentric_targets_are_validated_and_evaluated_without_render_mesh() {
        let model = synthetic_model(3);
        let mapping = DenseCorrespondenceSet::new(
            version(),
            vec![MediaPipeGnmDenseCorrespondence {
                mediapipe_index: 10,
                target: GnmSurfacePointRef::Barycentric {
                    vertex_indices: [0, 1, 2],
                    weights: [0.25, 0.25, 0.5],
                },
                region: FaceRegion::Mouth,
                anatomical_side: AnatomicalSide::Midline,
                base_weight: 0.8,
                provenance: CorrespondenceProvenance::ResearchDerived,
                reliability: CorrespondenceReliability::Medium,
            }],
            &model,
        )
        .unwrap();
        let mut output = GnmSparseVertices::with_len(1);
        mapping
            .evaluate_surface(
                &model,
                &model.neutral_identity(),
                &model.neutral_expression(),
                &GnmJointState::neutral(model.joint_count()),
                &mut output,
            )
            .unwrap();
        let value = output.values()[0];
        assert!((value[0] - 1.25).abs() < 1.0e-6);
        assert!((value[1] - 1.25).abs() < 1.0e-6);
        assert!(value[2].abs() < 1.0e-6);
    }

    #[test]
    fn invalid_mediapipe_points_are_excluded_with_typed_coverage() {
        let model = synthetic_model(4);
        let mapping = DenseCorrespondenceSet::new(version(), (0..4).map(row).collect(), &model)
            .unwrap();
        let mut landmarks = vec![[0.5, 0.5]; MEDIAPIPE_FACE_LANDMARK_COUNT];
        landmarks[1] = [f32::NAN, 0.5];
        landmarks[2] = [1.2, 0.5];

        let degraded = GnmDenseObservation::from_mediapipe_xy(
            42,
            1_000,
            &landmarks,
            &mapping,
            DenseCoveragePolicy::new(2, 0.75).unwrap(),
        )
        .unwrap();
        assert_eq!(degraded.source_seq(), 42);
        assert_eq!(degraded.points().len(), 2);
        assert_eq!(degraded.coverage().status, DenseObservationStatus::Degraded);
        assert!(
            degraded
                .points()
                .iter()
                .all(|point| point.source_confidence.is_none())
        );

        let insufficient = GnmDenseObservation::from_mediapipe_xy(
            43,
            2_000,
            &landmarks,
            &mapping,
            DenseCoveragePolicy::new(3, 0.75).unwrap(),
        )
        .unwrap();
        assert_eq!(
            insufficient.coverage().status,
            DenseObservationStatus::Insufficient
        );
    }

    #[test]
    fn canonical_image_conversion_is_not_preview_mirror_dependent() {
        assert_eq!(canonicalize_mediapipe_xy([0.2, 0.8]), Some([0.2, 0.8]));
        assert_eq!(canonicalize_mediapipe_xy([0.2, f32::INFINITY]), None);
        assert_eq!(canonicalize_mediapipe_xy([-0.1, 0.8]), None);
    }

    #[test]
    fn mapping_model_version_mismatch_is_fail_closed() {
        let model = synthetic_model(1);
        let mismatched = DenseMappingVersion {
            schema_revision: 1,
            model_version: GnmVersion { major: 9, minor: 9 },
        };
        assert!(matches!(
            DenseCorrespondenceSet::new(mismatched, vec![row(0)], &model),
            Err(GnmDenseError::ModelVersionMismatch { .. })
        ));
    }
}

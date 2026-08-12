//! Rest orientation cache for pose application.
//!
//! Captures bone rest rotations at binding time so the pose apply system
//! does not need to query `RestTransform` every frame. The cache is tied
//! to an avatar generation and must be rebuilt when the avatar changes.

use bevy::math::Quat;
use bevy::prelude::*;
use bevy_vrm1::prelude::RestTransform;

use crate::binding::AvatarBinding;
use crate::lifecycle::AvatarGeneration;

/// Error extracting rest orientation from a bone entity.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RestOrientationError {
    /// The bone entity has no `GlobalTransform` yet.
    MissingGlobalTransform {
        /// Bone name (e.g. "head", "neck").
        bone: &'static str,
        /// Entity that was expected to have the component.
        entity: Entity,
    },
    /// The bone entity has no `RestTransform` component.
    MissingRestTransform {
        /// Bone name (e.g. "head", "neck").
        bone: &'static str,
        /// Entity that was expected to have the component.
        entity: Entity,
    },
    /// The `RestTransform` contains non-uniform scale, making rotation
    /// extraction ambiguous.
    NonUniformScale {
        /// Bone name.
        bone: &'static str,
        /// Entity with the problematic transform.
        entity: Entity,
    },
    /// The `RestTransform` rotation could not be normalized (zero or NaN).
    InvalidRotation {
        /// Bone name.
        bone: &'static str,
        /// Entity with the problematic transform.
        entity: Entity,
    },
}

impl std::fmt::Display for RestOrientationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MissingGlobalTransform { bone, entity } => {
                write!(f, "bone `{bone}` entity {entity:?} has no GlobalTransform")
            }
            Self::MissingRestTransform { bone, entity } => {
                write!(f, "bone `{bone}` entity {entity:?} has no RestTransform")
            }
            Self::NonUniformScale { bone, entity } => {
                write!(
                    f,
                    "bone `{bone}` entity {entity:?} has non-uniform scale in RestTransform"
                )
            }
            Self::InvalidRotation { bone, entity } => {
                write!(
                    f,
                    "bone `{bone}` entity {entity:?} has invalid rotation in RestTransform"
                )
            }
        }
    }
}

impl std::error::Error for RestOrientationError {}

/// Cached rest orientations for the active avatar's root, head, and neck bones.
///
/// Built once at binding time and reused every frame by the pose apply system.
/// Must be rebuilt when the avatar generation changes.
#[derive(Clone, Debug, Component)]
pub struct RestOrientationCache {
    /// Avatar generation this cache was built for.
    pub generation: AvatarGeneration,
    /// Root entity's rest global rotation.
    pub root_rest_global: Quat,
    /// Head bone rest local rotation.
    pub head_rest_local: Quat,
    /// Head bone rest global rotation.
    pub head_rest_global: Quat,
    /// Neck bone rest local rotation (if present).
    pub neck_rest_local: Option<Quat>,
    /// Neck bone rest global rotation (if present).
    pub neck_rest_global: Option<Quat>,
}

/// Scale uniformity tolerance for rotation extraction.
const SCALE_UNIFORMITY_EPSILON: f32 = 1e-4;

/// Extract rest orientation cache from a bound avatar.
///
/// Reads `RestTransform` from the root, head, and optionally neck entities.
/// Returns a typed error if any required component is missing or has
/// non-uniform scale.
pub fn build_rest_orientation_cache(
    generation: AvatarGeneration,
    binding: &AvatarBinding,
    root_query: &Query<Option<&RestTransform>>,
    bone_query: &Query<(
        Option<&Transform>,
        Option<&GlobalTransform>,
        Option<&RestTransform>,
    )>,
) -> Result<RestOrientationCache, RestOrientationError> {
    // Root rest global rotation.
    let root_rest = root_query.get(binding.root).ok().flatten();
    let root_rest_global = root_rest
        .map(|r| extract_rotation(&r.0, "root", binding.root))
        .transpose()?
        .unwrap_or(Quat::IDENTITY);
    // If root has no RestTransform, use identity (common for root entities).

    // Head rest local and global.
    let (head_t, head_gt, head_rest) = bone_query.get(binding.head).unwrap_or((None, None, None));
    let _head_t = head_t.ok_or(RestOrientationError::MissingRestTransform {
        bone: "head",
        entity: binding.head,
    })?;
    let head_gt = head_gt.ok_or(RestOrientationError::MissingGlobalTransform {
        bone: "head",
        entity: binding.head,
    })?;
    let head_rest_transform = head_rest.ok_or(RestOrientationError::MissingRestTransform {
        bone: "head",
        entity: binding.head,
    })?;
    let head_rest_local = extract_rotation(&head_rest_transform.0, "head", binding.head)?;
    let head_rest_global = extract_global_rotation(head_gt);

    // Neck rest local and global (optional).
    let (neck_rest_local, neck_rest_global) = if let Some(neck_entity) = binding.neck {
        let (neck_t, neck_gt, neck_rest) =
            bone_query.get(neck_entity).unwrap_or((None, None, None));
        let _neck_t = neck_t.ok_or(RestOrientationError::MissingRestTransform {
            bone: "neck",
            entity: neck_entity,
        })?;
        let neck_gt = neck_gt.ok_or(RestOrientationError::MissingGlobalTransform {
            bone: "neck",
            entity: neck_entity,
        })?;
        let neck_rest_transform = neck_rest.ok_or(RestOrientationError::MissingRestTransform {
            bone: "neck",
            entity: neck_entity,
        })?;
        let local = extract_rotation(&neck_rest_transform.0, "neck", neck_entity)?;
        let global = extract_global_rotation(neck_gt);
        (Some(local), Some(global))
    } else {
        (None, None)
    };

    Ok(RestOrientationCache {
        generation,
        root_rest_global,
        head_rest_local,
        head_rest_global,
        neck_rest_local,
        neck_rest_global,
    })
}

/// Extract a unit quaternion from a `Transform`, rejecting non-uniform scale.
fn extract_rotation(
    transform: &Transform,
    bone: &'static str,
    entity: Entity,
) -> Result<Quat, RestOrientationError> {
    let scale = transform.scale;
    if !is_uniform_scale(scale) {
        return Err(RestOrientationError::NonUniformScale { bone, entity });
    }
    let rotation = transform.rotation;
    if !is_finite_quat(rotation) {
        return Err(RestOrientationError::InvalidRotation { bone, entity });
    }
    Ok(rotation.normalize())
}

/// Extract rotation from a `GlobalTransform`, ignoring scale.
fn extract_global_rotation(gt: &GlobalTransform) -> Quat {
    gt.to_scale_rotation_translation().1.normalize()
}

/// Check if a scale vector is approximately uniform.
fn is_uniform_scale(scale: Vec3) -> bool {
    let max = scale.x.abs().max(scale.y.abs()).max(scale.z.abs());
    let min = scale.x.abs().min(scale.y.abs()).min(scale.z.abs());
    max - min < SCALE_UNIFORMITY_EPSILON * max.max(1.0)
}

/// Check if a quaternion has all finite components and non-zero norm.
fn is_finite_quat(q: Quat) -> bool {
    q.x.is_finite()
        && q.y.is_finite()
        && q.z.is_finite()
        && q.w.is_finite()
        && q.length_squared() > 1e-10
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pose_rest_cache_uniform_scale_accepted() {
        let t = Transform {
            translation: Vec3::ZERO,
            rotation: Quat::from_rotation_y(0.5),
            scale: Vec3::splat(2.0),
        };
        let result = extract_rotation(&t, "head", Entity::PLACEHOLDER);
        assert!(result.is_ok());
    }

    #[test]
    fn pose_rest_cache_non_uniform_scale_rejected() {
        let t = Transform {
            translation: Vec3::ZERO,
            rotation: Quat::IDENTITY,
            scale: Vec3::new(1.0, 2.0, 1.0),
        };
        let result = extract_rotation(&t, "head", Entity::PLACEHOLDER);
        assert!(matches!(
            result,
            Err(RestOrientationError::NonUniformScale { .. })
        ));
    }

    #[test]
    fn pose_rest_cache_nan_rotation_rejected() {
        let t = Transform {
            translation: Vec3::ZERO,
            rotation: Quat::from_xyzw(f32::NAN, 0.0, 0.0, 1.0),
            scale: Vec3::ONE,
        };
        let result = extract_rotation(&t, "neck", Entity::PLACEHOLDER);
        assert!(matches!(
            result,
            Err(RestOrientationError::InvalidRotation { .. })
        ));
    }

    #[test]
    fn pose_rest_cache_identity_rest_preserved() {
        let t = Transform::IDENTITY;
        let q = extract_rotation(&t, "head", Entity::PLACEHOLDER).unwrap();
        assert!(q.abs_diff_eq(Quat::IDENTITY, 1e-6));
    }

    #[test]
    fn pose_rest_cache_non_identity_rest_preserved() {
        let expected = Quat::from_euler(bevy::math::EulerRot::XYZ, 0.1, 0.2, 0.3);
        let t = Transform {
            translation: Vec3::new(1.0, 2.0, 3.0),
            rotation: expected,
            scale: Vec3::ONE,
        };
        let q = extract_rotation(&t, "head", Entity::PLACEHOLDER).unwrap();
        assert!(q.abs_diff_eq(expected, 1e-6));
    }

    #[test]
    fn pose_rest_cache_is_uniform_scale() {
        assert!(is_uniform_scale(Vec3::ONE));
        assert!(is_uniform_scale(Vec3::splat(0.5)));
        assert!(is_uniform_scale(Vec3::splat(100.0)));
        assert!(!is_uniform_scale(Vec3::new(1.0, 1.01, 1.0)));
        assert!(!is_uniform_scale(Vec3::new(1.0, 2.0, 1.0)));
    }
}

pub mod extensions;

pub mod materials;

pub mod prelude {
    pub use crate::vrm::gltf::{
        extensions::{
            CoordinateBasis, VrmExtensions, VrmFirstPerson, VrmFirstPersonFlag, VrmGeneration,
            VrmHumanoid, VrmLookAt, VrmLookAtType, VrmMeshAnnotation, VrmMeta, VrmNode,
            VrmParseError, VrmRangeMap, VrmRuntimeDescriptor, parse_runtime_descriptor,
            vrmc_spring_bone::*, vrmc_vrm::*,
        },
        materials::*,
    };
}

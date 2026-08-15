use crate::prelude::*;
use crate::vrm::gltf::materials::LegacyAlphaMode;
use bevy::app::{App, Plugin};
use bevy::asset::Assets;
use bevy::math::{Affine2, Vec2};
use bevy::prelude::*;
use bevy::render::render_resource::Face;

pub struct MToonMaterialSetupPlugin;

impl Plugin for MToonMaterialSetupPlugin {
    fn build(
        &self,
        app: &mut App,
    ) {
        app.add_systems(Update, turn_to_mtoon_material);
    }
}

fn turn_to_mtoon_material(
    mut commands: Commands,
    mut mtoon_materials: ResMut<Assets<MToonMaterial>>,
    mut standard_materials: ResMut<Assets<StandardMaterial>>,
    registries: Query<&VrmcMaterialRegistry>,
    parents: Query<&ChildOf>,
    added_materials: Query<
        (Entity, &MeshMaterial3d<StandardMaterial>),
        Added<MeshMaterial3d<StandardMaterial>>,
    >,
) {
    added_materials.iter().for_each(|(entity, handle)| {
        let root = parents.root_ancestor(entity);
        let Ok(registry) = registries.get(root) else {
            return;
        };
        let Some(extension) = registry.materials.get(&handle.id()) else {
            return;
        };
        if extension.legacy_standard_fallback {
            if let Some(mut material) = standard_materials.get_mut(handle.id()) {
                material.unlit = true;
                material.base_color_texture = extension
                    .legacy_base_texture
                    .and_then(|index| registry.images.get(index))
                    .cloned()
                    .or_else(|| material.base_color_texture.clone());
                if let Some(color) = extension.legacy_base_color {
                    material.base_color = Color::linear_rgba(color[0], color[1], color[2], color[3]);
                }
                if let Some(alpha_mode) = extension.legacy_alpha_mode {
                    material.alpha_mode = match alpha_mode {
                        LegacyAlphaMode::Opaque => AlphaMode::Opaque,
                        LegacyAlphaMode::Mask(cutoff) => AlphaMode::Mask(cutoff),
                        LegacyAlphaMode::Blend => AlphaMode::Blend,
                    };
                }
                if let Some(double_sided) = extension.legacy_double_sided {
                    material.double_sided = double_sided;
                    material.cull_mode = if double_sided { None } else { Some(Face::Back) };
                }
                if let Some(transform) = extension.legacy_uv_transform {
                    material.uv_transform = Affine2::from_scale_angle_translation(
                        Vec2::new(transform[0], transform[1]),
                        0.0,
                        Vec2::new(transform[2], transform[3]),
                    );
                }
            }
            return;
        }
        let Some(base) = standard_materials.get(handle.id()).cloned() else {
            return;
        };
        let legacy_double_sided = extension.legacy_double_sided;
        let mut cmd = commands.entity(entity);
        cmd.remove::<MeshMaterial3d<StandardMaterial>>()
            .insert(MeshMaterial3d(
                mtoon_materials.add(MToonMaterial {
                    base_color_texture: extension
                        .legacy_base_texture
                        .and_then(|index| registry.images.get(index))
                        .cloned()
                        .or_else(|| base.base_color_texture.clone()),
                    uv_animation_mask_texture: extension
                        .uv_animation_mask_texture
                        .and_then(|tex| registry.images.get(tex.index))
                        .cloned(),
                    shade_multiply_texture: extension
                        .shade_multiply_texture
                        .and_then(|tex| registry.images.get(tex.index))
                        .cloned(),
                    shading_shift_texture: extension
                        .shading_shift_texture
                        .and_then(|tex| registry.images.get(tex.index))
                        .cloned(),
                    matcap_texture: extension
                        .matcap_texture
                        .and_then(|tex| registry.images.get(tex.index))
                        .cloned(),
                    rim_multiply_texture: extension
                        .rim_multiply_texture
                        .and_then(|tex| registry.images.get(tex.index))
                        .cloned(),
                    outline_width_multiply_texture: extension
                        .outline_width_multiply_texture
                        .and_then(|tex| registry.images.get(tex.index))
                        .cloned(),
                    shade: Shade::from(extension),
                    outline: MToonOutline::from(extension),
                    rim_lighting: RimLighting::from(extension),
                    uv_animation: UVAnimation::from(extension),
                    gi_equalization_factor: extension.gi_equalization_factor,
                    double_sided: legacy_double_sided.unwrap_or(base.double_sided),
                    alpha_mode: extension
                        .legacy_alpha_mode
                        .map(|mode| match mode {
                            LegacyAlphaMode::Opaque => AlphaMode::Opaque,
                            LegacyAlphaMode::Mask(cutoff) => AlphaMode::Mask(cutoff),
                            LegacyAlphaMode::Blend => AlphaMode::Blend,
                        })
                        .unwrap_or(base.alpha_mode),
                    depth_bias: base.depth_bias,
                    render_queue_offset: extension.render_queue_offset_number,
                    transparent_with_z_write: extension.transparent_with_z_write,
                    opaque_renderer_method: base.opaque_render_method,
                    base_color: extension
                        .legacy_base_color
                        .map(|color| Color::linear_rgba(color[0], color[1], color[2], color[3]))
                        .unwrap_or(base.base_color),
                    cull_mode: if legacy_double_sided == Some(true) {
                        None
                    } else {
                        base.cull_mode
                    },
                    emissive: extension
                        .legacy_emissive
                        .map(|color| LinearRgba::rgb(color[0], color[1], color[2]))
                        .unwrap_or(base.emissive),
                    emissive_texture: extension
                        .legacy_emissive_texture
                        .and_then(|index| registry.images.get(index))
                        .cloned()
                        .or_else(|| base.emissive_texture.clone()),
                    uv_transform: extension
                        .legacy_uv_transform
                        .map(|transform| {
                            Affine2::from_scale_angle_translation(
                                Vec2::new(transform[0], transform[1]),
                                0.0,
                                Vec2::new(transform[2], transform[3]),
                            )
                        })
                        .unwrap_or(base.uv_transform),
                }),
            ));
    });
}

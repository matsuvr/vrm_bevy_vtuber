use bevy::color::LinearRgba;
use bevy::prelude::Reflect;
use bevy::render::render_resource::ShaderType;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

#[derive(Serialize, Deserialize, Reflect, Debug, Clone)]
pub struct VrmcMaterialsExtensitions {
    /// Indicates the version number of `VRMC_materials_mtoon` extension.
    ///
    /// The value is fixed to "1.0".
    #[serde(rename = "specVersion")]
    pub spec_version: String,
    #[serde(rename = "matcapFactor")]
    pub matcap_factor: [f32; 3],
    #[serde(rename = "matcapTexture")]
    pub matcap_texture: Option<MatcapTexture>,
    #[serde(
        rename = "parametricRimFresnelPowerFactor",
        default = "default_parametric_rim_fresnel_power"
    )]
    pub parametric_rim_fresnel_power: f32,
    #[serde(rename = "rimMultiplyTexture")]
    pub rim_multiply_texture: Option<RimMultiplyTexture>,
    #[serde(rename = "outlineColorFactor")]
    pub outline_color_factor: [f32; 3],
    #[serde(rename = "outlineLightingMixFactor")]
    pub outline_lighting_mix_factor: f32,
    #[serde(rename = "outlineWidthFactor")]
    pub outline_width_factor: Option<f32>,
    #[serde(rename = "outlineWidthMultiplyTexture")]
    pub outline_width_multiply_texture: Option<OutlineWidthMultiplyTexture>,
    #[serde(rename = "outlineWidthMode")]
    pub outline_width_mode: String,
    #[serde(rename = "parametricRimColorFactor")]
    pub parametric_rim_color_factor: [f32; 3],
    #[serde(rename = "parametricRimLiftFactor")]
    pub parametric_rim_lift_factor: f32,
    #[serde(rename = "rimLightingMixFactor")]
    pub rim_lighting_mix_factor: f32,
    /// The shade color.
    /// The value is evaluated in linear color space.
    #[serde(rename = "shadeColorFactor")]
    pub shade_color_factor: [f32; 3],
    #[serde(rename = "shadeMultiplyTexture")]
    pub shade_multiply_texture: Option<VrmTexture>,
    #[serde(rename = "renderQueueOffsetNumber")]
    pub render_queue_offset_number: f32,
    #[serde(rename = "shadingShiftFactor")]
    pub shading_shift_factor: f32,
    #[serde(rename = "shadingShiftTexture")]
    pub shading_shift_texture: Option<ShadingShiftTexture>,
    #[serde(rename = "shadingToonyFactor")]
    pub shading_toony_factor: f32,
    #[serde(rename = "transparentWithZWrite")]
    pub transparent_with_z_write: bool,
    #[serde(rename = "uvAnimationMaskTexture")]
    pub uv_animation_mask_texture: Option<UVAnimationMaskTexture>,
    #[serde(rename = "uvAnimationRotationSpeedFactor")]
    pub uv_animation_rotation_speed_factor: f32,
    #[serde(rename = "uvAnimationScrollXSpeedFactor")]
    pub uv_animation_scroll_x_speed_factor: f32,
    #[serde(rename = "uvAnimationScrollYSpeedFactor")]
    pub uv_animation_scroll_y_speed_factor: f32,
    #[serde(rename = "giEqualizationFactor")]
    pub gi_equalization_factor: f32,
    /// Alpha behavior carried by the legacy shader/tag contract.
    ///
    /// This is intentionally not serialized: it is a compatibility hint used
    /// while constructing the existing renderer material.
    #[serde(skip)]
    #[reflect(ignore)]
    pub legacy_alpha_mode: Option<LegacyAlphaMode>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum LegacyAlphaMode {
    Opaque,
    Mask(f32),
    Blend,
}

fn default_parametric_rim_fresnel_power() -> f32 {
    5.0
}

impl VrmcMaterialsExtensitions {
    pub fn shade_color(&self) -> LinearRgba {
        let c = self.shade_color_factor;
        LinearRgba::rgb(c[0], c[1], c[2])
    }

    pub fn parametric_rim_color(&self) -> LinearRgba {
        let c = self.parametric_rim_color_factor;
        LinearRgba::rgb(c[0], c[1], c[2])
    }

    pub fn matcap_color(&self) -> LinearRgba {
        let c = self.matcap_factor;
        LinearRgba::rgb(c[0], c[1], c[2])
    }
}

/// Converts one VRM 0.x materialProperties entry into the existing `MToon`
/// material contract. Legacy values are normalized by name and texture index;
/// the renderer remains the same `MToon` renderer used for VRM 1.0.
pub fn convert_legacy_material_properties(value: &Value) -> Option<VrmcMaterialsExtensitions> {
    let mut properties = json!({
        "specVersion": "1.0",
        "matcapFactor": [1.0, 1.0, 1.0],
        "matcapTexture": null,
        "parametricRimFresnelPowerFactor": 5.0,
        "rimMultiplyTexture": null,
        "outlineColorFactor": [0.0, 0.0, 0.0],
        "outlineLightingMixFactor": 0.0,
        "outlineWidthFactor": null,
        "outlineWidthMultiplyTexture": null,
        "outlineWidthMode": "none",
        "parametricRimColorFactor": [0.0, 0.0, 0.0],
        "parametricRimLiftFactor": 0.0,
        "rimLightingMixFactor": 1.0,
        "shadeColorFactor": [0.0, 0.0, 0.0],
        "shadeMultiplyTexture": null,
        "renderQueueOffsetNumber": 0.0,
        "shadingShiftFactor": 0.0,
        "shadingShiftTexture": null,
        "shadingToonyFactor": 0.9,
        "transparentWithZWrite": false,
        "uvAnimationMaskTexture": null,
        "uvAnimationRotationSpeedFactor": 0.0,
        "uvAnimationScrollXSpeedFactor": 0.0,
        "uvAnimationScrollYSpeedFactor": 0.0,
        "giEqualizationFactor": 0.9
    })
    .as_object_mut()?
    .clone();

    let floats = value
        .get("floatProperties")
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();
    if let Some(render_queue) = value.get("renderQueue").and_then(finite_f32) {
        properties.insert(
            "renderQueueOffsetNumber".into(),
            json!(render_queue - 2000.0),
        );
    }
    for (name, value) in &floats {
        let Some(value) = finite_f32(value) else {
            continue;
        };
        match name.as_str() {
            "_ShadeToony" | "_ShadingToonyRate" => {
                properties.insert("shadingToonyFactor".into(), json!(value));
            }
            "_ShadeShift" | "_ShadingShiftRate" => {
                properties.insert("shadingShiftFactor".into(), json!(value));
            }
            "_OutlineWidth" => {
                properties.insert("outlineWidthFactor".into(), json!(value.max(0.0)));
                if value > 0.0 {
                    properties.insert("outlineWidthMode".into(), json!("worldCoordinates"));
                }
            }
            "_OutlineLightingMix" => {
                properties.insert("outlineLightingMixFactor".into(), json!(value));
            }
            "_RimLightingMix" => {
                properties.insert("rimLightingMixFactor".into(), json!(value));
            }
            "_RimFresnelPower" => {
                properties.insert("parametricRimFresnelPowerFactor".into(), json!(value));
            }
            "_RimLift" => {
                properties.insert("parametricRimLiftFactor".into(), json!(value));
            }
            "_GIEqualizationFactor" => {
                properties.insert("giEqualizationFactor".into(), json!(value));
            }
            "_UvAnimRotation" | "_UV_Animation_RotationSpeed" => {
                properties.insert("uvAnimationRotationSpeedFactor".into(), json!(value));
            }
            "_UvAnimScrollX" | "_UV_Animation_ScrollX" => {
                properties.insert("uvAnimationScrollXSpeedFactor".into(), json!(value));
            }
            "_UvAnimScrollY" | "_UV_Animation_ScrollY" => {
                properties.insert("uvAnimationScrollYSpeedFactor".into(), json!(value));
            }
            "_ZWrite" => {
                properties.insert("transparentWithZWrite".into(), json!(value > 0.5));
            }
            "_ShadingShiftTextureScale" => {
                properties.insert("shadingShiftTextureScale".into(), json!(value));
            }
            "_RenderQueue" => {
                properties.insert("renderQueueOffsetNumber".into(), json!(value - 2000.0));
            }
            _ => {}
        }
    }

    let vectors = value
        .get("vectorProperties")
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();
    for (name, value) in vectors {
        let Some(vector) = finite_array(&value) else {
            continue;
        };
        match name.as_str() {
            "_ShadeColor" => {
                if let Some(color) = rgb(&vector) {
                    properties.insert("shadeColorFactor".into(), json!(color));
                }
            }
            "_RimColor" => {
                if let Some(color) = rgb(&vector) {
                    properties.insert("parametricRimColorFactor".into(), json!(color));
                }
            }
            "_OutlineColor" => {
                if let Some(color) = rgb(&vector) {
                    properties.insert("outlineColorFactor".into(), json!(color));
                }
            }
            "_MatcapColor" | "_MatCapColor" => {
                if let Some(color) = rgb(&vector) {
                    properties.insert("matcapFactor".into(), json!(color));
                }
            }
            _ => {}
        }
    }

    let textures = value
        .get("textureProperties")
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();
    for (name, value) in textures {
        let Some(index) = value.as_u64().and_then(|value| usize::try_from(value).ok()) else {
            continue;
        };
        let texture = texture_with_identity_transform(index);
        match name.as_str() {
            "_ShadeTexture" => {
                properties.insert("shadeMultiplyTexture".into(), texture);
            }
            "_RimTexture" | "_RimMultiplyTexture" => {
                properties.insert("rimMultiplyTexture".into(), texture);
            }
            "_MatcapTexture" | "_MatCapTex" => {
                properties.insert("matcapTexture".into(), json!({"index": index}));
            }
            "_ShadingShiftTexture" => {
                let scale = floats
                    .get("_ShadingShiftTextureScale")
                    .and_then(finite_f32)
                    .unwrap_or(1.0);
                properties.insert(
                    "shadingShiftTexture".into(),
                    json!({"index": index, "texCoord": 0.0, "scale": scale}),
                );
            }
            "_OutlineWidthTexture" => {
                properties.insert(
                    "outlineWidthMultiplyTexture".into(),
                    json!({"index": index}),
                );
            }
            "_UvAnimMaskTex" | "_UvAnimMaskTexture" => {
                properties.insert("uvAnimationMaskTexture".into(), json!({"index": index}));
            }
            _ => {}
        }
    }

    let mut converted: VrmcMaterialsExtensitions =
        serde_json::from_value(Value::Object(properties)).ok()?;
    converted.legacy_alpha_mode = legacy_alpha_mode(value);
    Some(converted)
}

fn legacy_alpha_mode(value: &Value) -> Option<LegacyAlphaMode> {
    let render_type = value
        .get("tagMap")
        .and_then(Value::as_object)
        .and_then(|tags| tags.get("RenderType"))
        .and_then(Value::as_str);
    let shader = value.get("shader").and_then(Value::as_str);
    let cutoff = value
        .get("floatProperties")
        .and_then(Value::as_object)
        .and_then(|floats| floats.get("_Cutoff"))
        .and_then(finite_f32)
        .unwrap_or(0.5)
        .clamp(0.0, 1.0);
    let name = render_type.or(shader)?;
    if name.contains("TransparentCutout") || name.contains("Cutout") {
        Some(LegacyAlphaMode::Mask(cutoff))
    } else if name.contains("Transparent") {
        Some(LegacyAlphaMode::Blend)
    } else if name.contains("Opaque") || name.contains("Texture") || name.contains("MToon") {
        Some(LegacyAlphaMode::Opaque)
    } else {
        None
    }
}

fn finite_f32(value: &Value) -> Option<f32> {
    value
        .as_f64()
        .map(|value| value as f32)
        .filter(|value| value.is_finite())
}

fn finite_array(value: &Value) -> Option<Vec<f32>> {
    value
        .as_array()
        .map(|values| values.iter().filter_map(finite_f32).collect())
}

fn rgb(vector: &[f32]) -> Option<[f32; 3]> {
    (vector.len() >= 3).then(|| [vector[0], vector[1], vector[2]])
}

fn texture_with_identity_transform(index: usize) -> Value {
    json!({
        "index": index,
        "extensions": {
            "KHR_texture_transform": {
                "offset": [0.0, 0.0],
                "scale": [1.0, 1.0]
            }
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn converts_legacy_mtoon_properties_to_existing_fields() {
        let value = json!({
            "name": "Body",
            "shader": "VRM/MToon",
            "renderQueue": 2500,
            "floatProperties": {
                "_ShadeToony": 0.7,
                "_OutlineWidth": 0.02,
                "_ZWrite": 1.0,
                "_GIEqualizationFactor": 0.8,
                "_Cutoff": 0.35
            },
            "vectorProperties": {
                "_ShadeColor": [0.1, 0.2, 0.3, 1.0],
                "_OutlineColor": [0.4, 0.5, 0.6, 1.0]
            },
            "textureProperties": {
                "_ShadeTexture": 3,
                "_MatCapTex": 4
            },
            "tagMap": {"RenderType": "TransparentCutout"}
        });
        let converted = convert_legacy_material_properties(&value).expect("material should parse");
        assert_eq!(converted.shading_toony_factor, 0.7);
        assert_eq!(converted.outline_width_factor, Some(0.02));
        assert_eq!(converted.render_queue_offset_number, 500.0);
        assert!(converted.transparent_with_z_write);
        assert_eq!(converted.shade_color_factor, [0.1, 0.2, 0.3]);
        assert_eq!(converted.outline_color_factor, [0.4, 0.5, 0.6]);
        assert_eq!(converted.shade_multiply_texture.unwrap().index, 3);
        assert_eq!(converted.matcap_texture.unwrap().index, 4);
        assert_eq!(
            converted.legacy_alpha_mode,
            Some(LegacyAlphaMode::Mask(0.35))
        );
    }

    #[test]
    fn rejects_non_finite_legacy_values_without_panicking() {
        let value = json!({
            "floatProperties": {"_ShadeToony": null},
            "vectorProperties": {"_ShadeColor": [0.1, null, 0.3]}
        });
        let converted = convert_legacy_material_properties(&value).expect("defaults should parse");
        assert_eq!(converted.shading_toony_factor, 0.9);
        assert_eq!(converted.shade_color_factor, [0.0, 0.0, 0.0]);
        assert_eq!(converted.legacy_alpha_mode, None);
    }
}

#[derive(Serialize, Deserialize, Reflect, Debug, Clone, Copy)]
pub struct MatcapTexture {
    pub index: usize,
}

#[derive(Serialize, Deserialize, Reflect, Debug, Clone, Copy)]
pub struct RimMultiplyTexture {
    pub index: usize,
}

#[derive(Serialize, Deserialize, Reflect, Debug, Clone, Copy)]
pub struct OutlineWidthMultiplyTexture {
    pub index: usize,
}

#[derive(Serialize, Deserialize, Reflect, Debug, Clone, Copy)]
pub struct UVAnimationMaskTexture {
    pub index: usize,
}

#[derive(Serialize, Deserialize, Reflect, Debug, Clone, Copy)]
pub struct ShadingShiftTexture {
    pub index: usize,
    #[serde(rename = "texCoord")]
    pub tex_coord: f32,
    pub scale: f32,
}

#[derive(Serialize, Deserialize, Reflect, Debug, Clone, Copy)]
pub struct VrmTexture {
    pub extensions: VrmTextureExtensions,
    pub index: usize,
}

#[derive(Serialize, Deserialize, Reflect, Debug, Clone, Copy)]
pub struct VrmTextureExtensions {
    #[serde(rename = "KHR_texture_transform")]
    pub khr_texture_transform: KhrTextureTransform,
}

#[allow(dead_code)]
#[derive(Serialize, Deserialize, Reflect, Debug, Clone, PartialEq, Copy, ShaderType)]
pub struct KhrTextureTransform {
    pub offset: [f32; 2],
    pub scale: [f32; 2],
}

impl Default for KhrTextureTransform {
    fn default() -> Self {
        Self {
            offset: [0.0, 0.0],
            scale: [1.0, 1.0],
        }
    }
}

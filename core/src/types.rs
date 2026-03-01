use serde::{Deserialize, Serialize};

/// 2D vector (x, y)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Vec2 {
    pub x: f32,
    pub y: f32,
}

/// 3D vector (x, y, z)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Vec3 {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

/// RGBA color (0.0..1.0)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Color {
    pub r: f32,
    pub g: f32,
    pub b: f32,
    pub a: f32,
}

/// Curve texture entry used in terrain rendering
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CurveTexture {
    pub texture_index: i32,
    pub size: Vec2,
    pub fixed_angle: bool,
    pub fade_threshold: f32,
}

/// Mesh data: vertices (2D) and triangle indices
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MeshData {
    pub vertices: Vec<Vec2>,
    pub triangles: Vec<i16>,
}

/// Terrain data attached to a prefab instance
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TerrainData {
    pub fill_tile_offset: Vec2,
    pub fill_mesh: MeshData,
    pub fill_color: Color,
    pub fill_texture_index: i32,
    pub curve_mesh: MeshData,
    pub curve_textures: Vec<CurveTexture>,
    /// Raw PNG bytes of the control texture, base64-encoded in serialized form
    #[serde(default, skip_serializing_if = "Option::is_none", with = "base64_opt")]
    pub control_texture_png: Option<Vec<u8>>,
    pub has_collider: bool,
}

/// Serde helper: serialize/deserialize `Option<Vec<u8>>` as base64 string.
mod base64_opt {
    use base64::{engine::general_purpose::STANDARD, Engine};
    use serde::{self, Deserialize, Deserializer, Serializer};

    pub fn serialize<S>(data: &Option<Vec<u8>>, s: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match data {
            Some(bytes) => s.serialize_str(&STANDARD.encode(bytes)),
            None => s.serialize_none(),
        }
    }

    pub fn deserialize<'de, D>(d: D) -> Result<Option<Vec<u8>>, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s: Option<String> = Option::deserialize(d)?;
        match s {
            Some(encoded) => STANDARD
                .decode(&encoded)
                .map(Some)
                .map_err(serde::de::Error::custom),
            None => Ok(None),
        }
    }
}

/// Additional data attached to a prefab instance
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ObjectData {
    /// No additional data
    None,
    /// Terrain mesh + texture data
    Terrain(TerrainData),
    /// UTF-8 text describing component property overrides
    PrefabOverrides {
        /// Raw text content (ObjectDeserializer format)
        text: String,
    },
}

/// A level object node (recursive tree)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind")]
pub enum LevelObject {
    /// An instance of a prefab from the prefab list
    PrefabInstance {
        name: String,
        prefab_index: i16,
        position: Vec3,
        rotation: Vec3,
        scale: Vec3,
        data: ObjectData,
    },
    /// A parent container with child objects
    Parent {
        name: String,
        position: Vec3,
        children: Vec<LevelObject>,
    },
}

/// Top-level level file structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LevelFile {
    pub object_count: i32,
    pub objects: Vec<LevelObject>,
}

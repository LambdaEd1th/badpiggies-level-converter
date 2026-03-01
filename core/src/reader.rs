use std::io::{self, Read};

use crate::types::*;

/// Errors that can occur during level parsing.
#[derive(Debug)]
pub enum ParseError {
    Io(io::Error),
    InvalidDataType(u8),
    InvalidString(std::string::FromUtf8Error),
}

impl From<io::Error> for ParseError {
    fn from(e: io::Error) -> Self {
        ParseError::Io(e)
    }
}

impl From<std::string::FromUtf8Error> for ParseError {
    fn from(e: std::string::FromUtf8Error) -> Self {
        ParseError::InvalidString(e)
    }
}

impl std::fmt::Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ParseError::Io(e) => write!(f, "IO error: {}", e),
            ParseError::InvalidDataType(t) => write!(f, "Invalid data type: {}", t),
            ParseError::InvalidString(e) => write!(f, "Invalid UTF-8 string: {}", e),
        }
    }
}

impl std::error::Error for ParseError {}

// ─── Helper readers (little-endian, matching C# BinaryReader) ───

fn read_i16(r: &mut impl Read) -> Result<i16, ParseError> {
    let mut buf = [0u8; 2];
    r.read_exact(&mut buf)?;
    Ok(i16::from_le_bytes(buf))
}

fn read_i32(r: &mut impl Read) -> Result<i32, ParseError> {
    let mut buf = [0u8; 4];
    r.read_exact(&mut buf)?;
    Ok(i32::from_le_bytes(buf))
}

fn read_u32(r: &mut impl Read) -> Result<u32, ParseError> {
    let mut buf = [0u8; 4];
    r.read_exact(&mut buf)?;
    Ok(u32::from_le_bytes(buf))
}

fn read_f32(r: &mut impl Read) -> Result<f32, ParseError> {
    let mut buf = [0u8; 4];
    r.read_exact(&mut buf)?;
    Ok(f32::from_le_bytes(buf))
}

fn read_u8(r: &mut impl Read) -> Result<u8, ParseError> {
    let mut buf = [0u8; 1];
    r.read_exact(&mut buf)?;
    Ok(buf[0])
}

fn read_bool(r: &mut impl Read) -> Result<bool, ParseError> {
    Ok(read_u8(r)? != 0)
}

/// Read a C# BinaryReader-compatible string.
///
/// C# BinaryReader.ReadString() uses a 7-bit encoded variable-length integer
/// for the byte length, followed by that many UTF-8 bytes.
fn read_csharp_string(r: &mut impl Read) -> Result<String, ParseError> {
    let len = read_7bit_encoded_int(r)?;
    let mut buf = vec![0u8; len as usize];
    r.read_exact(&mut buf)?;
    Ok(String::from_utf8(buf)?)
}

/// Read a 7-bit encoded integer (variable-length, used by C# BinaryReader).
///
/// Each byte contributes 7 bits; if the high bit is set, another byte follows.
fn read_7bit_encoded_int(r: &mut impl Read) -> Result<u32, ParseError> {
    let mut result: u32 = 0;
    let mut shift = 0u32;
    loop {
        let b = read_u8(r)?;
        result |= ((b & 0x7F) as u32) << shift;
        if b & 0x80 == 0 {
            break;
        }
        shift += 7;
        if shift >= 35 {
            return Err(ParseError::Io(io::Error::new(
                io::ErrorKind::InvalidData,
                "Bad 7-bit encoded int",
            )));
        }
    }
    Ok(result)
}

fn read_vec2(r: &mut impl Read) -> Result<Vec2, ParseError> {
    Ok(Vec2 {
        x: read_f32(r)?,
        y: read_f32(r)?,
    })
}

fn read_vec3(r: &mut impl Read) -> Result<Vec3, ParseError> {
    Ok(Vec3 {
        x: read_f32(r)?,
        y: read_f32(r)?,
        z: read_f32(r)?,
    })
}

fn read_color(r: &mut impl Read) -> Result<Color, ParseError> {
    let packed = read_u32(r)?;
    Ok(Color {
        r: ((packed >> 24) & 0xFF) as f32 / 255.0,
        g: ((packed >> 16) & 0xFF) as f32 / 255.0,
        b: ((packed >> 8) & 0xFF) as f32 / 255.0,
        a: (packed & 0xFF) as f32 / 255.0,
    })
}

// ─── Mesh reading ───

/// Read mesh vertex data (2D positions only) + triangle indices.
///
/// `is_fill_mesh`: if true, this is a fill mesh (vertices are plain Vec2).
/// If false, this is a curve mesh (vertices are Vec2 but z is set to -0.01 in C#,
/// we just store 2D here).
fn read_mesh(r: &mut impl Read) -> Result<MeshData, ParseError> {
    let vertex_count = read_i32(r)?;
    let mut vertices = Vec::with_capacity(vertex_count as usize);
    for _ in 0..vertex_count {
        vertices.push(read_vec2(r)?);
    }

    let triangle_count = read_i32(r)?;
    let mut triangles = Vec::with_capacity(triangle_count as usize);
    for _ in 0..triangle_count {
        triangles.push(read_i16(r)?);
    }

    Ok(MeshData {
        vertices,
        triangles,
    })
}

// ─── Data section ───

fn read_terrain(r: &mut impl Read) -> Result<TerrainData, ParseError> {
    let fill_tile_offset = Vec2 {
        x: read_f32(r)?,
        y: read_f32(r)?,
    };

    // Fill mesh
    let fill_mesh = read_mesh(r)?;
    let fill_color = read_color(r)?;
    let fill_texture_index = read_i32(r)?;

    // Curve mesh
    let curve_mesh = read_mesh(r)?;

    // Curve textures
    let curve_texture_count = read_i32(r)?;
    let mut curve_textures = Vec::with_capacity(curve_texture_count as usize);
    for _ in 0..curve_texture_count {
        let texture_index = read_i32(r)?;
        let size = read_vec2(r)?;
        let fixed_angle = read_bool(r)?;
        let fade_threshold = read_f32(r)?;
        curve_textures.push(CurveTexture {
            texture_index,
            size,
            fixed_angle,
            fade_threshold,
        });
    }

    // Control texture (embedded PNG)
    let has_control_texture = read_i32(r)?;
    let control_texture_png = if has_control_texture > 0 {
        let png_len = read_i32(r)? as usize;
        let mut png_data = vec![0u8; png_len];
        r.read_exact(&mut png_data)?;
        Some(png_data)
    } else {
        None
    };

    // Collider
    let has_collider = read_bool(r)?;

    Ok(TerrainData {
        fill_tile_offset,
        fill_mesh,
        fill_color,
        fill_texture_index,
        curve_mesh,
        curve_textures,
        control_texture_png,
        has_collider,
    })
}

fn read_prefab_overrides(r: &mut impl Read) -> Result<String, ParseError> {
    let byte_count = read_i32(r)? as usize;
    let mut buf = vec![0u8; byte_count];
    r.read_exact(&mut buf)?;
    Ok(String::from_utf8(buf)?)
}

fn read_data(r: &mut impl Read) -> Result<ObjectData, ParseError> {
    let data_type = read_u8(r)?;
    match data_type {
        0 => Ok(ObjectData::None),
        1 => Ok(ObjectData::Terrain(read_terrain(r)?)),
        2 => Ok(ObjectData::PrefabOverrides {
            text: read_prefab_overrides(r)?,
        }),
        other => Err(ParseError::InvalidDataType(other)),
    }
}

// ─── Object reading ───

fn read_prefab_instance(r: &mut impl Read) -> Result<LevelObject, ParseError> {
    let name = read_csharp_string(r)?;
    let prefab_index = read_i16(r)?;
    let position = read_vec3(r)?;
    let rotation = read_vec3(r)?;
    let scale = read_vec3(r)?;
    let data = read_data(r)?;

    Ok(LevelObject::PrefabInstance {
        name,
        prefab_index,
        position,
        rotation,
        scale,
        data,
    })
}

fn read_parent_object(r: &mut impl Read, child_count: i16) -> Result<LevelObject, ParseError> {
    let name = read_csharp_string(r)?;
    let position = read_vec3(r)?;

    let mut children = Vec::with_capacity(child_count as usize);
    for _ in 0..child_count {
        children.push(read_object(r)?);
    }

    Ok(LevelObject::Parent {
        name,
        position,
        children,
    })
}

fn read_object(r: &mut impl Read) -> Result<LevelObject, ParseError> {
    let child_count = read_i16(r)?;
    if child_count == 0 {
        read_prefab_instance(r)
    } else {
        read_parent_object(r, child_count)
    }
}

// ─── Top-level API ───

/// Parse a Bad Piggies level file from a reader.
pub fn read_level(r: &mut impl Read) -> Result<LevelFile, ParseError> {
    let object_count = read_i32(r)?;
    let mut objects = Vec::with_capacity(object_count as usize);
    for _ in 0..object_count {
        objects.push(read_object(r)?);
    }
    Ok(LevelFile {
        object_count,
        objects,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    /// Helper: write a 7-bit encoded integer into a buffer.
    fn write_7bit_int(buf: &mut Vec<u8>, mut value: u32) {
        loop {
            let mut b = (value & 0x7F) as u8;
            value >>= 7;
            if value > 0 {
                b |= 0x80;
            }
            buf.push(b);
            if value == 0 {
                break;
            }
        }
    }

    /// Helper: write a C#-compatible string (7-bit length + UTF-8 bytes).
    fn write_csharp_string(buf: &mut Vec<u8>, s: &str) {
        let bytes = s.as_bytes();
        write_7bit_int(buf, bytes.len() as u32);
        buf.extend_from_slice(bytes);
    }

    #[test]
    fn test_7bit_encoded_int() {
        // 0 → [0x00]
        let data = vec![0x00];
        let val = read_7bit_encoded_int(&mut Cursor::new(data)).unwrap();
        assert_eq!(val, 0);

        // 127 → [0x7F]
        let data = vec![0x7F];
        let val = read_7bit_encoded_int(&mut Cursor::new(data)).unwrap();
        assert_eq!(val, 127);

        // 128 → [0x80, 0x01]
        let data = vec![0x80, 0x01];
        let val = read_7bit_encoded_int(&mut Cursor::new(data)).unwrap();
        assert_eq!(val, 128);

        // 300 → [0xAC, 0x02]
        let data = vec![0xAC, 0x02];
        let val = read_7bit_encoded_int(&mut Cursor::new(data)).unwrap();
        assert_eq!(val, 300);
    }

    #[test]
    fn test_csharp_string() {
        let mut buf = Vec::new();
        write_csharp_string(&mut buf, "Hello");
        let result = read_csharp_string(&mut Cursor::new(buf)).unwrap();
        assert_eq!(result, "Hello");
    }

    #[test]
    fn test_empty_level() {
        // A level with 0 objects
        let data: Vec<u8> = vec![0, 0, 0, 0]; // i32 LE = 0
        let level = read_level(&mut Cursor::new(data)).unwrap();
        assert_eq!(level.object_count, 0);
        assert!(level.objects.is_empty());
    }

    #[test]
    fn test_single_prefab_instance() {
        let mut buf = Vec::new();

        // object_count = 1
        buf.extend_from_slice(&1_i32.to_le_bytes());

        // child_count = 0 (prefab instance)
        buf.extend_from_slice(&0_i16.to_le_bytes());

        // name = "TestPrefab"
        write_csharp_string(&mut buf, "TestPrefab");

        // prefab_index = 3
        buf.extend_from_slice(&3_i16.to_le_bytes());

        // position (1.0, 2.0, 3.0)
        buf.extend_from_slice(&1.0_f32.to_le_bytes());
        buf.extend_from_slice(&2.0_f32.to_le_bytes());
        buf.extend_from_slice(&3.0_f32.to_le_bytes());

        // rotation (0.0, 0.0, 45.0)
        buf.extend_from_slice(&0.0_f32.to_le_bytes());
        buf.extend_from_slice(&0.0_f32.to_le_bytes());
        buf.extend_from_slice(&45.0_f32.to_le_bytes());

        // scale (1.0, 1.0, 1.0)
        buf.extend_from_slice(&1.0_f32.to_le_bytes());
        buf.extend_from_slice(&1.0_f32.to_le_bytes());
        buf.extend_from_slice(&1.0_f32.to_le_bytes());

        // data type = None (0)
        buf.push(0);

        let level = read_level(&mut Cursor::new(buf)).unwrap();
        assert_eq!(level.object_count, 1);
        match &level.objects[0] {
            LevelObject::PrefabInstance {
                name,
                prefab_index,
                position,
                rotation,
                scale,
                data,
            } => {
                assert_eq!(name, "TestPrefab");
                assert_eq!(*prefab_index, 3);
                assert!((position.x - 1.0).abs() < f32::EPSILON);
                assert!((position.y - 2.0).abs() < f32::EPSILON);
                assert!((position.z - 3.0).abs() < f32::EPSILON);
                assert!((rotation.z - 45.0).abs() < f32::EPSILON);
                assert!((scale.x - 1.0).abs() < f32::EPSILON);
                assert!(matches!(data, ObjectData::None));
            }
            _ => panic!("Expected PrefabInstance"),
        }
    }

    #[test]
    fn test_parent_with_children() {
        let mut buf = Vec::new();

        // object_count = 1 (one top-level parent)
        buf.extend_from_slice(&1_i32.to_le_bytes());

        // child_count = 2 (parent with 2 children)
        buf.extend_from_slice(&2_i16.to_le_bytes());

        // parent name
        write_csharp_string(&mut buf, "ParentObj");

        // parent position
        buf.extend_from_slice(&0.0_f32.to_le_bytes());
        buf.extend_from_slice(&5.0_f32.to_le_bytes());
        buf.extend_from_slice(&0.0_f32.to_le_bytes());

        // --- Child 1: prefab instance ---
        buf.extend_from_slice(&0_i16.to_le_bytes()); // childCount = 0
        write_csharp_string(&mut buf, "Child1");
        buf.extend_from_slice(&0_i16.to_le_bytes()); // prefab_index
                                                     // position, rotation, scale (all zeros)
        for _ in 0..9 {
            buf.extend_from_slice(&0.0_f32.to_le_bytes());
        }
        buf.push(0); // data type = None

        // --- Child 2: prefab instance ---
        buf.extend_from_slice(&0_i16.to_le_bytes());
        write_csharp_string(&mut buf, "Child2");
        buf.extend_from_slice(&1_i16.to_le_bytes());
        for _ in 0..9 {
            buf.extend_from_slice(&0.0_f32.to_le_bytes());
        }
        buf.push(0);

        let level = read_level(&mut Cursor::new(buf)).unwrap();
        assert_eq!(level.object_count, 1);
        match &level.objects[0] {
            LevelObject::Parent {
                name,
                position,
                children,
            } => {
                assert_eq!(name, "ParentObj");
                assert!((position.y - 5.0).abs() < f32::EPSILON);
                assert_eq!(children.len(), 2);
            }
            _ => panic!("Expected Parent"),
        }
    }

    #[test]
    fn test_prefab_overrides_data() {
        let mut buf = Vec::new();

        // object_count = 1
        buf.extend_from_slice(&1_i32.to_le_bytes());

        // child_count = 0 (prefab instance)
        buf.extend_from_slice(&0_i16.to_le_bytes());

        write_csharp_string(&mut buf, "OverrideObj");
        buf.extend_from_slice(&0_i16.to_le_bytes()); // prefab_index

        // position, rotation, scale
        for _ in 0..9 {
            buf.extend_from_slice(&0.0_f32.to_le_bytes());
        }

        // data type = PrefabOverrides (2)
        buf.push(2);

        // Override text data
        let override_text =
            "GameObject TestObj\n  Component LevelManager\n    Integer m_gridWidth 10\n";
        let text_bytes = override_text.as_bytes();
        buf.extend_from_slice(&(text_bytes.len() as i32).to_le_bytes());
        buf.extend_from_slice(text_bytes);

        let level = read_level(&mut Cursor::new(buf)).unwrap();
        match &level.objects[0] {
            LevelObject::PrefabInstance { data, .. } => match data {
                ObjectData::PrefabOverrides { text } => {
                    assert!(text.contains("m_gridWidth"));
                }
                _ => panic!("Expected PrefabOverrides"),
            },
            _ => panic!("Expected PrefabInstance"),
        }
    }
}

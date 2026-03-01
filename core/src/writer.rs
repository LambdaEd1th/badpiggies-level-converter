use std::io::{self, Write};

use crate::types::*;

/// Write errors
#[derive(Debug)]
pub enum WriteError {
    Io(io::Error),
}

impl From<io::Error> for WriteError {
    fn from(e: io::Error) -> Self {
        WriteError::Io(e)
    }
}

impl std::fmt::Display for WriteError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            WriteError::Io(e) => write!(f, "IO error: {}", e),
        }
    }
}

impl std::error::Error for WriteError {}

// ─── Helper writers (little-endian, matching C# BinaryWriter) ───

fn write_i16(w: &mut impl Write, v: i16) -> Result<(), WriteError> {
    w.write_all(&v.to_le_bytes())?;
    Ok(())
}

fn write_i32(w: &mut impl Write, v: i32) -> Result<(), WriteError> {
    w.write_all(&v.to_le_bytes())?;
    Ok(())
}

fn write_u32(w: &mut impl Write, v: u32) -> Result<(), WriteError> {
    w.write_all(&v.to_le_bytes())?;
    Ok(())
}

fn write_f32(w: &mut impl Write, v: f32) -> Result<(), WriteError> {
    w.write_all(&v.to_le_bytes())?;
    Ok(())
}

fn write_u8(w: &mut impl Write, v: u8) -> Result<(), WriteError> {
    w.write_all(&[v])?;
    Ok(())
}

fn write_bool(w: &mut impl Write, v: bool) -> Result<(), WriteError> {
    write_u8(w, if v { 1 } else { 0 })
}

/// Write a 7-bit encoded integer (matching C# BinaryWriter).
fn write_7bit_encoded_int(w: &mut impl Write, mut value: u32) -> Result<(), WriteError> {
    loop {
        let mut b = (value & 0x7F) as u8;
        value >>= 7;
        if value > 0 {
            b |= 0x80;
        }
        write_u8(w, b)?;
        if value == 0 {
            break;
        }
    }
    Ok(())
}

/// Write a C# BinaryWriter-compatible string (7-bit length prefix + UTF-8 bytes).
fn write_csharp_string(w: &mut impl Write, s: &str) -> Result<(), WriteError> {
    let bytes = s.as_bytes();
    write_7bit_encoded_int(w, bytes.len() as u32)?;
    w.write_all(bytes)?;
    Ok(())
}

fn write_vec2(w: &mut impl Write, v: &Vec2) -> Result<(), WriteError> {
    write_f32(w, v.x)?;
    write_f32(w, v.y)?;
    Ok(())
}

fn write_vec3(w: &mut impl Write, v: &Vec3) -> Result<(), WriteError> {
    write_f32(w, v.x)?;
    write_f32(w, v.y)?;
    write_f32(w, v.z)?;
    Ok(())
}

fn write_color(w: &mut impl Write, c: &Color) -> Result<(), WriteError> {
    let r = (c.r * 255.0).round() as u32 & 0xFF;
    let g = (c.g * 255.0).round() as u32 & 0xFF;
    let b = (c.b * 255.0).round() as u32 & 0xFF;
    let a = (c.a * 255.0).round() as u32 & 0xFF;
    let packed = (r << 24) | (g << 16) | (b << 8) | a;
    write_u32(w, packed)
}

// ─── Mesh writing ───

fn write_mesh(w: &mut impl Write, mesh: &MeshData) -> Result<(), WriteError> {
    write_i32(w, mesh.vertices.len() as i32)?;
    for v in &mesh.vertices {
        write_vec2(w, v)?;
    }
    write_i32(w, mesh.triangles.len() as i32)?;
    for &t in &mesh.triangles {
        write_i16(w, t)?;
    }
    Ok(())
}

// ─── Data section ───

fn write_terrain(w: &mut impl Write, terrain: &TerrainData) -> Result<(), WriteError> {
    write_f32(w, terrain.fill_tile_offset.x)?;
    write_f32(w, terrain.fill_tile_offset.y)?;

    // Fill mesh
    write_mesh(w, &terrain.fill_mesh)?;
    write_color(w, &terrain.fill_color)?;
    write_i32(w, terrain.fill_texture_index)?;

    // Curve mesh
    write_mesh(w, &terrain.curve_mesh)?;

    // Curve textures
    write_i32(w, terrain.curve_textures.len() as i32)?;
    for ct in &terrain.curve_textures {
        write_i32(w, ct.texture_index)?;
        write_vec2(w, &ct.size)?;
        write_bool(w, ct.fixed_angle)?;
        write_f32(w, ct.fade_threshold)?;
    }

    // Control texture
    match &terrain.control_texture_png {
        Some(png_data) => {
            write_i32(w, 1)?; // has control texture
            write_i32(w, png_data.len() as i32)?;
            w.write_all(png_data)?;
        }
        None => {
            write_i32(w, 0)?; // no control texture
        }
    }

    // Collider
    write_bool(w, terrain.has_collider)?;

    Ok(())
}

fn write_prefab_overrides(w: &mut impl Write, text: &str) -> Result<(), WriteError> {
    let bytes = text.as_bytes();
    write_i32(w, bytes.len() as i32)?;
    w.write_all(bytes)?;
    Ok(())
}

fn write_data(w: &mut impl Write, data: &ObjectData) -> Result<(), WriteError> {
    match data {
        ObjectData::None => {
            write_u8(w, 0)?;
        }
        ObjectData::Terrain(terrain) => {
            write_u8(w, 1)?;
            write_terrain(w, terrain)?;
        }
        ObjectData::PrefabOverrides { text } => {
            write_u8(w, 2)?;
            write_prefab_overrides(w, text)?;
        }
    }
    Ok(())
}

// ─── Object writing ───

fn write_object(w: &mut impl Write, obj: &LevelObject) -> Result<(), WriteError> {
    match obj {
        LevelObject::PrefabInstance {
            name,
            prefab_index,
            position,
            rotation,
            scale,
            data,
        } => {
            write_i16(w, 0)?; // child_count = 0 → prefab instance
            write_csharp_string(w, name)?;
            write_i16(w, *prefab_index)?;
            write_vec3(w, position)?;
            write_vec3(w, rotation)?;
            write_vec3(w, scale)?;
            write_data(w, data)?;
        }
        LevelObject::Parent {
            name,
            position,
            children,
        } => {
            write_i16(w, children.len() as i16)?;
            write_csharp_string(w, name)?;
            write_vec3(w, position)?;
            for child in children {
                write_object(w, child)?;
            }
        }
    }
    Ok(())
}

// ─── Top-level API ───

/// Write a Bad Piggies level file to a writer.
pub fn write_level(w: &mut impl Write, level: &LevelFile) -> Result<(), WriteError> {
    write_i32(w, level.objects.len() as i32)?;
    for obj in &level.objects {
        write_object(w, obj)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::reader;
    use std::io::Cursor;

    #[test]
    fn test_round_trip_empty() {
        let level = LevelFile {
            object_count: 0,
            objects: vec![],
        };

        let mut buf = Vec::new();
        write_level(&mut buf, &level).unwrap();

        let parsed = reader::read_level(&mut Cursor::new(&buf)).unwrap();
        assert_eq!(parsed.object_count, 0);
        assert!(parsed.objects.is_empty());
    }

    #[test]
    fn test_round_trip_prefab_instance() {
        let level = LevelFile {
            object_count: 1,
            objects: vec![LevelObject::PrefabInstance {
                name: "TestObj".to_string(),
                prefab_index: 5,
                position: Vec3 {
                    x: 1.5,
                    y: -2.0,
                    z: 3.0,
                },
                rotation: Vec3 {
                    x: 0.0,
                    y: 0.0,
                    z: 45.0,
                },
                scale: Vec3 {
                    x: 1.0,
                    y: 1.0,
                    z: 1.0,
                },
                data: ObjectData::None,
            }],
        };

        let mut buf = Vec::new();
        write_level(&mut buf, &level).unwrap();

        let parsed = reader::read_level(&mut Cursor::new(&buf)).unwrap();
        assert_eq!(parsed.object_count, 1);
        match &parsed.objects[0] {
            LevelObject::PrefabInstance {
                name,
                prefab_index,
                position,
                rotation,
                ..
            } => {
                assert_eq!(name, "TestObj");
                assert_eq!(*prefab_index, 5);
                assert!((position.x - 1.5).abs() < f32::EPSILON);
                assert!((position.y - (-2.0)).abs() < f32::EPSILON);
                assert!((rotation.z - 45.0).abs() < f32::EPSILON);
            }
            _ => panic!("Expected PrefabInstance"),
        }
    }

    #[test]
    fn test_round_trip_parent() {
        let level = LevelFile {
            object_count: 1,
            objects: vec![LevelObject::Parent {
                name: "Container".to_string(),
                position: Vec3 {
                    x: 0.0,
                    y: 5.0,
                    z: 0.0,
                },
                children: vec![
                    LevelObject::PrefabInstance {
                        name: "A".to_string(),
                        prefab_index: 0,
                        position: Vec3 {
                            x: 1.0,
                            y: 0.0,
                            z: 0.0,
                        },
                        rotation: Vec3 {
                            x: 0.0,
                            y: 0.0,
                            z: 0.0,
                        },
                        scale: Vec3 {
                            x: 1.0,
                            y: 1.0,
                            z: 1.0,
                        },
                        data: ObjectData::None,
                    },
                    LevelObject::PrefabInstance {
                        name: "B".to_string(),
                        prefab_index: 1,
                        position: Vec3 {
                            x: 2.0,
                            y: 0.0,
                            z: 0.0,
                        },
                        rotation: Vec3 {
                            x: 0.0,
                            y: 0.0,
                            z: 0.0,
                        },
                        scale: Vec3 {
                            x: 1.0,
                            y: 1.0,
                            z: 1.0,
                        },
                        data: ObjectData::PrefabOverrides {
                            text: "test data".to_string(),
                        },
                    },
                ],
            }],
        };

        let mut buf = Vec::new();
        write_level(&mut buf, &level).unwrap();

        let parsed = reader::read_level(&mut Cursor::new(&buf)).unwrap();
        assert_eq!(parsed.object_count, 1);
        match &parsed.objects[0] {
            LevelObject::Parent { name, children, .. } => {
                assert_eq!(name, "Container");
                assert_eq!(children.len(), 2);
                match &children[1] {
                    LevelObject::PrefabInstance { name, data, .. } => {
                        assert_eq!(name, "B");
                        match data {
                            ObjectData::PrefabOverrides { text } => {
                                assert_eq!(text, "test data");
                            }
                            _ => panic!("Expected PrefabOverrides"),
                        }
                    }
                    _ => panic!("Expected PrefabInstance"),
                }
            }
            _ => panic!("Expected Parent"),
        }
    }

    /// Round-trip test: read a real file, write it, compare bytes.
    #[test]
    fn test_round_trip_real_file() {
        let test_file = "bytes_files/Level_01_data.bytes";
        if !std::path::Path::new(test_file).exists() {
            eprintln!("Skipping real file round-trip test (file not found)");
            return;
        }

        let original_bytes = std::fs::read(test_file).unwrap();
        let level = reader::read_level(&mut Cursor::new(&original_bytes)).unwrap();

        let mut output = Vec::new();
        write_level(&mut output, &level).unwrap();

        assert_eq!(
            original_bytes.len(),
            output.len(),
            "Output size mismatch: expected {} got {}",
            original_bytes.len(),
            output.len()
        );
        assert_eq!(
            original_bytes, output,
            "Round-trip produced different bytes"
        );
    }
}

# Bad Piggies Level Converter

A fast and robust command-line tool written in Rust for unpacking and packing Bad Piggies binary level files (`.bytes`).

## Overview

Bad Piggies stores its level data in a proprietary binary format (`.bytes`). This tool enables you to convert these binary files into human-readable YAML for easy editing, modding, or analysis, and then pack your modified YAML back into game-ready `.bytes` files. 

### Features

- **Perfect Round-Trip Accuracy**: The converter was tested against all 28 original game levels. A `.bytes` file unpacked to YAML and packed back is functionally identical to the original.
- **Human-Readable YAML Output**: Instead of dealing with unreadable binary data, you get structured YAML.
- **Base64 Texture Embeds**: Native `base64` embedding of PNG control textures within the YAML to preserve image integrity across conversions.
- **Cross-Platform**: Small, compiled standalone binary for Windows, macOS, and Linux without external runtime dependencies like Python or Node.js.
- **Clean Workspace Architecture**: Built as a standard Cargo Workspace containing a pure binary conversion crate (`core`) and a CLI wrapper crate (`cli`).

## Installation

You must have [Rust and Cargo](https://rustup.rs/) installed. Clone the repository and build:

```bash
git clone https://github.com/LambdaEd1th/badpiggies-level-converter.git
cd badpiggies-level-converter
cargo build --release
```

The compiled binary will be located at `target/release/bad-piggies-level`.

## Usage

### Unpack `.bytes` to `YAML`
Converts a binary level file into a structured YAML format.

```bash
bad-piggies-level unpack path/to/Level_01_data.bytes -o Level_01_data.yaml
```

If the `-o` (or `--output`) option is omitted, the YAML will be printed directly to the console (`stdout`).

### Pack `YAML` to `.bytes`
Converts your modified YAML file back into a binary level file format ready to be loaded by the game.

```bash
bad-piggies-level pack path/to/Level_01_data.yaml -o Level_01_data.bytes
```

If `-o` is omitted, the tool will automatically output a file with the same name but with a `.bytes` extension in the same directory as the input YAML file.

## YAML Format Details

The YAML file maps 1:1 with the game's internal data structures. Key elements include:

- `object_count`: The total number of top-level game objects.
- `objects`: An array of `LevelObject`.
  - `kind`: The object type (`PrefabInstance`, `Parent`).
  - `name`: The object's string name (e.g., `CameraSystem`, `LevelStart`).
  - `prefab_index`: Internal prefab reference ID.
  - `position`, `rotation` (Euler angles in degrees), `scale`.
  - `children`: Nested arrays of `LevelObject` for parent-child hierarchies.
  - `data`: Attached component data. Available types matching the game's serialization:
    - `None`: No attached data.
    - `PrefabOverrides`: C#-style object graph property overrides (as raw multi-line strings).
    - `Terrain`: Spline-based 2D terrain data (e2dTerrain), containing mesh vertices, triangles, edge textures, UVs, and base64-encoded PNG control masks.

## Crate Architecture 

- `core/`: Stand-alone, UI-agnostic library containing zero-copy parsing techniques for the binary structure. Serialization is handled through `serde`.
- `cli/`: Command-line executable wrapper parsing arguments with `clap` and rendering output with `serde_yaml`.

## License

This project is open-source and available under the terms of the GNU General Public License v3.0 (GPLv3). See the [LICENSE](LICENSE) file for more details.

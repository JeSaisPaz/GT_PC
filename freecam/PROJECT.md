# GT PSP Freecam Tool — Project Planning

## Overview
Build a standalone tool to open GT PSP tracks and navigate them with a freecam, rendered using Vulkan.

## Track Data Sources

### Extracted Files Location
```
GT.VOL extraction (via GTPSPVolTools):
├── crs/
│   ├── c001.ad     # Course metadata (spawn position, checkpoints)
│   ├── c001x       # Full course 3D model (PACL format, 3-7MB)
│   ├── c002.ad
│   ├── c002x
│   └── ... (109 tracks total)
│   ├── race.mdl    # Low-detail track mesh (~28KB, 199v/28tri)
│   └── race.txs    # Course texture (TXS3 format)
```

### Course Metadata (.ad files)
- **Purpose**: Spawn position, checkpoints, track metadata
- **Format**: Binary with record types
- **Key fields**:
  - Record type `1`: Contains x, z spawn coordinates at offsets +4, +8
  - Checkpoints stored as (x, z) pairs
- **Reference**: `pc_port/src/engine/model.rs:730` — `parse_course_metadata_full()`

### Course Models
- **Low-detail**: `race.mdl` — 3LDM format, ~199 vertices (debug/test mesh)
- **Full-detail**: `cXXXx` files — PACL format (ModelSet3), 3-7MB per track
- **Format**: 3LDM (little-endian variant of MDL3)
  - Magic: `3LDM` (0x33, 0x4C, 0x44, 0x4D)
  - Version: ModelSet3 v2 for GTPSP
  - Contains: vertices, triangle strips, UVs, normals

### Textures
- **Format**: TXS3 (Texture Set 3)
- **Course texture**: `crs/race.txs` — standalone 3SXT format
- **Embedded in models**: TXS3 texture set pointer in 3LDM header at offset 0x48

### Course List
- **Location**: `textdata/gt5m/courselist.xml`
- **Maps IDs to names**:
  ```xml
  <courselist>
    <course id="c001" name="London" />
    <course id="c002" name="Fuji Speedway" />
    ...
  </courselist>
  ```

## 3LDM Format Reference

### Header (0xE4 bytes)
| Field | Offset | Type | Notes |
|-------|--------|------|-------|
| Magic | 0x00 | u32 | 0x334C444D (3LDM) |
| File Size | 0x04 | u32 | |
| Model Count | 0x10 | u16 | |
| Shape Count | 0x14 | u16 | Mesh count |
| FVF Count | 0x18 | u16 | Flexible Vertex Format count |
| Meshes Pointer | 0x38 | u32 ptr | → Mesh array |
| FVF Pointer | 0x40 | u32 ptr | → FVF array |
| Texture Set Pointer | 0x48 | u32 ptr | → Embedded TXS3 |

### FVF (Vertex Layout)
- **Semantic codes**: 0=position, 1=normal, 2=color, 3=UV/map, 4=tangent, 5=binormal
- **Type codes**: 0=f32[2], 1=f32[3], 2=f32[4], 3=i16[2], 4=i16[4], 5=u8[4], 6=i16n[2], 7=i16n[4], 8=u8n[4]
- **Common car vertex**: position(f32[3]) + normal(i16n[4]) + uv(i16[2]) = 24 bytes/vertex

### Mesh Entry (0x30 bytes)
| Field | Offset | Type | Notes |
|-------|--------|------|-------|
| FVF Index | 0x02 | i16 | |
| Vertex Count | 0x08 | u32 | |
| Vertex Pointer | 0x0C | u32 ptr | → Raw vertex buffer |
| Tri Length | 0x14 | u32 | Index buffer byte size |
| Tri Pointer | 0x18 | u32 ptr | → i16 strip indices |
| Tri Count | 0x26 | i16 | Index count |

### Triangle Strip De-stripping
- Index buffer uses **triangle strips** with i16 indices
- Negative values = strip restart markers
- Winding alternates per triangle in strip

```python
def destrip(indices):
    faces = []
    strip = []
    for idx in indices:
        if idx < 0:
            strip = []
        else:
            strip.append(idx)
            if len(strip) >= 3:
                n = len(strip)
                a, b, c = strip[-3], strip[-2], strip[-1]
                if (n - 3) % 2 == 0:
                    faces.append((a, b, c))
                else:
                    faces.append((a, c, b))
    return faces
```

## Existing Code to Reference

### pc_port/src/engine/model.rs
- `load_course(course_id)` — loads track mesh from cXXXx + metadata from .ad
- `parse_course_metadata_full(data)` — extracts spawn position
- `parse_3ldm_mesh(data)` — parses 3LDM format
- `find_mesh_blocks(data)` — heuristic scanner for GT PSP variant (null mesh/FVF pointers)
- `load_car_model()` — car model loading
- `load_course_texture()` — TXS3 texture parsing
- `load_camera()` — .cam file loading

### pc_port/src/engine/sprite.rs
- `parse_txs3_texture()` — TXS3/3SXT texture parser

## Tools & Dependencies

### Required Tools
1. **GTPSPVolTools** — Extract/repack GT.VOL archives
   - `workflow/GTPSPVolTools/GTPSPVolTools.exe`
   - Usage: `GTPSPVolTools.exe unpack -i GT.VOL -o output_folder`

2. **PDTools** (C#) — Reference implementation for ModelSet3 parsing
   - https://github.com/Nenkai/PDTools

3. **GT-File-Specifications-Documentation** — 010 Editor templates
   - https://github.com/Nenkai/GT-File-Specifications-Documentation

### SpecDB Tools
- **GTSpecDB** — Edit course information, car specs, race events
  - https://github.com/Nenkai/GTSpecDB

## Rendering Approach

### Option 1: Extend PC Port
- Uses SDL2 + OpenGL currently
- Convert to Vulkan for better performance
- Benefits: Existing course loading code, physics, track following
- Drawback: Not a clean separation

### Option 2: Fresh Implementation
- New Rust/C++ project with Vulkan
- Reuse model.rs parsing logic (copy/adapt)
- Full freecam controls (WASD + mouse look)
- Benefits: Clean architecture, Vulkan-native
- Drawback: Need to reimplement parsing

### Recommended: Start from PC Port
1. Copy relevant code from pc_port (model.rs parsing)
2. Replace SDL2/GL with Vulkan (vulkano or wgpu in Rust)
3. Add freecam controls (detach from car-following camera)
4. Keep course loading pipeline

## Freecam Features
- WASD movement (forward/back/strafe)
- Mouse look (pitch/yaw)
- Shift/Space for up/down
- Speed control (slow/fast mode)
- Track selection UI
- Display current position (debug)

## Track List (109 courses)
See `textdata/gt5m/courselist.xml` for ID → name mapping.
- c001: London
- c002: Fuji Speedway
- c003: Trial Mountain
- ... (total 109)

## External References

| Resource | URL |
|----------|-----|
| GT Modding Hub | https://nenkai.github.io/gt-modding-hub/ |
| MDL3 Format Docs | https://nenkai.github.io/gt-modding-hub/formats/models/mdl3_modelset3/ |
| PDTools | https://github.com/Nenkai/PDTools |
| GT PSP Vol Tools | https://github.com/Nenkai/GTPSPVolTools |
| GTSpecDB | https://github.com/Nenkai/GTSpecDB |

## Completed ✓
1. ✅ **Extract track data** using GTPSPVolTools — 109 tracks available in `assets/game/crs/`
2. ✅ **Review 3LDM format** fully documented above
3. ✅ **Set up Vulkan project** — Rust + wgpu 29 + winit 0.30, modern Vulkan backend
4. ✅ **Implement 3LDM parser** — heuristic vertex scanning + FVF-based mesh parsing + triangle strip de-stripping
5. ✅ **Implement TXS3 texture parser** — supports DXT1 (BC1), DXT5 (BC3), and RGBA8888 course textures
6. ✅ **Build Vulkan rendering pipeline** — wgpu shader with lighting, depth testing, texture sampling; fallback grid shader
7. ✅ **Add freecam controls** — WASD + mouse look (click to lock, Esc to release), Shift/Space up/down, Ctrl fast mode
8. ⬜ **Build track selector UI** — currently hardcoded to track c114, needs course list browser
9. ⬜ **Test with sample tracks** — geometry parser is heuristic and may need tuning per track

## Quick Start
```powershell
# Build (debug)
cargo build
# Build (release with LTO)
cargo build --release

# Run
.\target\release\gt-freecam.exe
```

## Controls
| Input | Action |
|-------|--------|
| Left Click | Lock mouse (capture cursor) |
| Escape | Release mouse |
| W/S | Forward/Backward |
| A/D | Strafe left/right |
| Space | Move up |
| Shift | Move down |
| Control | Fast mode (3x speed) |
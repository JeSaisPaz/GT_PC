---
tags: [pc-port, rust, model, parser]
type: documentation
project: GT PSP PC Port
section: Core Engine
---

# 3D Model Parser — PC Port

> 3D model parsing in Rust (`pc_port/src/engine/model.rs`).

## Overview

Parses GT PSP's 3LDM (ModelSet3 little-endian) format.

## Data Structures

```rust
pub struct CarModel {
    pub vertices: Vec<(f32, f32, f32)>,
    pub triangles: Vec<(u32, u32, u32)>,
    pub normals: Vec<(f32, f32, f32)>,
    pub uvs: Vec<(f32, f32)>,  // Texture coordinates
    pub has_uvs: bool,
    pub center: (f32, f32, f32),
}

pub struct TrackState {
    pub vertices: Vec<(f32, f32, f32)>,
    pub triangles: Vec<(u32, u32, u32)>,
    pub normals: Vec<(f32, f32, f32)>,
    pub uvs: Vec<(f32, f32)>,  // Texture coordinates
    pub has_uvs: bool,
    pub center: (f32, f32, f32),
    pub course_loaded: bool,
    pub car_loaded: bool,
    pub models_loaded: u32,
}
```

## 3LDM Header Parsing

```rust
struct ModelSet3Header {
    file_size: u32,
    model_count: u16,
    shape_count: u16,
    fvf_count: u16,
    bones_count: u16,
    models_ptr: u32,
    meshes_ptr: u32,
    fvf_ptr: u32,
    materials_ptr: u32,
    bones_ptr: u32,
    texture_set_ptr: u32,
}

fn read_3ldm_header(data: &[u8]) -> Result<ModelSet3Header, String>
```

Validates:
- File length ≥ 0xE4
- Magic == `3LDM` (0x334C444D)

## Model Entry

```rust
struct ModelEntry {
    origin: (f32, f32, f32),
    bounds_count: u16,
    bounds_ptr: u32,
    setup_cmds_ptr: u32,
    setup_cmds_size: u32,
}
```

## GT PSP Variant Handling

GT PSP uses null mesh/FVF pointers. Geometry stored in:
1. Model setup commands (VM opcodes)
2. Scattered vertex buffers

### All-Runs Heuristic Scanner

```rust
fn extract_geometry_from_models(data: &[u8])
    -> Result<(vertices, triangles, normals), String>
```

Algorithm:
1. Read header → get model count + pointer
2. For each model:
   - Skip if `setup_cmds_size == 0`
   - Scan for valid vertex runs (stride 20 = 12 pos + 8 normal)
   - Find runs with ≥8 consecutive valid vertices
3. De-strip triangle indices:
   - Positive = vertex reference
   - Negative = strip restart
   - Winding alternates

## Vertex Validation

```rust
fn is_valid_vertex(x: f32, y: f32, z: f32) -> bool {
    if !x.is_finite() || !y.is_finite() || !z.is_finite() { return false; }
    if x.abs() > 100000.0 || y.abs() > 100000.0 || z.abs() > 100000.0 { return false; }
    if x == 0.0 && y == 0.0 && z == 0.0 { return false; }
    let mag = (x*x + y*y + z*z).sqrt();
    mag > 0.001
}
```

Filters: NaN, Inf, overflows, zero vectors.

## UV Coordinate Extraction ✅ (2026-04-29)

Texture coordinates extracted from FVF (Flexible Vertex Definition) when available:

```rust
// FVF Semantic 3 = UV/map coordinates
let uv_field = fields.iter().find(|f| f.semantic == 3);
let uv_offset = uv_field.map(|f| f.offset);
let uv_type = uv_field.map(|f| f.data_type).unwrap_or(0);

// Read UV per vertex
if let Some(off) = uv_offset {
    if let Some(uv) = read_vertex_uv(data, mesh.vertex_ptr, v, 
                                      fvf.vertex_stride, off, uv_type) {
        all_uvs.push(uv);
        has_uvs = true;
    }
}
```

**UV Data Types:**
- Type 0: `f32[2]` (8 bytes) - float UVs
- Type 3/6: `i16[2]` (4 bytes) - fixed-point UVs (divide by 4096.0)

**Usage in Rendering:**
- If `has_uvs == true` and texture is available → textured rendering
- Otherwise → solid color fallback

## Loading Functions

| Function | Purpose |
|----------|---------|
| `load_car_model(id)` | Load car model from `car/<id>/body` |
| `load_course(id)` | Load track from `crs/c{id:03}/race.mdl` |
| `load_car_texture(id)` | Extract embedded TXS3 from car model |
| `load_course_texture()` | Load track texture |

## Track Loading

Tracks stored in `crs/<course_id>/race.mdl`:

```rust
pub fn load_course(course_id: u32) -> Result<TrackState, String> {
    let path = format!("crs/c{:03}/race.mdl", course_id);
    // Parse 3LDM header
    // Extract vertices + triangles
    // Compute center
    // Build spatial index
}
```

## See Also

- [[10_PC_Port/00_Index|PC Port Index]]
- [[10_PC_Port/12_Render_Issue|Render Issue]] (course texture loading)
- [[30_Technical/01_3LDM_Format|3LDM Format Spec]]
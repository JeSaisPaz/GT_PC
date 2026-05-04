---
tags: [pc-port, freecam, rust, wgpu, vulkan, rendering]
type: reference
project: GT PSP Freecam
status: active
created: 2026-04-30
updated: 2026-05-04
affected: src/main.rs, src/loader/mod.rs, src/renderer/mod.rs, src/camera/mod.rs, src/texture.rs, src/lib.rs
location: C:\Users\boutr\Desktop\sandbox\GTPSP-Freecam
---

# GT PSP Freecam Tool

> Standalone Rust + wgpu (Vulkan) tool for navigating GT PSP tracks with a freecam.
> **Location:** `C:\Users\boutr\Desktop\sandbox\GTPSP-Freecam\` (separate from the main `D:\GTPSP-decompile` repo)

## Project Structure

| File | Role |
|------|------|
| `src/main.rs` | Entry point, event loop, asset path resolution |
| `src/loader/mod.rs` | 3LDM model parser, TXS3 texture parser, course metadata |
| `src/renderer/mod.rs` | wgpu renderer (pipelines, buffers, texture binding) |
| `src/camera/mod.rs` | Freecam controls (WASD + mouse look) |

## Asset Path Resolution

Assets are expected at `assets/game/` relative to CWD or exe directory:

```
assets/game/
├── crs/
│   ├── c001.ad .. c114.ad   # Course metadata (spawn, checkpoints)
│   ├── c001x .. c114x       # Full course 3D models (3LDM/PACL)
│   ├── race.txs             # Shared course texture (TXS3/3SXT)
│   └── race.mdl             # Low-detail test mesh
```

### Path Resolution Strategy (`main.rs:17-29`)

1. Check `assets/game/crs` relative to CWD
2. Fallback to `{exe_dir}/assets/game/crs`
3. If neither exists, the error is logged

## Track Loading Fix

**Symptom:** `Failed to load track: The system cannot find the path specified. (os error 3)`

**Root Cause:** Hardcoded paths that didn't handle running from different working directories or exe-relative paths.

**Fix:** Added `get_assets_path()` with CWD-first, exe-fallback resolution. Also fixed `load_course_metadata()` in `src/loader/mod.rs:215` which was using a hardcoded `"assets/game/crs"` path — updated to use the same dynamic resolution.

## Texture Pipeline

The TXS3 texture (`race.txs`) is now properly loaded and passed to the renderer:

1. `load_track_texture()` parses 3SXT format (supports BC1/DXT1, BC3/DXT5, RGBA8888)
2. `Renderer::set_texture()` creates a GPU texture + sampler + bind group
3. Render pass selects `pipeline_tex` when texture is loaded and mesh triangles exist

## PPSSPP Texture Reference

Study performed on PPSSPP source (`source/PPSSPP/`) to understand PSP GPU texture processing.

### GE Texture Formats (`GPU/ge_constants.h`)

| Enum | Value | BPP | Description |
|------|-------|-----|-------------|
| `GE_TFMT_5650` | 0 | 16 | R=5,G=6,B=5, no alpha |
| `GE_TFMT_5551` | 1 | 16 | R=5,G=5,B=5,A=1 |
| `GE_TFMT_4444` | 2 | 16 | R=4,G=4,B=4,A=4 |
| `GE_TFMT_8888` | 3 | 32 | R=8,G=8,B=8,A=8 |
| `GE_TFMT_CLUT4` | 4 | 4 | 4-bit indexed (palette) |
| `GE_TFMT_CLUT8` | 5 | 8 | 8-bit indexed (palette) |
| `GE_TFMT_CLUT16` | 6 | 16 | 16-bit indexed (palette) |
| `GE_TFMT_CLUT32` | 7 | 32 | 32-bit indexed (palette) |
| `GE_TFMT_DXT1` | 8 | 4 | S3TC/DXT1 compressed |
| `GE_TFMT_DXT3` | 9 | 8 | S3TC/DXT3 compressed |
| `GE_TFMT_DXT5` | 10 | 8 | S3TC/DXT5 compressed |

Bits-per-pixel table in `GPU/Common/TextureDecoder.cpp:31-48`.

### Texture Swizzling

PSP textures use a **32-byte tile** swizzle pattern for cache efficiency.

**Key functions** in `GPU/Common/TextureDecoder.cpp`:
- `DoSwizzleTex16(src, dst, bxc, byc, pitch)` — linear → swizzled
- `DoUnswizzleTex16(src, dst, bxc, byc, pitch)` — swizzled → linear

**Block layout:** 8 rows × 4 columns of 4-byte tiles → 128 bytes per swizzle block.

**Offset calculation** (`GPU/Software/Sampler.cpp:267-285`):
```cpp
const uint32_t tile_size_bits = 32;
const uint32_t tiles_in_block_horizontal = 4;
const uint32_t tiles_in_block_vertical = 8;
constexpr uint32_t texels_per_tile = tile_size_bits / texel_size_bits;
uint32_t tile_u = u / texels_per_tile;
uint32_t tile_idx = (v % tiles_in_block_vertical) * tiles_in_block_horizontal
    + (v / tiles_in_block_vertical) * ((row_pitch * bpp / tile_size_bits) * tiles_in_block_vertical)
    + (tile_u % tiles_in_block_horizontal)
    + (tile_u / tiles_in_block_horizontal) * (tiles_in_block_horizontal * tiles_in_block_vertical);
return tile_idx * (tile_size_bits / 8) + ((u % texels_per_tile) * texel_size_bits) / 8;
```

### Texture Decoding Pipeline (`GPU/Common/TextureCacheCommon.cpp:1778`)

- **Unswizzle**: `UnswizzleFromMem()` → `DoUnswizzleTex16()` if `gstate.isTextureSwizzled()`
- **CLUT**: `DeIndexTexture`/`DeIndexTexture4` templates for 4/8/16/32-bit palette lookups
- **DXT**: `DecodeDXTBlocks<DXT1Block>` etc. — PSP stores DXT blocks in reverse byte order vs PC
- **16-bit (5650/5551/4444)**: `ConvertFormatToRGBA8888()` or `ReverseColors()` in `Common/Data/Convert/ColorConv.cpp`

### CLUT (Palette) Formats (`GPU/ge_constants.h:429-434`)

| Enum | Description |
|------|-------------|
| `GE_CMODE_16BIT_BGR5650` | Palette entries RGB565 |
| `GE_CMODE_16BIT_ABGR5551` | Palette entries ABGR1555 |
| `GE_CMODE_16BIT_ABGR4444` | Palette entries ABGR4444 |
| `GE_CMODE_32BIT_ABGR8888` | Palette entries ABGR8888 |

### GPU State Texture Registers

| Register | Offset | Field |
|----------|--------|-------|
| `texaddr[8]` | 0xA0-0xA7 | Texture base addresses per mip level |
| `texbufwidth[8]` | 0xA8-0xAF | Buffer width per mip level |
| `texformat` | 0xC3 | `texformat & 0xF` → `GETextureFormat` |
| `texmode` | 0xC2 | Bit 0 = swizzle, Bit 8 = shared CLUT for mips |
| `clutaddr` | 0xB0-0xB1 | CLUT base address |
| `clutformat` | 0xC5 | CLUT palette format + index shift/mask/start |
| `texsize[8]` | 0xB8-0xBF | Width/height: `1 << (texsize & 0xF)`, `1 << ((texsize>>8) & 0xF)` |

Texture width/height are always powers of two.

## 3LDM Mesh Parsing — Current Investigation

### Issue

The 3LDM parser only produces bounding box wireframes — actual mesh geometry is not parsed.

### Hex Analysis of c114x

File starts with **PACL** wrapper (`0x5050 0x4341` = "PACL" LE). The 3LDM magic is at file offset **0x200**.

3LDM header values:
- Models: 214 (hdr+0x10)
- Shapes: 241 (hdr+0x14)
- FVFs: 0 (hdr+0x18) — no FVF entries!
- Models ptr: 0xC4 (hdr+0x30, relative to 3LDM base)
- Meshes ptr: **0x0** (hdr+0x38) — null!
- FVF ptr: **0x0** (hdr+0x40) — null!
- Texture ptr: 0xA178 (hdr+0x48) — embedded TXS3

### Problem

Both `meshes_ptr` and `fvf_ptr` are null in this GT PSP variant. The PROJECT.md references `find_mesh_blocks()` from PDtools as a heuristic scanner for this case. Possible approaches:

1. **Fixed vertex format** (FVF count = 0): Use a hardcoded layout — `position(f32[3]) + normal(i16n[4]) + uv(i16[2])` = 24 bytes/vertex
2. **Mesh entries follow model array**: When meshes_ptr is null, shape entries may be stored immediately after the model entries (at hdr+0xC4 + 214*0x30)
3. **Embedded in model entries**: Each model entry may contain its own mesh data inline

### Renderer Fix

The `pipeline_tex` (TriangleList) was incorrectly selected for bounding box wireframe data. Fix: separate rendering into two passes:
- **Lines pass**: Always renders bounding boxes via `pipeline_lines` (LineList)
- **Mesh pass**: Only renders if mesh triangles exist, using `pipeline_tex` or `pipeline_no_tex`

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

## PSP Texture Decoder (Rust)

Ported from PPSSPP into `src/texture.rs`. Covers the full PSP GE texture pipeline.

### Module: `src/texture.rs`

**Format enum** — `GeTextureFormat` with 11 values: Rgb565, Rgba5551, Rgba4444, Rgba8888, Clut4/8/16/32, Dxt1/3/5.

**Color expansion helpers** (port of `Common/Data/Convert/ColorConv.h`):
- `convert_4_to_8(v)` — `(v << 4) | v`
- `convert_5_to_8(v)` — `(v << 3) | (v >> 2)`
- `convert_6_to_8(v)` — `(v << 2) | (v >> 4)`

**16-bit → RGBA8888** conversions:
- `rgba4444_to_rgba8888(src)` — R in bits 0-3 → 8-bit expansion
- `rgba5551_to_rgba8888(src)` — 1-bit alpha in bit 15
- `rgb565_to_rgba8888(src)` — always alpha=255

**Reversed (PSP native → PC) 16-bit swaps:**
- `reverse_4444(src)` — RGBA ↔ ABGR (A moves to bits 0-3)
- `reverse_5551(src)` — A moves to bit 0
- `reverse_565(src)` — R and B swap

**DXT block structs** (PSP layout, reversed from PC DDS):
- `Dxt1Block` — lines[4] + color1(u16 LE) + color2(u16 LE) = 8 bytes
- `Dxt3Block` — Dxt1Block + alpha_lines[4 u16 LE] = 16 bytes
- `Dxt5Block` — Dxt1Block + alpha_data2(u32 LE) + alpha_data1(u16 LE) + alpha1(u8) + alpha2(u8) = 16 bytes

**DXT decoding:**
- `decode_dxt1_block(dst, src)` — 2-bit color indices, 4-color palette from RGB565 endpoints
- `decode_dxt3_block(dst, src)` — explicit 4-bit alpha per pixel
- `decode_dxt5_block(dst, src)` — interpolated 8-bit alpha with 6- or 8-entry ramp
- DXT1 color selection: if c0 > c1 (LE) → 4-color mode with 2/3 mix; else 3-color + transparent
- DXT5 alpha: if a0 > a1 → 8 entries (7-interpolated); else → 6 entries + 0 + 255

**Swizzle** (port of `DoUnswizzleTex16`):
- `unswizzle_16_byte_blocks(dest, src, bxc, byc)` — rearranges 16-byte chunks: for each 8-row by 16-byte block, 8 rows of 4×u32 each packed contiguously in source → spread across 8 destination rows
- `unswizzle_texture_to_rgba()` — full pipeline: unswizzle raw → convert to RGBA8888 per format

**Full decode:**
- `decode_psp_texture(data, width, height, bufw, format, swizzled)` — coordinates everything: swizzle if needed, DXT decode, or direct format conversion to RGBA8888
- `compute_texture_bufw(width, format)` — align to 16-byte swizzle blocks per format BPP

### BPP Table (port of PPSSPP `textureBitsPerPixel`)

| Format | BPP | Effective |
|--------|-----|-----------|
| Rgb565, Rgba5551, Rgba4444 | 16 | 2 bytes/px |
| Rgba8888 | 32 | 4 bytes/px |
| Clut4 | 4 | 0.5 bytes/px |
| Clut8 | 8 | 1 byte/px |
| Clut16 | 16 | 2 bytes/px |
| Clut32 | 32 | 4 bytes/px |
| Dxt1 | 4 | 8 bytes/4×4 block |
| Dxt3, Dxt5 | 8 | 16 bytes/4×4 block |

## Linked Documents

- [[10_PC_Port/00_Index|PC Port Index]]
- `PROJECT.md` in repo root — full format reference for 3LDM, TXS3
- [[10_PC_Port/03_Model_Parser|3D Model Parser]] — reference in pc_port C++ codebase
- [[10_PC_Port/11_Course_Models|Course Models]] — cXXXx file format research
- PPSSPP source at `source/PPSSPP/` — `GPU/ge_constants.h`, `GPU/Common/TextureDecoder.{h,cpp}`, `GPU/Common/TextureCacheCommon.cpp`, `GPU/Software/Sampler.cpp`

---

*Created: 2026-04-30 — Initial documentation of the standalone Rust freecam tool*

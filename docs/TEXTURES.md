# Gran Turismo 3D Models — GT PSP / GT5 / GT6

> Compiled research focused on the GT PSP, GT5, and GT6 era (2009–2013).  
> Goal: fully understand the native model format to enable **bidirectional conversion** between GT's `.mdl` files and modern 3D formats (OBJ, FBX, GLTF, etc.).

---

## Table of Contents

1. [Engine & Format Lineage](#1-engine--format-lineage)
2. [MDL3 / ModelSet3 — The Core Format](#2-mdl3--modelset3--the-core-format)
   - [Header](#21-header)
   - [Model Entry](#22-model-entry)
   - [Mesh (Shape) Entry](#23-mesh-shape-entry)
   - [PMSH — Packed Mesh (GT6)](#24-pmsh--packed-mesh-gt6)
   - [FVF — Flexible Vertex Definition](#25-fvf--flexible-vertex-definition)
   - [Materials](#26-materials)
   - [Virtual Machine & Commands](#27-virtual-machine--commands)
   - [Bones & Skeletal Data](#28-bones--skeletal-data)
   - [Wing Data](#29-wing-data)
3. [ColorPatch / Pat0 — Paint Color System](#3-colorpatch--pat0--paint-color-system)
4. [Car Model File Structure (on disk)](#4-car-model-file-structure-on-disk)
5. [Key Differences by Platform](#5-key-differences-by-platform)
6. [LOD System & Tessellation](#6-lod-system--tessellation)
7. [Texture Binding Inside Models](#7-texture-binding-inside-models)
8. [Volume / Archive Extraction](#8-volume--archive-extraction)
9. [Current Extraction State (GT PSP / GT5 / GT6)](#9-current-extraction-state-gt-psp--gt5--gt6)
10. [Existing Tools & Their Capabilities](#10-existing-tools--their-capabilities)
11. [Conversion Pipeline — GT → Modern Format](#11-conversion-pipeline--gt--modern-format)
12. [Conversion Pipeline — Modern Format → GT (Re-import)](#12-conversion-pipeline--modern-format--gt-re-import)
13. [Known Blockers & Open Problems](#13-known-blockers--open-problems)
14. [Reference Code & Templates](#14-reference-code--templates)
15. [Sources](#15-sources)

---

## 1. Engine & Format Lineage

Gran Turismo PSP, GT5, and GT6 all share the same core engine lineage. The full progression of model formats across the series is:

| Format | Name | Games | Platform |
|---|---|---|---|
| *(PS1 proprietary)* | — | GT1, GT2 | PS1 |
| `MDLS` / ModelSet2 | ModelSet2 | GT3, GT4 | PS2 |
| **`MDL3` / ModelSet3** | **ModelSet3** | **GT PSP, GT5, GT6** | **PSP / PS3** |
| *(evolved MDL3)* | ModelSet3 v2 | GT Sport, GT7 | PS4/PS5 |

GT PSP is internally codenamed `gt5m` ("Gran Turismo 5 Mobile") and was developed in parallel with GT5. The two games share the same engine core, making GT PSP effectively a PSP port of the GT5 engine. Both games use MDL3, with the major difference being **endianness**: GT5/GT6 are big-endian (PS3), GT PSP is little-endian (PSP).

---

## 2. MDL3 / ModelSet3 — The Core Format

**Applies to:** GT PSP, GT5, GT6  
**Extension:** `.mdl` / no extension  
**Magic:** `MDL3` (big-endian, GT5/GT6) or `3LDM` (little-endian, GT PSP)  
**Endian:** Big (GT5/GT6 — PS3) / Little (GT PSP — PSP)

MDL3 is a direct evolution of MDLS/ModelSet2 from GT3/GT4. Key upgrades over ModelSet2:
- Flexible vertex definitions (FVF) — vertex layout is no longer hardcoded.
- Bundled shaders (RSX shader programs embedded in the model file on PS3).
- A built-in virtual machine (VM) with bytecode that controls how meshes are assembled and linked to each model at runtime.
- GT6 additionally introduces **PMSH** (Packed Mesh) for memory-efficient geometry storage, and **tessellated meshes** for high-quality smooth surfaces.

> ⚠️ The format is complex and many fields are still partially understood. The best authoritative reference is Nenkai's [PDTools.Files](https://github.com/Nenkai/PDTools/tree/master/PDTools.Files/Models/ModelSet3) source code and the [010 Editor templates](https://github.com/Nenkai/GT-File-Specifications-Documentation/tree/master/Formats/PS3/Models).

---

### 2.1 Header

Size: `0xE4` (Version 14)

| Field | Offset | Type | Description |
|---|---|---|---|
| Magic | `0x00` | `Int` | `MDL3` (BE) or `3LDM` (LE/PSP) |
| File Size | `0x04` | `Int` | Total size of the file |
| Relocation Pointer | `0x08` | `Int` | Pointer relocation table offset |
| Version Major | `0x0C` | `ushort` | Format version (controls parsing logic) |
| Runtime Flags | `0x0E` | `ushort` | Assigned at runtime only |
| Model Count | `0x10` | `ushort` | Number of models in the set (one model ≈ one mesh group) |
| Model Key Count | `0x12` | `ushort` | Number of model keys (debug only, optional) |
| Shape Count | `0x14` | `ushort` | Number of meshes (shapes) in the set |
| Shape Key Count | `0x16` | `ushort` | Number of mesh keys (debug only, optional) |
| FVF Count | `0x18` | `ushort` | Number of flexible vertex definitions |
| Bones Count | `0x1A` | `ushort` | Number of bones |
| Host Method Count | `0x1E` | `ushort` | Number of host VM callback methods |
| VM Stack Size | `0x20` | `ushort` | Size of the virtual machine stack |
| Models Pointer | `0x30` | `Model*` | Pointer to Model array |
| Model Keys Pointer | `0x34` | `ModelKey*` | Pointer to Model Key array |
| Meshes Pointer | `0x38` | `Mesh*` | Pointer to Mesh array |
| Mesh Keys Pointer | `0x3C` | `MeshKey*` | Pointer to Mesh Key array |
| FVF Pointer | `0x40` | `FVF*` | Pointer to FVF (vertex definition) array |
| Materials Pointer | `0x44` | `MaterialInfo*` | Pointer to material info |
| Texture Set Pointer | `0x48` | `TXS3*` | Embedded TXS3 texture set (linked to materials) |
| Shaders Header Pointer | `0x4C` | `ShaderHeader*` | Pointer to embedded RSX shader programs |
| Bones Pointer | `0x50` | `Bone*` | Pointer to bone array |
| Host Method Pointer | `0x58` | `HostMethod*` | Pointer to host method callbacks |
| VM Opcodes Size | `0x60` | `UInt` | Size of the VM bytecode block |
| VM Opcodes Offset | `0x64` | `byte*` | Pointer to VM bytecode |
| VM Instance Offset | `0x68` | `byte*` | Pre-allocated VM stack (runtime use only) |
| Texture Type Count | `0x8E` | `ushort` | Number of texture type keys |
| Texture Types Pointer | `0x90` | `TextureKey*` | Pointer to texture type name array |
| Wing Data Count | `0x98` | `ushort` | Wing/spoiler geometry count |
| Wing Data Pointer | `0x9C` | `WingData*` | Pointer to wing data |
| Shape Streaming Pointer | `0xAC` | `ShapeStreamInfo*` | Streaming info (used by courses) |
| Packed Mesh Key Count | `0xCA` | `ushort` | GT6: PMSH key count |
| Packed Mesh Keys Pointer | `0xCC` | `PackedMeshKey*` | GT6: PMSH key array |
| Packed Mesh Hedr Pointer | `0xCC` | `PackedMeshHeader*` | GT6: PMSH header (`PMSH` magic) |
| Separate Data Info Ptr | `0xDC` | `SepDataInfo*` | GT6: pointer for car streaming (`.sepdat`) |

---

### 2.2 Model Entry

One "Model" in MDL3 terminology is closer to a **mesh group** or **part** (e.g., body, interior, spoiler). Which meshes belong to a model is determined at runtime by the VM bytecode command stream.

Size: `0x30`

| Field | Offset | Type | Description |
|---|---|---|---|
| Unknown | `0x00` | `float` | Unknown |
| Origin | `0x04` | `Vector3` | World-space origin of this model part |
| Bounds Count | `0x12` | `ushort` | Number of bounding volume components |
| Bounds Pointer | `0x14` | `Vector3*` | Pointer to the bounding box |
| Setup Commands Pointer | `0x18` | `void*` | Pointer to VM opcodes for this model |
| Setup Commands Size | `0x1C` | `Int` | Size of the setup opcode block |
| VM Pointers | `0x20` | `Int[3]` | Runtime VM state pointers |
| Unknown | `0x2C` | `short` | Possibly an index |
| Unknown | `0x2E` | `ushort` | Possibly flags |

**Model Keys** (debug only, not required for parsing):

| Field | Offset | Type | Description |
|---|---|---|---|
| Name Pointer | `0x00` | `char*` | Source path string for this model (null-terminated) |
| Model ID | `0x04` | `Int` | Unique model ID |

---

### 2.3 Mesh (Shape) Entry

A Mesh/Shape is the actual geometric primitive — it holds vertex data, index data (triangles), material assignment, and a reference to the FVF that describes how to interpret the vertex buffer.

| Field | Offset | Type | Description |
|---|---|---|---|
| Flags | `0x00` | `ushort` | Mesh flags |
| FVF Index | `0x02` | `short` | Which FVF (vertex layout) to use. If `-1`, check PMSH. |
| Material Index | `0x04` | `short` | Which material to use for this mesh |
| Vertex Count | `0x08` | `uint` | Number of vertices. If `0`, check PMSH. |
| Vertex Pointer | `0x0C` | `byte*` | Pointer to vertex buffer. Read according to FVF. |
| Tri Length | `0x14` | `uint` | Byte size of the triangle index buffer. If `0`, check PMSH. |
| Tri Pointer | `0x18` | `short*` | Pointer to triangle index buffer. If `null`, check PMSH. |
| Tri Count | `0x26` | `short` | Number of triangles. If `0`, check PMSH. |
| Boundary Box Pointer | `0x28` | `Vector3*` | Pointer to the axis-aligned bounding box |
| PMSH Reference Pointer | `0x2C` | `PMSHRef*` | If not null, mesh uses packed GT6 geometry |

---

### 2.4 PMSH — Packed Mesh (GT6)

GT6 introduced PMSH ("Packed Mesh") as a memory-efficient geometry format. When a Mesh entry's `FVF Index` is `-1` and its `PMSH Reference Pointer` is non-null, the actual vertex and index data live in the PMSH block rather than inline.

The PMSH block starts with the magic bytes `PMSH`. The `PMSHRef` inside each Mesh entry holds:

| Field | Offset | Type | Description |
|---|---|---|---|
| (empty/reserved) | `0x00` | `Int[12]` | Padding |
| PMSH Index | `0x30` | `Int` | Which PMSH entry to use |

**Note on tessellated meshes:** Many car parts in GT6 use GPU tessellation. These meshes are marked with `_T` in their name/key. They appear as low-detail meshes because tessellation is applied at runtime by the GPU. They are actually the *highest quality* representations — the tessellator subdivides them at render time. This same technique is also present in GT Sport/GT7.

---

### 2.5 FVF — Flexible Vertex Definition

The FVF (Flexible Vertex Format / Flexible Vertex Definition) describes the per-vertex data layout for a mesh. Unlike fixed-function vertex formats, each FVF entry declares which fields are present and at what byte offsets inside the vertex buffer.

Size: `0x78`

| Field | Offset | Type | Description |
|---|---|---|---|
| Name Pointer | `0x00` | `char*` | FVF type name. Usually `"Flex"`. |
| Shader Related Index | `0x04` | `short` | Links to the shader for this vertex type |
| Field Def Pointer | `0x08` | `FVFField*` | Pointer to the field definition array |
| Field Count | `0x18` | `byte` | Number of fields in this FVF |
| FVF Structure Size | `0x19` | `byte` | Total size in bytes of one vertex (stride) |
| Field Array Pointer | `0x74` | `FVFArrayDef*` | Pointer to the field array definition |

**Common FVF field names (semantic names):**

| Semantic | Description |
|---|---|
| `pos` | Position (XYZ, typically `float3`) |
| `nrm` | Normal vector (`float3` or packed) |
| `tan` | Tangent vector (for normal mapping) |
| `map10` | Primary UV coordinates (UV channel 0) |
| `map12` | Secondary UV (UV channel 1) |
| `col0` | Vertex color (RGBA) |
| `wei` | Blend weights (for skeletal animation) |
| `idx` | Bone indices (for skeletal animation) |

The combination of fields and their data types varies per mesh. A typical car body mesh will have `pos`, `nrm`, `tan`, and at least `map10`. Track meshes often have multiple UV channels.

---

### 2.6 Materials

Materials in MDL3 tie together:
- A **texture index** (pointing into the embedded TXS3 texture set)
- **Shader parameters** (RSX-specific on PS3, GE-specific on PSP)
- **Render state** (blending mode, culling, alpha test, etc.)

The material system is tightly coupled with the embedded RSX shaders (PS3) or GE state (PSP), making full material reconstruction in modern formats non-trivial. Existing extraction tools approximate materials by assigning the diffuse texture and ignoring shader-specific effects.

---

### 2.7 Virtual Machine & Commands

MDL3 contains a **built-in bytecode virtual machine**. Each Model entry contains a pointer to a block of opcodes that:
1. Link specific meshes to a model.
2. Set RSX render state parameters directly (e.g., enable/disable specific render targets).
3. Trigger callbacks to the host application via "host methods."

Both PSP and PS3 support most of the same opcodes, but each platform also has its own platform-specific opcodes. The full opcode definitions are documented in [PDTools.Files/Models/ModelSet3/Commands](https://github.com/Nenkai/PDTools/tree/master/PDTools.Files/Models/ModelSet3/Commands).

> This VM is the primary reason why **mesh creation/re-import is currently unresolved** — you need to generate valid bytecode that correctly links meshes back to models, and the parameters need to match what the engine expects.

---

### 2.8 Bones & Skeletal Data

MDL3 supports skeletal (skinned) meshes through a bone array. Vertex weights and bone indices are stored in the vertex buffer via the `wei` and `idx` FVF fields.

Current extraction tools export skeletal models statically (all bones baked into world-space positions). Wheel positions and some other parts may be misaligned in extracted output because the skeleton is not applied during export.

---

### 2.9 Wing Data

The `WingData` and `WingDataKey` arrays handle dynamic spoiler/wing geometry. These are model parts that change position based on car speed (deployable aerodynamic elements). They are separate from the main mesh array.

---

## 3. ColorPatch / Pat0 — Paint Color System

**Applies to:** GT4 (PS2 reference), similar system in GT PSP  
**Extension:** `.pat`  
**Magic:** `Pat0`

Color Patches are responsible for holding the data to switch a car between paint colors without reloading the full model. Rather than having one model file per color, a single MDL/MDLS file is loaded once, and a ColorPatch file provides byte-level *patches* that overwrite specific parts of the in-memory model set (specifically its embedded texture set pointer entries) when switching between colors.

**Header (0x20 bytes):**

| Field | Offset | Type | Description |
|---|---|---|---|
| Magic | `0x00` | `int` | `Pat0` |
| Relocation Pointer | `0x04` | `int` | Pointer relocation table |
| Paint Count | `0x10` | `short` | Number of paint variations for this car |
| Patch Count | `0x12` | `short` | Number of byte patches per paint variation |
| Patch List | `0x20...` | `Patch*` | Array of patch entries |

**Patch Entry:**

| Field | Offset | Type | Description |
|---|---|---|---|
| Target Offset | `0x00` | `int` | Byte offset within the loaded model set to overwrite |
| Patch Size | `0x04` | `int` | Number of bytes to write |
| Patch Data | `0x08...` | `byte[]` | The replacement bytes (typically texture buffer pointers) |

> **Note:** Because model sets are loaded directly into runtime memory and texture data is swizzled inside the model structure, constructing ColorPatch files requires knowing exactly which bytes encode which texture reference inside the loaded model — this is extremely hard to reconstruct without access to the original authoring tools.

---

## 4. Car Model File Structure (on disk)

After unpacking the volume, each car's assets are stored under `car/<car_code>/`. The naming convention follows a consistent pattern across GT PSP, GT5, and GT6.

### GT PSP — `car/` directory

```
car/<car_code>/
├─ body           ← Main body MDL3 model (no extension)
├─ body_s         ← Secondary body (shadow / LOD mesh, if present)
├─ interior       ← Interior MDL3 model
├─ wheel_f        ← Front wheel model
├─ wheel_r        ← Rear wheel model
├─ tire_f         ← Front tire model (sometimes separate)
├─ tire_r         ← Rear tire model (sometimes separate)
└─ (no .pat file in GTPSP — color swapping uses a different mechanism)
```

### GT5 / GT6 — `car/` directory (Premium cars)

```
car/<car_code>/
├─ body           ← High-detail body MDL3 (Premium)
├─ body_s         ← MUST be in same directory as body (secondary geometry)
├─ interior       ← Interior model
├─ wheel_f / wheel_r
├─ brake_f / brake_r  ← Brake disc/caliper geometry
├─ <car_code>.img ← Embedded TXS3 textures (or inline in model)
└─ (GT6 only) *.sepdat ← Separate geometry/texture streaming file
```

> **Important (GT6):** If a `body_s` file exists in the same folder as `body`, it **must** be present during extraction. The GT6 extractor tool reads it alongside `body` to reconstruct the full mesh. If it's absent, the tool may fail silently or produce incomplete output.

### Standard (Economy) Cars — GT5

Standard cars in GT5 are upscaled assets from GT4 (PS2-era). They use the older MDLS/ModelSet2 format, not MDL3. Their body panel textures were not updated for GT5. They are stored separately from Premium cars.

---

## 5. Key Differences by Platform

| Feature | GT PSP | GT5 (PS3) | GT6 (PS3) |
|---|---|---|---|
| Endianness | Little-endian (`3LDM`) | Big-endian (`MDL3`) | Big-endian (`MDL3`) |
| Shaders embedded | No (GE state only) | Yes (RSX VP/FP programs) | Yes (RSX VP/FP programs) |
| PMSH (Packed Mesh) | No | No | **Yes** |
| Tessellation | No | No | **Yes** (meshes marked `_T`) |
| `.sepdat` streaming | No | No | **Yes** (GT6 DLC/updates) |
| Color patch | Similar system (no `.pat`) | Via TXS3 swapping | Via TXS3 swapping |
| Vertex format | FVF (flexible) | FVF (flexible) | FVF (flexible) |
| Model VM | Yes (PSP opcodes) | Yes (RSX opcodes) | Yes (RSX opcodes) |
| Bone support | Yes (static export only) | Yes (static export only) | Yes (static export only) |

---

## 6. LOD System & Tessellation

GT PSP, GT5, and GT6 all use a **multi-LOD** system. A single car file contains meshes at multiple levels of detail; the engine selects which LOD to render based on distance from camera.

The extraction tools target the **highest LOD available** (LOD0). The `body_s` secondary file typically contains lower-resolution shadow meshes and some additional geometry.

**GT6 Tessellation (`_T` meshes):**

Many GT6 car parts use PN-Triangle or similar GPU tessellation. These are the *highest quality* meshes, despite appearing low-poly before the tessellator subdivides them. They are identified by the `_T` suffix in their mesh key name. Current extraction tools export them as-is (not pre-subdivided), which is technically the raw base mesh that the GPU would tessellate at runtime. For a fully faithful reconstruction, tessellation should be applied as a subdivision step during export (Catmull-Clark or linear subdivision approximates PN-Triangle output).

---

## 7. Texture Binding Inside Models

The MDL3 header contains a direct pointer to an **embedded TXS3 texture set** at offset `0x48`. This means each model file is self-contained — its textures are stored inline within the `.mdl` file itself, not as separate `.img` files.

The material entries reference textures by index into this embedded TXS3 set. When exporting, the embedded TXS3 must be extracted first (see `TEXTURES.md`) before UV/material assignment can be resolved.

The texture set pointer is also what ColorPatch files manipulate — they overwrite the pointer values that link materials to specific texture entries, effectively swapping the color layer at runtime.

---

## 8. Volume / Archive Extraction

Before any model can be accessed, the game's volume archive must be extracted.

### GT PSP — `GT.VOL`

```bat
GTPSPVolTools unpack -i <path_to_GT.VOL>
```

Output will be the full `car/`, `crs/`, `piece_gt5m/` folder tree.

### GT5 / GT6 — `GT.VOL` + `PDIPFS`

GT5 and GT6 use a two-layer system:
- **`GT.VOL`** — the base game volume (main build)
- **`PDIPFS`** — patch file system used for game updates and DLC

**GTToolsSharp** handles both:

```bat
# Unpack GT.VOL (GT5/GT6)
GTToolsSharp unpack -i <path_to_GT.VOL> -o <output_folder>

# Unpack PDIPFS (updates/DLC)
GTToolsSharp unpack -i <PDIPFS_folder> -o <output_folder>

# Repack modified files back into PDIPFS
GTToolsSharp pack -i <PDIPFS> -p <RepackInput_folder> -o <RepackedFiles_folder>
```

> **GT5/GT6 Volume Encryption:** These volumes use a custom crypto algorithm with B-tree file tables. Decryption keys are required — most are bundled with GTToolsSharp, but some patch versions may require additional keys. Movies and some databases also have a secondary encryption layer.

### GT6 PDIPFS (no VOL)

Some GT6 DLCs and updates exist as pure PDIPFS without a backing VOL. Use the modified Flatz-based extractor (available on 3D Model Archives) to extract these. Note that only **full** PDIPFS entries are supported — partial entries are not.

---

## 9. Current Extraction State (GT PSP / GT5 / GT6)

| Game | Volume Extraction | Model Extraction | Texture Extraction | Re-import |
|---|---|---|---|---|
| **GT PSP** | ✅ Full (GTPSPVolTools) | ⚠️ Via JPCSP scene capture or Ninja Ripper; no direct file parser | ✅ Full (TXS3Converter) | ❌ Not supported |
| **GT5** | ✅ Full (GTToolsSharp) | ❌ No working MDL3 extractor exists | ✅ Full (TXS3Converter) | ❌ Not supported |
| **GT6** | ✅ Full (GTToolsSharp / modified Flatz tool) | ⚠️ Partial (~99% car meshes) via community tool; some UVs 2x scaled | ✅ Full (TXS3Converter) | ❌ Not supported |

**Why GT5 models cannot be extracted directly:**  
GT5 Premium cars use full MDL3 with embedded RSX shaders and a complex relocation pointer system. No public tool has fully implemented MDL3 parsing for GT5.

**Why GT6 extraction works better:**  
The community GT6 extractor (by id-daemon, based on EcheloCross's research) directly parses the MDL3 structure in `body` / `body_s` files. It handles ~99% of car meshes and extracts embedded textures. Known limitation: some UV coordinates are scaled 2× (fix not yet implemented).

---

## 10. Existing Tools & Their Capabilities

### GT PSP

| Tool | What it does |
|---|---|
| **GTPSPVolTools** | Unpack/repack `GT.VOL` |
| **JPCSP** + scene capture | Rip visible geometry from running game session |
| **Ninja Ripper** (+ JPCSP) | Runtime mesh/texture capture — unreliable (draw-call visibility issue) |
| **TXS3Converter** | Extract embedded `.img` textures from model files |
| **010 Editor** + MDL3 template | Binary inspection of model files |

### GT5

| Tool | What it does |
|---|---|
| **GTToolsSharp** | Unpack/repack `GT.VOL` and `PDIPFS` |
| **TXS3Converter** | Extract textures |
| **PDTools** (Nenkai) | Library for reading GT5/6 file formats; MDL3 partially implemented |
| **RPCS3** + scene capture | Runtime ripping via emulator |

### GT6

| Tool | What it does |
|---|---|
| **GTToolsSharp** / modified Flatz tool | Unpack `GT.VOL` / `PDIPFS` |
| **GT6 MDL extractor** (id-daemon / 3D Model Archives) | Drop `body` file → outputs `.OBJ` + `.ASCII` with all UV layers; ~99% car support |
| **GTS_MDL** (by id-daemon) | Originally for GT Sport / GT7; also works for GT6 models |
| **GTS_PACK** | Unpack course `.pack` files (SYS + VRAM sections) |
| **TXS3Converter** | Extract textures |

### GT Sport / GT7 (reference — same MDL3 family)

| Tool | What it does |
|---|---|
| **GTS_MDL** / **GTS_mdl_q** | Extract static + skeletal models to OBJ + ASCII; SMD skeleton output; tessellated mesh output as quads |
| **GTS_PACK** | Unpack PACK course files (merge `.sys` + `.vram` before passing to GTS_MDL) |

---

## 11. Conversion Pipeline — GT → Modern Format

### Step 1 — Extract the Volume

```bat
# GT PSP
GTPSPVolTools unpack -i GT.VOL

# GT5 / GT6
GTToolsSharp unpack -i GT.VOL -o extracted/
```

### Step 2 — Locate the Car Model Files

Navigate to `car/<car_code>/`. The relevant files are:
- `body` — primary geometry (MDL3, no extension)
- `body_s` — secondary geometry (keep in same folder)
- Interior, wheels, etc. as separate files

### Step 3 — Extract the Model (GT6)

Using the GT6 extractor (id-daemon tool):

```
Drag-and-drop `body` onto the tool executable.
→ Outputs: body.obj  (mesh)
           body.txt  (ASCII with all UV layers)
           body.dds  (extracted textures, if embedded)
```

For GT5, no direct file extractor is available. Use RPCS3 emulator + a GPU debugging tool (e.g., RenderDoc) to capture the scene geometry at runtime as a workaround.

### Step 4 — Extract Embedded Textures

```bat
TXS3Converter convert-png body
```

This extracts the embedded TXS3 from inside the model file and saves each texture as PNG.

### Step 5 — Import into 3D Software

Import the `.obj` into Blender, 3ds Max, Maya, or Cinema 4D. Assign the extracted PNG textures to the corresponding UV channels.

**Known issues after import:**
- Some UV coordinates may be scaled 2× (GT6 tool limitation)
- Wheels and some parts will be at incorrect positions (skeleton not applied)
- Tessellated `_T` meshes need manual subdivision to approximate runtime tessellation
- Shaders/materials are lost (diffuse texture only)

### GT PSP — Alternative (Runtime Ripping via JPCSP)

1. Load the game in JPCSP (Java PSP emulator)
2. Navigate to the car you want in-game
3. Use JPCSP's capture feature to dump the current scene
4. Import into Cinema 4D or 3ds Max
5. Reassign textures manually (UV mapping is captured but texture assignment needs manual work)

> Note: Due to GT PSP's draw-call-based visibility culling, only geometry currently on screen is drawn. Full car capture requires rotating the camera to expose all panels.

---

## 12. Conversion Pipeline — Modern Format → GT (Re-import)

> ⚠️ **This direction is currently unsupported and largely unsolved.** The following documents the known requirements and blockers.

### What Would Be Required

To successfully import a custom mesh back into a GT MDL3 file, a tool would need to:

1. **Build valid FVF entries** describing the new vertex layout (position, normals, UVs, tangents, vertex colors if needed).
2. **Write the vertex buffer** in the correct binary layout as described by the FVF.
3. **Write the index buffer** (triangle strip or indexed triangle list — format TBD).
4. **Build or reuse the material entries** linking the new mesh to TXS3 texture references.
5. **Rebuild or preserve the embedded TXS3** texture set, or replace it with new texture data (see `TEXTURES.md` for the TXS3 format).
6. **Generate valid VM bytecode** for the Model's command block — this is the hardest part. The opcodes must correctly reference mesh indices, material indices, and set the correct render state.
7. **Update all relocation pointers** in the header — MDL3 uses a pointer relocation system where internal offsets are rewritten at load time. Every pointer in the file must be registered in the relocation table.
8. **Maintain ColorPatch compatibility** if paint color switching is needed. This requires constructing new byte-level diffs between texture states.
9. **Respect platform endianness** — GT PSP requires little-endian output, GT5/GT6 require big-endian.

### Current Status

Nenkai's PDTools library has partial MDL3 write support: meshes can be constructed, but the VM bytecode generation for linking meshes to models is unresolved. Without valid command bytecode, the engine will not render the model correctly (wrong mesh groups activated, render state not set up, etc.).

No public end-to-end import tool exists.

---

## 13. Known Blockers & Open Problems

| Problem | Severity | Notes |
|---|---|---|
| VM bytecode generation | 🔴 Critical | Required for re-import. Many opcodes documented but correct sequencing for car models not established. |
| GT5 MDL3 extraction | 🔴 No tool | Full GT5 Premium model extraction is unsupported. Needs a dedicated parser. |
| UV 2× scaling in GT6 extractor | 🟡 Medium | Some UV layers come out doubled. No fix yet in the existing tool. |
| Tessellated mesh reconstruction | 🟡 Medium | `_T` meshes export as low-poly base; subdivision must be applied manually. |
| Skeletal / wheel positioning | 🟡 Medium | Bone transforms not applied on export; some parts at wrong positions. |
| ColorPatch construction | 🔴 Critical | Required for paint color switching. Requires byte-diff of in-memory texture states. |
| RSX shader reconstruction | 🔴 Not feasible | Embedded RSX shaders cannot easily be converted to modern GLSL/HLSL. |
| Relocation pointer table | 🔴 Critical | Every new pointer added to the MDL3 must be registered; incorrect tables crash the game. |
| GT PSP model files — no direct parser | 🟡 Medium | GT PSP MDL3 uses little-endian but shares structure with GT5. PDTools should be adaptable. |
| Material system complexity | 🟠 Hard | Materials tightly bind to platform shaders. Approximation only for export. |

---

## 14. Reference Code & Templates

| Resource | Description | URL |
|---|---|---|
| **PDTools** (Nenkai) | C# library — most complete MDL3 parser; GT5/6/PSP support | https://github.com/Nenkai/PDTools |
| **GT File Spec Documentation** | 010 Editor templates for MDL3, TXS3, Tex1, Pat0 | https://github.com/Nenkai/GT-File-Specifications-Documentation |
| **GTToolsSharp** | GT5/6/7 volume unpacker and PDIPFS repacker | https://github.com/Nenkai/GTToolsSharp |
| **GTPSPVolTools** | GT PSP volume unpacker/repacker | https://github.com/Nenkai/GTPSPVolTools |
| **TXS3Converter** | TXS3 `.img` ↔ PNG/DDS converter | https://github.com/Nenkai/TXS3Converter |
| **MDL3 Commands source** | VM opcode definitions for ModelSet3 | https://github.com/Nenkai/PDTools/tree/master/PDTools.Files/Models/ModelSet3/Commands |
| **GT6 MDL Extractor** | Drop-on-tool for GT6 car meshes → OBJ + ASCII | https://forum.xentax.com/viewtopic.php?t=19919 |
| **GTS_MDL / GTS_mdl_q** | GT Sport/GT7 model extractor (also handles GT6) | https://reshax.com/files/file/31-gran-turismo-sport-gran-turismo-7-ps4-model-tools/ |
| **GT Modding Hub — MDL3 page** | Most complete public structural documentation | https://nenkai.github.io/gt-modding-hub/formats/models/mdl3_modelset3/ |
| **GT Modding Hub — PS3 Models** | Current modding status + known limitations | https://nenkai.github.io/gt-modding-hub/ps3/models/ |
| **XeNTaX GT6 thread** | Community research thread; EcheloCross vertex/face extraction notes | https://forum.xentax.com/viewtopic.php?t=11783 |
| **TCRF — GT PSP** | Internal engine codenames, GT5m shared asset evidence | https://tcrf.net/Gran_Turismo_(PlayStation_Portable) |
| **GTPlanet — 3D import thread** | Community discussion on JPCSP capture workflow | https://www.gtplanet.net/forum/threads/importing-3d-vehicles-from-gt4-gt3.302578/ |

---

## 15. Sources

- [GT Modding Hub — MDL3/ModelSet3 Format](https://nenkai.github.io/gt-modding-hub/formats/models/mdl3_modelset3/)
- [GT Modding Hub — PS3 Models (Status)](https://nenkai.github.io/gt-modding-hub/ps3/models/)
- [GT Modding Hub — Pat0/ColorPatch](https://nenkai.github.io/gt-modding-hub/formats/models/pat0_colorpatch/)
- [GT Modding Hub — PSP Getting Started](https://nenkai.github.io/gt-modding-hub/psp/getting_started/)
- [PDTools — MDL3 Source Code](https://github.com/Nenkai/PDTools/tree/master/PDTools.Files/Models/ModelSet3)
- [GTToolsSharp — GT5/6/7 Volume Tools](https://github.com/Nenkai/GTToolsSharp)
- [GTPSPVolTools](https://github.com/Nenkai/GTPSPVolTools)
- [3D Model Archives — Gran Turismo Tools](https://sites.google.com/view/3d-model-archives/tools/daemon-tools/gran-turismo)
- [ResHax — GTS/GT7 Model Tools](https://reshax.com/files/file/31-gran-turismo-sport-gran-turismo-7-ps4-model-tools/)
- [XeNTaX — Gran Turismo 6 Models thread](https://forum.xentax.com/viewtopic.php?t=11783)
- [GTPlanet — Importing 3D Vehicles from GT4/GT3](https://www.gtplanet.net/forum/threads/importing-3d-vehicles-from-gt4-gt3.302578/)
- [TCRF — Gran Turismo PSP](https://tcrf.net/Gran_Turismo_(PlayStation_Portable))
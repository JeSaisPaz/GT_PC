# Gran Turismo PSP — `3LDM` / ModelSet3 File Format
> Research notes for parsing full mesh geometry from GTPSP car and track model files.

---

## 1. Overview

`3LDM` is the **little-endian variant** of the `MDL3` (ModelSet3) binary format used by
Polyphony Digital across GT5, GT6, and Gran Turismo PSP.  
Because the PSP is a little-endian platform, the 4-byte magic at offset `0x00` reads
`33 4C 44 4D` (`3LDM`) rather than `4D 44 4C 33` (`MDL3`).

Key facts:
- **Endian:** Little-endian (all multi-byte integers/floats are LE).
- **Used in:** Gran Turismo PSP (internal codename `gt5m`, "GT5 Mobile").
- **File extension:** `.mdl` or no extension (inside car model folders).
- **Version:** ModelSet3 Version `2` for GTPSP (from build metadata).
- **Engine branch:** Same engine as GT5/GT6, stripped to PSP hardware.
- **Predecessor:** `MDLS` / ModelSet2 (used in GT4 PS2, big-endian).
- **Container:** Files reside inside `GT.VOL` (encrypted volume). Extract with
  [GTPSPVolTools](https://github.com/Nenkai/GTPSPVolTools).
- **Car models location (after extraction):** `GTPSP/car/<car_label>/`

> **Reference implementation:** [PDTools (C#)](https://github.com/Nenkai/PDTools) —
> `PDTools.Files/Models/ModelSet3/` — the most authoritative parsing code available.  
> **010 Editor templates:** [GT-File-Specifications-Documentation](https://github.com/Nenkai/GT-File-Specifications-Documentation/tree/master/Formats/PS3/Models)
> (written for PS3/big-endian; byte-swap all fields for PSP).

---

## 2. High-Level File Structure

```
[File]
 ├── Header              (0xE4 bytes, version 14 fields documented)
 ├── Model Array         (Model[ModelCount])
 ├── Model Key Array     (ModelKey[ModelKeyCount])   ← optional/debug
 ├── Mesh/Shape Array    (Mesh[ShapeCount])
 ├── Mesh Key Array      (MeshKey[ShapeKeyCount])    ← optional/debug
 ├── FVF Array           (FVF[FVFCount])
 ├── Material Info
 ├── Texture Set (TXS3)  ← embedded texture set
 ├── Shader Header
 ├── Bone Array          (Bone[BonesCount])
 ├── VM Opcode Blob      ← bytecode for model setup virtual machine
 └── Various runtime/unknown sections
```

The file is **self-describing through pointer fields** in the header. Every array's
starting offset is stored as an absolute pointer in the header. On load the game
re-maps the file in-place (no copy-parsing), so all pointers are relative to file start.

---

## 3. File Header (offset `0x00`, size `0xE4`)

All fields are **little-endian** for GTPSP.

| Field                    | Offset | Type      | Notes |
|--------------------------|--------|-----------|-------|
| Magic                    | `0x00` | `u32`     | `0x4D444C33` = `MDL3` (BE) / `0x334C444D` = `3LDM` (LE/PSP) |
| File Size                | `0x04` | `u32`     | Total byte length of the file |
| Relocation Pointer       | `0x08` | `u32`     | Offset to relocation table |
| Version Major            | `0x0C` | `u16`     | ModelSet3 file version (GTPSP = `2`) |
| Runtime Flags            | `0x0E` | `u16`     | Set at runtime, ignore when parsing |
| Model Count              | `0x10` | `u16`     | Number of Model entries |
| Model Key Count          | `0x12` | `u16`     | Number of ModelKey debug entries |
| Shape Count              | `0x14` | `u16`     | Number of Mesh/Shape entries |
| Shape Key Count          | `0x16` | `u16`     | Number of MeshKey debug entries |
| FVF Count                | `0x18` | `u16`     | Number of Flexible Vertex Definitions |
| Bones Count              | `0x1A` | `u16`     | Number of bones |
| SizeFor0x68              | `0x1C` | `u16`     | Unknown |
| Host Method Count        | `0x1E` | `u16`     | VM host method count |
| VM Stack Size            | `0x20` | `u16`     | VM stack allocation size |
| Count 0x5C               | `0x22` | `u16`     | Unknown count |
| Unknown                  | `0x24` | `u16`     | Unknown |
| Count 0x78               | `0x26` | `u16`     | Unknown count |
| Count 0xA4               | `0x28` | `u16`     | Unknown count |
| Count 0x54               | `0x2A` | `u16`     | Unknown count |
| Count 0x88               | `0x2C` | `u16`     | Unknown count |
| Unknown                  | `0x2E` | `u16`     | Unknown, tends to be large |
| **Models Pointer**       | `0x30` | `u32 ptr` | → Model array |
| **Model Keys Pointer**   | `0x34` | `u32 ptr` | → ModelKey array |
| **Meshes Pointer**       | `0x38` | `u32 ptr` | → Mesh/Shape array |
| **Mesh Keys Pointer**    | `0x3C` | `u32 ptr` | → MeshKey array |
| **FVF Pointer**          | `0x40` | `u32 ptr` | → FVF array |
| **Materials Pointer**    | `0x44` | `u32 ptr` | → MaterialInfo structure |
| **Texture Set Pointer**  | `0x48` | `u32 ptr` | → Embedded TXS3 texture set |
| **Shaders Hdr Pointer**  | `0x4C` | `u32 ptr` | → Shader header |
| **Bones Pointer**        | `0x50` | `u32 ptr` | → Bone array |
| Unk 0x54 Pointer         | `0x54` | `u32 ptr` | Unknown keys |
| Host Method Pointer      | `0x58` | `u32 ptr` | → Host methods |
| Unk 0x5C Pointer         | `0x5C` | `u32 ptr` | Unknown |
| VM Opcodes Size          | `0x60` | `u32`     | Byte size of the VM opcode block |
| VM Opcodes Offset        | `0x64` | `u32 ptr` | → VM opcode byte array |
| VM Instance Offset       | `0x68` | `u32 ptr` | Pre-allocated VM instance (runtime) |
| Relocation Pointer 2     | `0x6C` | `u32`     | Possibly a second relocation pointer |
| Empty                    | `0x70` | `u32`     | Always 0 |
| Runtime Value            | `0x74` | `u32`     | Set at runtime |
| Unk 0x78 Pointer         | `0x78` | `u32 ptr` | Stride 0x14/entry |
| Unknown                  | `0x7C` | `u32`     | Unknown |
| Empty                    | `0x80` | `u32`     | Always 0 |
| Unknown Index            | `0x84` | `u16`     | Unknown |
| Unknown Index            | `0x86` | `u16`     | Unknown |
| Unk 0x88 Pointer         | `0x88` | `u32 ptr` | Also referenced by some material entries |
| Unknown Index            | `0x8C` | `u16`     | Unknown |
| Texture Type Count       | `0x8E` | `u16`     | Number of texture key entries |
| Texture Types Pointer    | `0x90` | `u32 ptr` | → Texture key name array |
| Empty                    | `0x94` | `u32`     | Always 0 |
| Wing Data Count          | `0x98` | `u16`     | Wing-related data count |
| Wing Data Key Count      | `0x9A` | `u16`     | Wing key count |
| Wing Data Pointer        | `0x9C` | `u32 ptr` | → Wing data |
| Wing Data Keys Pointer   | `0xA0` | `u32 ptr` | → Wing keys |
| Unk 0xA4 Pointer         | `0xA4` | `u32 ptr` | Stride 0x04/entry |
| Empty                    | `0xA8` | `u32`     | Always 0 |
| Shape Streaming Pointer  | `0xAC` | `u32 ptr` | ShapeStreamInfo for course files |
| Unk 0xB0 Pointer         | `0xB0` | `u32 ptr` | Stride 0x40/entry |
| Unk 0xB4                 | `0xB4` | `u32`     | Unknown |
| Unk 0xB8 Pointer         | `0xB8` | `u32 ptr` | Unknown |
| VM Context Pointer       | `0xBC` | `u32 ptr` | Stride 0x20/entry |
| Unk 0xC0 Pointer         | `0xC0` | `u32 ptr` | Unknown |
| Count 0xC0               | `0xC4` | `u16`     | Count for 0xC0 array |
| Empty                    | `0xC6` | `u16`     | Always 0 |
| Unk 0xC8                 | `0xC8` | `i16`     | Typically -1 |
| Packed Mesh Key Count    | `0xCA` | `u16`     | PMSH key count (GT6 mostly) |
| Packed Mesh Keys Pointer | `0xCC` | `u32 ptr` | → Packed mesh keys |
| Packed Mesh Hdr Pointer  | `0xD0` | `u32 ptr` | → `PMSH` packed mesh header |
| Empty                    | `0xD4` | `u16`     | Always 0 |
| Empty                    | `0xD8` | `u16`     | Always 0 |
| Separate Data Info Ptr   | `0xDC` | `u32 ptr` | GT6 only – separate geometry streaming |

> **PSP note:** PMSH (packed mesh) is a GT6 feature. In GTPSP files the PMSH
> pointer fields are typically null and mesh data is always inline.

---

## 4. Model Entry (size `0x30`)

There are `Model Count` entries starting at `Models Pointer`.

| Field                  | Offset | Type       | Notes |
|------------------------|--------|------------|-------|
| Unknown                | `0x00` | `f32`      | Unknown float |
| Origin                 | `0x04` | `vec3 f32` | World-space origin of this model group |
| Unknown                | `0x10` | `u8`       | Unknown |
| Unknown                | `0x11` | `u8`       | Unknown |
| Bounds Count           | `0x12` | `u16`      | Number of vec3 bound points |
| Bounds Pointer         | `0x14` | `u32 ptr`  | → Array of `vec3 f32` boundary points |
| Setup Commands Pointer | `0x18` | `u32 ptr`  | → VM bytecode opcodes for this model |
| Setup Commands Size    | `0x1C` | `u32`      | Byte size of the opcode block |
| VM Pointers            | `0x20` | `u32[3]`   | Runtime VM state, ignore |
| Unknown                | `0x2C` | `i16`      | Possibly an index |
| Unknown                | `0x2E` | `u16`      | Possibly flags |

### 4.1 Setup Commands (VM Opcodes)

Each model has an embedded opcode stream that drives a tiny virtual machine. The VM
**links meshes to the model** and can set shader/material parameters. The opcodes relevant
to geometry parsing include commands that specify which mesh index to activate. Full
opcode list: see `PDTools.Files/Models/ModelSet3/Commands/`.

---

## 5. Mesh / Shape Entry (size `0x30`)

There are `Shape Count` entries starting at `Meshes Pointer`.
**This is the core structure for geometry extraction.**

| Field                   | Offset | Type      | Notes |
|-------------------------|--------|-----------|-------|
| Flags                   | `0x00` | `u16`     | Mesh flags (visibility, strip type, etc.) |
| FVF Index               | `0x02` | `i16`     | Index into the FVF array. `-1` if PMSH (N/A on PSP). |
| Material Index          | `0x04` | `i16`     | Index into the material list |
| Unknown                 | `0x06` | `u8`      | Possibly padding |
| Unknown                 | `0x07` | `u8`      | Possibly padding |
| **Vertex Count**        | `0x08` | `u32`     | Total number of vertices in this mesh |
| **Vertex Pointer**      | `0x0C` | `u32 ptr` | → Raw vertex buffer (format defined by FVF) |
| Unknown                 | `0x10` | `u32`     | Unknown |
| **Tri Length**          | `0x14` | `u32`     | Byte length of the index/tri buffer |
| **Tri Pointer**         | `0x18` | `u32 ptr` | → Index buffer (array of `i16` strip indices) |
| Unknown                 | `0x1C` | `u32`     | Unknown/zero |
| Unknown                 | `0x20` | `u32`     | Unknown/zero |
| Unknown                 | `0x24` | `i16`     | Unknown/zero |
| **Tri Count**           | `0x26` | `i16`     | Number of triangle indices |
| Boundary Box Pointer    | `0x28` | `u32 ptr` | → `vec3[2]` AABB min/max |
| PMSH Reference Pointer  | `0x2C` | `u32 ptr` | → PMSHRef (null on PSP) |

**Index buffer format:** Triangle strips using `i16` (signed 16-bit) indices.
Negative indices act as strip-restart markers (PSP GE convention). Tri Count gives
the total index element count; divide by 3 after de-stripping to get face count.

---

## 6. FVF — Flexible Vertex Definition (size `0x78`)

There are `FVF Count` entries starting at `FVF Pointer`.
The FVF describes the **exact layout of each vertex** in the vertex buffer.

### 6.1 FVF Structure

| Field               | Offset | Type      | Notes |
|---------------------|--------|-----------|-------|
| Name Pointer        | `0x00` | `u32 ptr` | → Null-terminated string (usually `"Flex"`) |
| Shader Index        | `0x04` | `i16`     | Related shader/technique index |
| Field Def Pointer   | `0x08` | `u32 ptr` | → Array of `FVFField` entries |
| Runtime Index       | `0x0C` | `i16`     | Assigned at runtime, ignore |
| Empty               | `0x0E` | `i16`     | Zero |
| Unknown             | `0x10` | `u32`     | Unknown/zero |
| Unk 0x14 Pointer    | `0x14` | `u32 ptr` | Unknown |
| **Field Count**     | `0x18` | `u8`      | Number of FVFField entries |
| **Vertex Stride**   | `0x19` | `u8`      | Byte size of **one vertex** (sum of all field sizes) |
| Empty               | `0x1A` | `i16`     | Zero |
| Runtime Data        | `0x1C` | `u8[88]`  | Pre-allocated runtime block, ignore |
| Array Def Pointer   | `0x74` | `u32 ptr` | → FVFArrayDef (array metadata) |

### 6.2 FVFField — Individual Vertex Attribute

Each `FVFField` describes one attribute (position, normal, UV, color, etc.):

| Field        | Offset | Type  | Notes |
|--------------|--------|-------|-------|
| Semantic     | `0x00` | `u8`  | Attribute semantic (see table below) |
| Type         | `0x01` | `u8`  | Data type (see table below) |
| Offset       | `0x02` | `u8`  | Byte offset within one vertex struct |
| Index        | `0x03` | `u8`  | Attribute index (e.g. UV0, UV1) |

**Known FVFField Semantics:**

| Value | Name       | Description              |
|-------|------------|--------------------------|
| `0`   | `position` | Vertex position XYZ      |
| `1`   | `normal`   | Surface normal XYZ       |
| `2`   | `color`    | Vertex color RGBA        |
| `3`   | `map`      | UV / texture coordinates |
| `4`   | `tangent`  | Tangent vector           |
| `5`   | `binormal` | Binormal vector          |
| `6`   | `weight`   | Blend weight (skinning)  |
| `7`   | `index`    | Blend index (skinning)   |

**Known FVFField Types:**

| Value | C type      | Size    | Notes                         |
|-------|-------------|---------|-------------------------------|
| `0`   | `f32[2]`    | 8 bytes | 2×float                       |
| `1`   | `f32[3]`    | 12 bytes| 3×float (common for position) |
| `2`   | `f32[4]`    | 16 bytes| 4×float                       |
| `3`   | `i16[2]`    | 4 bytes | 2×int16 (fixed-point UV)      |
| `4`   | `i16[4]`    | 8 bytes | 4×int16                       |
| `5`   | `u8[4]`     | 4 bytes | RGBA byte color               |
| `6`   | `i16n[2]`   | 4 bytes | Normalized 2×int16            |
| `7`   | `i16n[4]`   | 8 bytes | Normalized 4×int16 (normals)  |
| `8`   | `u8n[4]`    | 4 bytes | Normalized 4×uint8            |

> **Tip:** For GTPSP cars, a typical vertex is `position(f32[3]) + normal(i16n[4]) + uv(i16[2])` =
> 12 + 8 + 4 = **24 bytes per vertex**. Always read `Vertex Stride` (FVF offset `0x19`) to
> confirm rather than assuming.

---

## 7. Model Keys and Mesh Keys (size `0x08` each)

Debug-only entries; present only in debug builds. Both share the same structure:

| Field        | Offset | Type      | Notes |
|--------------|--------|-----------|-------|
| Name Pointer | `0x00` | `u32 ptr` | → Null-terminated string (source file path) |
| ID           | `0x04` | `u32`     | Index into Model or Mesh array |

---

## 8. Relocation Table

Pointer fields in the file store **file-relative** offsets (absolute from file start `0x00`).
No runtime relocation is needed to parse — read a pointer field, seek to that offset, done.
The relocation table (at `Relocation Pointer`) only matters for the game engine's in-place
memory mapping and can be ignored during parsing.

---

## 9. Parsing Walkthrough

Below is the recommended step-by-step order to extract all mesh geometry:

```
1. READ HEADER
   - Verify magic: bytes[0..3] == [0x33, 0x4C, 0x44, 0x4D]  (LE "3LDM")
   - Read: ModelCount, ShapeCount, FVFCount  (at 0x10, 0x14, 0x18)
   - Read: MeshesPtr (0x38), FVFPtr (0x40), ModelsPtr (0x30)

2. READ FVF ARRAY  (FVFPtr → FVF[FVFCount])
   For each FVF:
     - stride = fvf.VertexStride  (offset 0x19)
     - fieldCount = fvf.FieldCount  (offset 0x18)
     - Seek to fvf.FieldDefPointer (offset 0x08)
     - Read fieldCount × FVFField structs
     - Build a layout map: { semantic, type, byte_offset }

3. READ MESH ARRAY  (MeshesPtr → Mesh[ShapeCount])
   For each mesh [i]:
     a. fvfIndex    = mesh[0x02] (i16)
     b. vertCount   = mesh[0x08] (u32)
     c. vertPtr     = mesh[0x0C] (u32 ptr → vertex buffer)
     d. triLen      = mesh[0x14] (u32)
     e. triPtr      = mesh[0x18] (u32 ptr → index buffer)
     f. triCount    = mesh[0x26] (i16)
     g. materialIdx = mesh[0x04] (i16)

     VERTICES:
       stride = fvf[fvfIndex].VertexStride
       For v in range(vertCount):
         raw = file[vertPtr + v*stride : vertPtr + v*stride + stride]
         For each field in fvf[fvfIndex].fields:
           Extract field value at (raw + field.byteOffset) using field.type
           Assign to vertex.position / vertex.normal / vertex.uv etc.

     INDICES (triangle strips):
       raw_indices = read i16[triCount] at triPtr
       De-strip:
         strip = []
         faces = []
         for idx in raw_indices:
           if idx < 0:          # negative = strip restart
             strip = []
           else:
             strip.append(idx)
             if len(strip) >= 3:
               if len(strip) % 2 == 1:   # winding alternates
                 faces.append((strip[-3], strip[-2], strip[-1]))
               else:
                 faces.append((strip[-3], strip[-1], strip[-2]))

4. (OPTIONAL) READ MODELS  (ModelsPtr → Model[ModelCount])
   Parse the VM opcode stream of each model to discover which mesh indices
   are associated with each named model group (body, wheel, etc.).
```

---

## 10. Triangle Strip De-stripping

GTPSP uses the PSP GE's native triangle-strip topology. The index buffer is a flat
`i16` array where:
- **Positive values** = vertex index.
- **Negative values** or a dedicated restart index signal the start of a new strip.
- Winding alternates for even/odd triangles within a strip.

Standard de-strip pseudocode (Python):

```python
def destrip(indices: list[int]) -> list[tuple]:
    faces = []
    strip = []
    for idx in indices:
        if idx < 0:
            strip = []
            continue
        strip.append(idx)
        n = len(strip)
        if n >= 3:
            a, b, c = strip[-3], strip[-2], strip[-1]
            # Alternate winding to maintain consistent face normals
            if (n - 3) % 2 == 0:
                faces.append((a, b, c))
            else:
                faces.append((a, c, b))
    return faces
```

---

## 11. Fixed-Point / Normalized Value Conversion

Several FVF types store values as integers that must be converted to floats:

| Type        | Conversion formula                        |
|-------------|-------------------------------------------|
| `i16n[2/4]` | `float_val = i16_val / 32767.0`           |
| `u8n[4]`    | `float_val = u8_val / 255.0`              |
| `i16[2]` UV | `float_val = i16_val / 4096.0` (typical) |

UV scale factor may vary; `4096.0` (2^12) is the most common divisor observed in
GT-series fixed-point UV coordinates.

---

## 12. LODs and Mesh Naming

Cars in GTPSP typically have **3 LODs** (Level of Detail) per part. Mesh keys
(if present) contain the source filename/path which often encodes the LOD level.
When keys are absent, LODs are usually ordered by mesh index from highest (LOD0)
to lowest detail (LOD2).

Meshes marked `_T` in their name use **tessellation** and appear low-poly — they
are actually the highest-quality meshes (parametric sub-division was applied at
runtime). This convention was carried from GTPSP into GT6.

---

## 13. Tools & References

| Tool / Resource | URL | Notes |
|---|---|---|
| **GTPSPVolTools** | https://github.com/Nenkai/GTPSPVolTools | Extract GT.VOL |
| **PDTools** (C#) | https://github.com/Nenkai/PDTools | ModelSet3 parser reference |
| **GT-File-Specs** | https://github.com/Nenkai/GT-File-Specifications-Documentation | 010 Editor templates |
| **GT Modding Hub** | https://nenkai.github.io/gt-modding-hub/ | Official documentation hub |
| **MDL3 Format Page** | https://nenkai.github.io/gt-modding-hub/formats/models/mdl3_modelset3/ | Detailed field tables |
| **Model Blog Post** | https://nenkai.github.io/gt-modding-hub/blog/2023/11/26/lifting-bonnet-on-gt-models/ | Design explanation |
| **PPSSPP** | https://www.ppsspp.org/ | PSP emulator (enable SCEIO debug log to trace file loads) |

---

## 14. Known Unknowns

- Several header fields (`0x24`, `0x2E`, `0x7C`, etc.) remain undocumented.
- Material structure internals are not fully documented in public sources.
- Bone/skinning data layout is not fully confirmed for GTPSP (most car parts are rigid).
- Host method (VM callback) semantics beyond mesh attachment are not fully reversed.

---

*Sources: [GT Modding Hub](https://nenkai.github.io/gt-modding-hub/) by Nenkai,
[PDTools source code](https://github.com/Nenkai/PDTools),
[GT-File-Specifications-Documentation](https://github.com/Nenkai/GT-File-Specifications-Documentation).
Compiled for reverse-engineering / research purposes.*
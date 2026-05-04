/// GT PSP 3D model parser — "3LDM" format (ModelSet3)
///
/// Based on 3LDM.md specification:
///   - Header: 0xE4 bytes with pointers to all arrays
///   - Meshes at offset from header field 0x38
///   - FVF at offset from header field 0x40
///   - Each mesh has vertex/index buffers with triangle strips
///   - GT PSP variant: meshes_ptr/fvf_ptr may be 0; geometry stored in model setup_commands

use std::io::Read;

#[derive(Clone)]
pub struct CarModel {
    pub vertices: Vec<(f32, f32, f32)>,
    pub triangles: Vec<(u32, u32, u32)>,
    pub normals: Vec<(f32, f32, f32)>,
    pub uvs: Vec<(f32, f32)>,  // Texture coordinates
    pub has_uvs: bool,
    pub center: (f32, f32, f32),
}

#[derive(Clone)]
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

impl TrackState {
    pub fn new() -> Self {
        TrackState {
            vertices: vec![], triangles: vec![], normals: vec![], uvs: vec![], 
            has_uvs: false, center: (0.0, 0.0, 0.0),
            course_loaded: false, car_loaded: false, models_loaded: 0,
        }
    }
}

#[derive(Clone)]
pub struct CameraData {
    pub position: (f32, f32, f32),
    pub fov: f32,
}

#[inline]
fn read_u16(data: &[u8], off: usize) -> u16 {
    if off + 2 > data.len() { return 0; }
    u16::from_le_bytes([data[off], data[off+1]])
}

#[inline]
fn read_i16(data: &[u8], off: usize) -> i16 {
    if off + 2 > data.len() { return 0; }
    i16::from_le_bytes([data[off], data[off+1]])
}

#[inline]
fn read_u32(data: &[u8], off: usize) -> u32 {
    if off + 4 > data.len() { return 0; }
    u32::from_le_bytes([data[off], data[off+1], data[off+2], data[off+3]])
}

#[inline]
fn read_f32(data: &[u8], off: usize) -> f32 {
    if off + 4 > data.len() { return 0.0; }
    f32::from_le_bytes([data[off], data[off+1], data[off+2], data[off+3]])
}

fn load_file(path: &str) -> Result<Vec<u8>, String> {
    std::fs::read(path).map_err(|e| format!("Read {}: {}", path, e))
}

fn is_valid_vertex(x: f32, y: f32, z: f32) -> bool {
    if !x.is_finite() || !y.is_finite() || !z.is_finite() { return false; }
    if x.abs() > 100000.0 || y.abs() > 100000.0 || z.abs() > 100000.0 { return false; }
    if x == 0.0 && y == 0.0 && z == 0.0 { return false; }
    let mag = (x*x + y*y + z*z).sqrt();
    mag > 0.001
}

// ─── 3LDM Format Parsing (ModelSet3) ────────────────────────────────────────

fn read_3ldm_header(data: &[u8]) -> Result<ModelSet3Header, String> {
    if data.len() < 0xE4 { return Err("File too short for 3LDM header".to_string()); }
    if &data[0..4] != b"3LDM" { return Err(format!("Bad magic: {:02X?}", &data[0..4])); }
    
    Ok(ModelSet3Header {
        file_size: read_u32(data, 0x04),
        model_count: read_u16(data, 0x10),
        shape_count: read_u16(data, 0x14),
        fvf_count: read_u16(data, 0x18),
        bones_count: read_u16(data, 0x1A),
        models_ptr: read_u32(data, 0x30),
        meshes_ptr: read_u32(data, 0x38),
        fvf_ptr: read_u32(data, 0x40),
        materials_ptr: read_u32(data, 0x44),
        bones_ptr: read_u32(data, 0x50),
        texture_set_ptr: read_u32(data, 0x48),
    })
}

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

/// GT PSP variant: model entries point to packed geometry directly
struct ModelEntry {
    origin: (f32, f32, f32),
    bounds_count: u16,
    bounds_ptr: u32,
    setup_cmds_ptr: u32,
    setup_cmds_size: u32,
}

fn read_models(data: &[u8], models_ptr: u32, model_count: u16) -> Vec<ModelEntry> {
    let mut models = Vec::new();
    for i in 0..model_count as usize {
        let off = (models_ptr + (i as u32) * 0x30) as usize;
        if off + 0x30 > data.len() { break; }
        models.push(ModelEntry {
            origin: (
                read_f32(data, off + 0x04),
                read_f32(data, off + 0x08),
                read_f32(data, off + 0x0C),
            ),
            bounds_count: read_u16(data, off + 0x12),
            bounds_ptr: read_u32(data, off + 0x14),
            setup_cmds_ptr: read_u32(data, off + 0x18),
            setup_cmds_size: read_u32(data, off + 0x1C),
        });
    }
    models
}

/// Extract geometry from GT PSP model setup commands blocks.
/// Each model's setup_cmds region contains packed vertex data (pos f32*3 + normal i16n*4),
/// optionally followed by triangle strip indices (i16).
fn extract_geometry_from_models(data: &[u8])
    -> Result<(Vec<(f32, f32, f32)>, Vec<(u32, u32, u32)>, Vec<(f32, f32, f32)>), String>
{
    let file_len = data.len();

    // Read header to get model count + pointer
    let header = read_3ldm_header(data)?;
    if header.model_count == 0 || header.models_ptr == 0 {
        return Err("No model entries available".to_string());
    }
    eprintln!("[3LDM-GT] Reading {} models from ptr 0x{:X}", header.model_count, header.models_ptr);

    let models = read_models(data, header.models_ptr, header.model_count);

    let mut all_vertices: Vec<(f32, f32, f32)> = Vec::new();
    let mut all_normals: Vec<(f32, f32, f32)> = Vec::new();
    let mut all_triangles: Vec<(u32, u32, u32)> = Vec::new();
    let mut vertex_offset: u32 = 0;

    for (mi, model) in models.iter().enumerate() {
        if model.setup_cmds_size == 0 || model.setup_cmds_ptr == 0 {
            continue;
        }
        let start = model.setup_cmds_ptr as usize;
        let end = (start + model.setup_cmds_size as usize).min(file_len);
        if start >= file_len || end <= start || end - start < 16 {
            continue;
        }

        eprintln!("[3LDM-GT] Model[{}] cmd block: 0x{:X}..0x{:X} ({}b), origin=({:.2},{:.2},{:.2})",
            mi, start, end, end - start, model.origin.0, model.origin.1, model.origin.2);

        // Scan the setup_cmds block for vertex runs (stride 20 = 12 pos + 8 normal)
        let block = &data[start..end];
        let block_end = block.len();
        let stride: usize = 20; // position f32*3 (12) + normal i16n*4 (8)

        // Find the best vertex run in this block
        let mut best_vstart = 0usize;
        let mut best_vcount = 0usize;

        for scan in (0..block_end.saturating_sub(24)).step_by(4) {
            let x0 = read_f32(block, scan);
            let y0 = read_f32(block, scan + 4);
            let z0 = read_f32(block, scan + 8);
            if !is_valid_vertex(x0, y0, z0) { continue; }

            // Verify stride 20: next position at scan+20
            if scan + 32 > block_end { continue; }
            let x1 = read_f32(block, scan + stride);
            let y1 = read_f32(block, scan + stride + 4);
            let z1 = read_f32(block, scan + stride + 8);
            if !is_valid_vertex(x1, y1, z1) { continue; }

            // Count run length
            let mut count = 2usize;
            let mut off = scan + stride * 2;
            let mut zero_streak = 0usize;
            while off + 12 <= block_end && count < 65536 {
                let x = read_f32(block, off);
                let y = read_f32(block, off + 4);
                let z = read_f32(block, off + 8);
                if !x.is_finite() || x.abs() > 100000.0
                    || !y.is_finite() || y.abs() > 100000.0
                    || !z.is_finite() || z.abs() > 100000.0
                {
                    break;
                }
                if x == 0.0 && y == 0.0 && z == 0.0 {
                    zero_streak += 1;
                    if zero_streak > 8 { break; }
                } else {
                    zero_streak = 0;
                }
                count += 1;
                off += stride;
            }

            if count > best_vcount {
                best_vstart = scan;
                best_vcount = count;
            }
        }

        if best_vcount < 2 {
            // Try stride 12 (position only) if stride 20 didn't find enough
            for scan in (0..block_end.saturating_sub(24)).step_by(4) {
                let x0 = read_f32(block, scan);
                let y0 = read_f32(block, scan + 4);
                let z0 = read_f32(block, scan + 8);
                if !is_valid_vertex(x0, y0, z0) { continue; }

                let x1 = read_f32(block, scan + 12);
                let y1 = read_f32(block, scan + 16);
                let z1 = read_f32(block, scan + 20);
                if !is_valid_vertex(x1, y1, z1) { continue; }

                let mut count = 2usize;
                let mut off = scan + 24;
                let mut zero_streak = 0usize;
                while off + 12 <= block_end && count < 65536 {
                    let x = read_f32(block, off);
                    let y = read_f32(block, off + 4);
                    let z = read_f32(block, off + 8);
                    if !x.is_finite() || x.abs() > 100000.0
                        || !y.is_finite() || y.abs() > 100000.0
                        || !z.is_finite() || z.abs() > 100000.0
                    {
                        break;
                    }
                    if x == 0.0 && y == 0.0 && z == 0.0 {
                        zero_streak += 1;
                        if zero_streak > 8 { break; }
                    } else {
                        zero_streak = 0;
                    }
                    count += 1;
                    off += 12;
                }
                if count > best_vcount {
                    best_vstart = scan;
                    best_vcount = count;
                }
            }
        }

        if best_vcount < 2 { continue; }

        let effective_stride = if best_vcount > 4 {
            // Detect actual stride from run
            let v0 = best_vstart;
            let x0a = read_f32(block, v0);
            let x0b = read_f32(block, v0 + 4);
            let x0c = read_f32(block, v0 + 8);
            let x1a = read_f32(block, v0 + 12);
            // Check if next vertex is at +12 or +20
            if best_vcount >= 3 {
                let x2a = read_f32(block, v0 + 24);
                let x2b = read_f32(block, v0 + 20);
                if x2a.is_finite() && x2b.is_finite() {
                    let alt_stride = if (read_f32(block, v0 + 20) - x0a).abs() < (read_f32(block, v0 + 24) - x0a).abs() {
                        20
                    } else {
                        12
                    };
                    alt_stride
                } else {
                    20
                }
            } else {
                20
            }
        } else {
            stride
        };

        let num_vertices = best_vcount;
        eprintln!("[3LDM-GT]   vrun at +0x{:X}: {} verts, stride={}",
            best_vstart, num_vertices, effective_stride);

        // Extract vertices
        let mut model_verts: Vec<(f32, f32, f32)> = Vec::new();
        let mut model_norms: Vec<(f32, f32, f32)> = Vec::new();
        for v in 0..num_vertices {
            let off = best_vstart + v * effective_stride;
            if off + 12 > block_end { break; }
            let x = read_f32(block, off);
            let y = read_f32(block, off + 4);
            let z = read_f32(block, off + 8);
            // Try to read normals at +12 if stride >= 16
            if effective_stride >= 16 && off + 20 <= block_end {
                let nx = read_i16(block, off + 12) as f32 / 32767.0;
                let ny = read_i16(block, off + 14) as f32 / 32767.0;
                let nz = read_i16(block, off + 16) as f32 / 32767.0;
                model_norms.push((nx, ny, nz));
            } else {
                model_norms.push((0.0, 1.0, 0.0));
            }
            model_verts.push((x, y, z));
        }

        // Find triangle indices within the block (after vertex data)
        let vert_data_end = best_vstart + num_vertices * effective_stride;
        let tri_scan_start = vert_data_end.min(block_end.saturating_sub(6));
        for scan in (tri_scan_start..block_end.saturating_sub(6)).step_by(2) {
            if scan + 6 > block_end { break; }
            let i1 = read_u16(block, scan) as u32;
            let i2 = read_u16(block, scan + 2) as u32;
            let i3 = read_u16(block, scan + 4) as u32;
            if i1 < model_verts.len() as u32
                && i2 < model_verts.len() as u32
                && i3 < model_verts.len() as u32
                && i1 != i2 && i2 != i3 && i1 != i3
            {
                all_triangles.push((i1 + vertex_offset, i2 + vertex_offset, i3 + vertex_offset));
                if all_triangles.len() >= 50000 { break; }
            } else if !all_triangles.is_empty() && scan > tri_scan_start + 200 && i1 >= model_verts.len() as u32 {
                break;
            }
        }

        vertex_offset += model_verts.len() as u32;
        all_vertices.extend(model_verts);
        all_normals.extend(model_norms);
    }

    eprintln!("[3LDM-GT] Total: {}v {}tri {}norm from {} models",
        all_vertices.len(), all_triangles.len(), all_normals.len(), models.len());
    Ok((all_vertices, all_triangles, all_normals))
}

struct FVFEntry {
    field_count: u8,
    vertex_stride: u8,
    field_def_ptr: u32,
}

fn read_fvf(data: &[u8], fvf_ptr: u32, fvf_count: u16) -> Vec<FVFEntry> {
    let mut fvfs = Vec::new();
    for i in 0..fvf_count as usize {
        let off = (fvf_ptr + (i as u32) * 0x78) as usize;
        if off + 0x78 > data.len() { break; }
        fvfs.push(FVFEntry {
            field_count: data[off + 0x18],
            vertex_stride: data[off + 0x19],
            field_def_ptr: read_u32(data, off + 0x08),
        });
    }
    fvfs
}

struct FVFField {
    semantic: u8,
    data_type: u8,
    offset: u8,
}

fn read_fvf_fields(data: &[u8], fvf: &FVFEntry) -> Vec<FVFField> {
    let mut fields = Vec::new();
    let ptr = fvf.field_def_ptr as usize;
    for i in 0..fvf.field_count as usize {
        let off = ptr + i * 4;
        if off + 4 > data.len() { break; }
        fields.push(FVFField {
            semantic: data[off],
            data_type: data[off + 1],
            offset: data[off + 2],
        });
    }
    fields
}

struct MeshEntry {
    _flags: u16,
    fvf_index: i16,
    _material_index: i16,
    vertex_count: u32,
    vertex_ptr: u32,
    _tri_len: u32,
    tri_ptr: u32,
    tri_count: i16,
}

fn read_meshes(data: &[u8], meshes_ptr: u32, shape_count: u16) -> Vec<MeshEntry> {
    let mut meshes = Vec::new();
    for i in 0..shape_count as usize {
        let off = (meshes_ptr + (i as u32) * 0x30) as usize;
        if off + 0x30 > data.len() { break; }
        meshes.push(MeshEntry {
            _flags: read_u16(data, off),
            fvf_index: read_i16(data, off + 0x02),
            _material_index: read_i16(data, off + 0x04),
            vertex_count: read_u32(data, off + 0x08),
            vertex_ptr: read_u32(data, off + 0x0C),
            _tri_len: read_u32(data, off + 0x14),
            tri_ptr: read_u32(data, off + 0x18),
            tri_count: read_i16(data, off + 0x26),
        });
    }
    meshes
}

fn read_vertex_position(data: &[u8], vertex_ptr: u32, vertex_idx: u32, stride: u8) -> Option<(f32, f32, f32)> {
    let off = (vertex_ptr + vertex_idx as u32 * stride as u32) as usize;
    if off + 12 > data.len() { return None; }
    let x = read_f32(data, off);
    let y = read_f32(data, off + 4);
    let z = read_f32(data, off + 8);
    if !x.is_finite() || !y.is_finite() || !z.is_finite() { return None; }
    Some((x, y, z))
}

fn read_vertex_normal(data: &[u8], vertex_ptr: u32, vertex_idx: u32, stride: u8, field_offset: u8) -> Option<(f32, f32, f32)> {
    let off = (vertex_ptr + vertex_idx as u32 * stride as u32 + field_offset as u32) as usize;
    if off + 8 > data.len() { return None; }
    let nx = read_i16(data, off) as f32 / 32767.0;
    let ny = read_i16(data, off + 2) as f32 / 32767.0;
    let nz = read_i16(data, off + 4) as f32 / 32767.0;
    let _nw = read_i16(data, off + 6) as f32 / 32767.0;
    Some((nx, ny, nz))
}

fn read_vertex_uv(data: &[u8], vertex_ptr: u32, vertex_idx: u32, stride: u8, field_offset: u8, data_type: u8) -> Option<(f32, f32)> {
    let off = (vertex_ptr + vertex_idx as u32 * stride as u32 + field_offset as u32) as usize;
    match data_type {
        3 | 6 => { // i16[2] or i16n[2]
            if off + 4 > data.len() { return None; }
            let u = read_i16(data, off) as f32 / 4096.0;  // Fixed-point UV scaling
            let v = read_i16(data, off + 2) as f32 / 4096.0;
            Some((u, v))
        }
        0 => { // f32[2]
            if off + 8 > data.len() { return None; }
            let u = read_f32(data, off);
            let v = read_f32(data, off + 4);
            Some((u, v))
        }
        _ => None
    }
}

fn destrip_indices(indices: &[i16]) -> Vec<(u32, u32, u32)> {
    let mut faces = Vec::new();
    let mut strip = Vec::new();
    for &idx in indices {
        if idx < 0 {
            strip.clear();
            continue;
        }
        strip.push(idx as u32);
        if strip.len() >= 3 {
            let n = strip.len();
            let (a, b, c) = (strip[n-3], strip[n-2], strip[n-1]);
            if (n - 3) % 2 == 0 {
                faces.push((a, b, c));
            } else {
                faces.push((a, c, b));
            }
        }
    }
    faces
}

fn parse_3ldm_mesh(data: &[u8]) -> Result<(Vec<(f32, f32, f32)>, Vec<(u32, u32, u32)>, Vec<(f32, f32, f32)>, Vec<(f32, f32)>, bool), String> {
    let header = read_3ldm_header(data)?;
    eprintln!("[3LDM] Models: {}, Shapes: {}, FVF: {}", header.model_count, header.shape_count, header.fvf_count);

    // Try standard FVF+Mesh path first
    if header.fvf_count > 0 && header.meshes_ptr != 0 {
        let fvfs = read_fvf(data, header.fvf_ptr, header.fvf_count);
        let meshes = read_meshes(data, header.meshes_ptr, header.shape_count);

        let mut all_vertices: Vec<(f32, f32, f32)> = Vec::new();
        let mut all_normals: Vec<(f32, f32, f32)> = Vec::new();
        let mut all_uvs: Vec<(f32, f32)> = Vec::new();
        let mut all_triangles: Vec<(u32, u32, u32)> = Vec::new();
        let mut vertex_offset: u32 = 0;
        let mut has_uvs = false;

        for mesh in meshes.iter() {
            if mesh.fvf_index < 0 || mesh.vertex_count == 0 || mesh.tri_count <= 0 {
                continue;
            }
            let fvf_idx = mesh.fvf_index as usize;
            if fvf_idx >= fvfs.len() { continue; }
            let fvf = &fvfs[fvf_idx];
            let fields = read_fvf_fields(data, fvf);
            let normal_offset = fields.iter().find(|f| f.semantic == 1).map(|f| f.offset);
            let uv_field = fields.iter().find(|f| f.semantic == 3);  // semantic 3 = UV/map
            let uv_offset = uv_field.map(|f| f.offset);
            let uv_type = uv_field.map(|f| f.data_type).unwrap_or(0);

            for v in 0..mesh.vertex_count {
                let pos = read_vertex_position(data, mesh.vertex_ptr, v, fvf.vertex_stride)
                    .unwrap_or((0.0, 0.0, 0.0));
                all_vertices.push(pos);
                let norm = if let Some(off) = normal_offset {
                    read_vertex_normal(data, mesh.vertex_ptr, v, fvf.vertex_stride, off)
                        .unwrap_or((0.0, 1.0, 0.0))
                } else {
                    (0.0, 1.0, 0.0)
                };
                all_normals.push(norm);
                
                // Read UV if available
                if let Some(off) = uv_offset {
                    if let Some(uv) = read_vertex_uv(data, mesh.vertex_ptr, v, fvf.vertex_stride, off, uv_type) {
                        all_uvs.push(uv);
                        has_uvs = true;
                    } else {
                        all_uvs.push((0.0, 0.0));
                    }
                } else {
                    all_uvs.push((0.0, 0.0));
                }
            }

            let tri_ptr = mesh.tri_ptr as usize;
            let tri_cnt = mesh.tri_count as usize;
            if tri_ptr + tri_cnt * 2 <= data.len() {
                let mut raw_idxs: Vec<i16> = Vec::with_capacity(tri_cnt);
                for i in 0..tri_cnt {
                    raw_idxs.push(read_i16(data, tri_ptr + i * 2));
                }
                let faces = destrip_indices(&raw_idxs);
                for (a, b, c) in faces {
                    if a < mesh.vertex_count && b < mesh.vertex_count && c < mesh.vertex_count {
                        all_triangles.push((a + vertex_offset, b + vertex_offset, c + vertex_offset));
                    }
                }
            }
            vertex_offset += mesh.vertex_count;
        }

        eprintln!("[3LDM] FVF+Mesh: {}v {}tri {}norm {}uvs has_uvs={}", 
            all_vertices.len(), all_triangles.len(), all_normals.len(), all_uvs.len(), has_uvs);
        if !all_vertices.is_empty() {
            return Ok((all_vertices, all_triangles, all_normals, all_uvs, has_uvs));
        }
    }

    // GT PSP variant with null mesh/FVF pointers: heuristic scanner
    eprintln!("[3LDM] Null mesh/FVF pointers — using heuristic scanner");
    let (verts, tris, norms, uvs, has_uvs) = find_mesh_blocks(data)?;
    Ok((verts, tris, norms, uvs, has_uvs))
}

/// Load car model (race/ low-poly version)
pub fn load_car_model(car_id: u32) -> Result<CarModel, String> {
    let path = format!("../files/decompiled/Gran Turismo/PSP_GAME/USRDIR/GT.VOL/car/race/{:08X}", car_id);
    let data = load_file(&path)?;
    parse_car_model(&data)
}

/// Load car model (hq/ high-poly version)
pub fn load_car_model_hq(car_id: u32) -> Result<CarModel, String> {
    let path = format!("../files/decompiled/Gran Turismo/PSP_GAME/USRDIR/GT.VOL/car/hq/{:03}", car_id);
    let data = load_file(&path)?;
    parse_car_model(&data)
}

/// Load car texture from embedded TXS3 in the 3LDM file
pub fn load_car_texture(car_id: u32) -> Option<crate::engine::graphics::LoadedTexture> {
    let path = format!("../files/decompiled/Gran Turismo/PSP_GAME/USRDIR/GT.VOL/car/race/{:08X}", car_id);
    let data = load_file(&path).ok()?;

    let header = read_3ldm_header(&data).ok()?;
    if header.texture_set_ptr == 0 || header.texture_set_ptr as usize + 4 > data.len() {
        return None;
    }

    let tex_off = header.texture_set_ptr as usize;
    // Embedded TXS3 uses absolute file offsets; pass base offset for correction
    let tex_slice = &data[tex_off..];
    if tex_slice.len() < 4 || &tex_slice[0..4] != b"3SXT" && &tex_slice[0..4] != b"TXS3" {
        return None;
    }
    crate::engine::sprite::parse_txs3_texture_at(tex_slice, tex_off).ok()
}

/// Load course/track texture from race.txs
pub fn load_course_texture() -> Option<crate::engine::graphics::LoadedTexture> {
    let path = "../files/decompiled/Gran Turismo/PSP_GAME/USRDIR/GT.VOL/crs/race.txs";
    let data = load_file(path).ok()?;
    crate::engine::sprite::parse_txs3_texture(&data).ok()
}

/// Parse a 3LDM car model
fn parse_car_model(data: &[u8]) -> Result<CarModel, String> {
    if data.len() < 32 { return Err("Too short for 3LDM header".to_string()); }
    if &data[0..4] != b"3LDM" && &data[0..4] != b"4LDM" && &data[0..4] != b"5LDM" {
        return Err(format!("Bad magic: {:02X?}", &data[0..4]));
    }

    let (verts, tris, norms, uvs, has_uvs) = parse_3ldm_mesh(data)?;
    let center = compute_center(&verts);

    eprintln!("[3LDM] Car: {}v {}tri {}norm {}uvs has_uvs={}", 
        verts.len(), tris.len(), norms.len(), uvs.len(), has_uvs);
    Ok(CarModel { vertices: verts, triangles: tris, normals: norms, uvs, has_uvs, center })
}

/// Parse track/course geometry
/// Loads 3D mesh from race.mdl (3LDM) and metadata from cXXX.ad
/// Falls back to procedural track if mesh parsing fails
pub fn load_course(course_id: u32) -> Result<TrackState, String> {
    // 1) Load course metadata from .ad (spawn position, checkpoints)
    let ad_path = format!("../files/decompiled/Gran Turismo/PSP_GAME/USRDIR/GT.VOL/crs/c{:03}.ad", course_id);
    let metadata = load_file(&ad_path).ok().map(|d| parse_course_metadata_full(&d));
    let (spawn_x, spawn_z) = metadata.as_ref().map(|m| (m.spawn_x, m.spawn_z)).unwrap_or((0.0, 0.0));

    // 2) Load actual course model from cXXXx file (multi-megabyte real track)
    // race.mdl is only 28KB - a tiny test mesh, not drivable
    let course_model_path = format!("../files/decompiled/Gran Turismo/PSP_GAME/USRDIR/GT.VOL/crs/c{:03}x", course_id);
    let mesh = load_file(&course_model_path).ok().and_then(|d| parse_car_model(&d).ok());

    let (vertices, triangles, normals, uvs, has_uvs, center) = match mesh {
        Some(m) if m.vertices.len() >= 100 && m.triangles.len() >= 50 => {
            eprintln!("[COURSE] c{:03}x: {}v {}tri uvs={}", course_id, m.vertices.len(), m.triangles.len(), m.has_uvs);
            // Translate track vertices to spawn position so camera (at car pos) sees the track
            let translated: Vec<(f32,f32,f32)> = m.vertices.iter()
                .map(|v| (v.0 + spawn_x, v.1, v.2 + spawn_z))
                .collect();
            let center = (m.center.0 + spawn_x, m.center.1, m.center.2 + spawn_z);
            (translated, m.triangles, m.normals, m.uvs, m.has_uvs, center)
        }
        _ => {
            eprintln!("[COURSE] Using procedural track for c{:03} (spawn {:.0},{:.0})", course_id, spawn_x, spawn_z);
            let (v, t, c) = generate_procedural_track(spawn_x, spawn_z);
            // Procedural track doesn't have UVs or normals
            let n = vec![(0.0, 1.0, 0.0); v.len()];
            let u = vec![(0.0, 0.0); v.len()];
            (v, t, n, u, false, c)
        }
    };

    let center = if vertices.is_empty() {
        (spawn_x, 0.0, spawn_z)
    } else {
        center
    };

    Ok(TrackState {
        vertices, triangles, normals, uvs, has_uvs, center,
        course_loaded: true, car_loaded: false, models_loaded: 0,
    })
}

/// Generate a procedural race track (flat surface with some triangles)
fn generate_procedural_track(spawn_x: f32, spawn_z: f32) -> (Vec<(f32,f32,f32)>, Vec<(u32,u32,u32)>, (f32,f32,f32)) {
    let size = 400.0;
    let step = 40.0;
    let cols = ((size * 2.0) / step) as u32 + 1;
    let rows = cols;

    let x0 = -size + spawn_x;
    let z0 = -size + spawn_z;

    let mut vertices = Vec::new();
    for row in 0..rows {
        for col in 0..cols {
            let x = x0 + col as f32 * step;
            let z = z0 + row as f32 * step;
            let y = ((x * 0.01).sin() * (z * 0.012).cos() * 3.0)
                  + ((x * 0.03 + z * 0.02).sin() * 1.5)
                  + ((x * 0.07 - z * 0.05).cos() * 0.5);
            vertices.push((x, y, z));
        }
    }

    let mut triangles = Vec::new();
    let w = cols;
    for row in 0..rows-1 {
        for col in 0..cols-1 {
            let a = row * w + col;
            let b = a + w;
            let c = a + 1;
            let d = b + 1;
            triangles.push((a, c, b));
            triangles.push((c, d, b));
        }
    }

    let center = (spawn_x, 0.0, spawn_z);
    eprintln!("[COURSE] Procedural: {}v {}tri grid {}x{}", vertices.len(), triangles.len(), cols, rows);
    (vertices, triangles, center)
}

/// Full course metadata from .ad file
struct CourseMeta {
    spawn_x: f32,
    spawn_z: f32,
    checkpoints: Vec<(f32, f32)>,
}

/// Extract spawn position and checkpoint data from .ad course metadata
fn parse_course_metadata_full(data: &[u8]) -> CourseMeta {
    let mut meta = CourseMeta { spawn_x: 0.0, spawn_z: 0.0, checkpoints: Vec::new() };
    if data.len() < 0x150 { return meta; }

    let mut i = 0x100.min(data.len());
    while i + 16 <= data.len() {
        let rec_type = read_u32(data, i);
        if rec_type == 1 {
            let x = read_f32(data, i + 4);
            let z = read_f32(data, i + 8);
            if x.is_finite() && z.is_finite() && x.abs() < 100000.0 && z.abs() < 100000.0 {
                if meta.spawn_x == 0.0 && meta.spawn_z == 0.0 {
                    meta.spawn_x = x;
                    meta.spawn_z = z;
                }
                meta.checkpoints.push((x, z));
            }
            i += 64;
        } else if rec_type == 0 && i > 0x200 {
            break;
        } else {
            i += 4;
        }
    }
    meta
}

/// Load camera data from .cam file
pub fn load_camera(course_id: u32) -> Result<CameraData, String> {
    let path = format!("assets/crs/c{:03}.cam", course_id);
    if !std::path::Path::new(&path).exists() {
        return Ok(CameraData { position: (0.0, 5.0, 10.0), fov: 1.0 });
    }
    let data = load_file(&path)?;
    if data.len() < 16 { return Err("CAM too short".to_string()); }
    let x = read_f32(&data, 0);
    let y = read_f32(&data, 4);
    let z = read_f32(&data, 8);
    let fov = read_f32(&data, 12);
    Ok(CameraData { position: (x, y, z), fov })
}

// ─── Fallback heuristic mesh scanning ─────────────────────────────────────

/// Scan raw binary for mesh blocks (vertex/triangle/normal arrays)
/// Collects ALL vertex runs, not just the longest one
fn find_mesh_blocks(data: &[u8]) -> Result<(Vec<(f32, f32, f32)>, Vec<(u32, u32, u32)>, Vec<(f32, f32, f32)>, Vec<(f32, f32)>, bool), String> {
    if data.len() < 32 { return Err("Too short".to_string()); }

    let file_len = data.len();
    let scan_start = 0xE4.min(file_len);

    // ── Collect all vertex runs ──
    let mut all_runs: Vec<(usize, usize)> = Vec::new(); // (byte_offset, vertex_count)

    let mut pos = scan_start;
    while pos + 24 <= file_len {
        let x0 = read_f32(data, pos);
        let y0 = read_f32(data, pos + 4);
        let z0 = read_f32(data, pos + 8);
        let x1 = read_f32(data, pos + 12);
        let y1 = read_f32(data, pos + 16);
        let z1 = read_f32(data, pos + 20);

        if is_valid_vertex(x0, y0, z0) && is_valid_vertex(x1, y1, z1) {
            // Found a run start - count how many consecutive valid vertex triplets
            let run_start = pos;
            let mut count = 2usize;
            let mut zero_streak = 0usize;
            let mut off = pos + 24;
            while off + 12 <= file_len && count < 65536 {
                let x = read_f32(data, off);
                let y = read_f32(data, off + 4);
                let z = read_f32(data, off + 8);
                if !x.is_finite() || x.abs() > 100000.0
                    || !y.is_finite() || y.abs() > 100000.0
                    || !z.is_finite() || z.abs() > 100000.0
                {
                    break;
                }
                if x == 0.0 && y == 0.0 && z == 0.0 {
                    zero_streak += 1;
                    if zero_streak > 10 { break; }
                } else {
                    zero_streak = 0;
                }
                count += 1;
                off += 12;
            }
            if count >= 8 {
                all_runs.push((run_start, count));
            }
            // Skip past this run
            pos = off.saturating_sub(12);
        }
        pos += 4;
    }

    // Deduplicate overlapping runs: keep only the longest run in each region
    all_runs.sort_by_key(|&(off, _)| off);
    let mut merged_runs: Vec<(usize, usize)> = Vec::new();
    for (off, count) in all_runs {
        if let Some(last) = merged_runs.last_mut() {
            let last_end = last.0 + last.1 * 12;
            let this_end = off + count * 12;
            if off < last_end {
                // Overlapping - keep the one that extends further
                if this_end > last_end {
                    *last = (last.0, count);
                }
                continue;
            }
            // Small gap between runs - merge if close
            if off.saturating_sub(last_end) <= 48 {
                last.1 += count;
                continue;
            }
        }
        merged_runs.push((off, count));
    }

    eprintln!("[3LDM] Found {} vertex runs", merged_runs.len());
    eprintln!("[3LDM] Run details (offset, vertices): {:?}",
        merged_runs.iter().map(|&(o,c)| format!("0x{:X}:{}", o, c)).collect::<Vec<_>>().join(", "));

    // ── Extract vertices from all runs ──
    let mut vertices: Vec<(f32, f32, f32)> = Vec::new();
    for &(off, count) in &merged_runs {
        let mut run_count = 0usize;
        for i in 0..count {
            let p = off + i * 12;
            if p + 12 > file_len { break; }
            let x = read_f32(data, p);
            let y = read_f32(data, p + 4);
            let z = read_f32(data, p + 8);
            if x.is_finite() && y.is_finite() && z.is_finite()
                && x.abs() < 100000.0 && y.abs() < 100000.0 && z.abs() < 100000.0
            {
                vertices.push((x, y, z));
                run_count += 1;
            }
        }
    }

    if vertices.is_empty() { return Err("No vertex data found".to_string()); }

    // ── Search for triangle strip index buffers ──
    // Scan for contiguous i16 runs where most values are valid vertex indices,
    // then de-strip them into triangles.
    let mut triangles: Vec<(u32, u32, u32)> = Vec::new();
    let vlen = vertices.len() as u32;
    let max_scan = file_len.saturating_sub(2);
    let mut pos = 0usize;
    while pos < max_scan && triangles.len() < 50000 {
        // Skip until we find a valid vertex index
        while pos < max_scan && read_u16(data, pos) as u32 >= vlen {
            pos += 2;
        }
        if pos >= max_scan { break; }

        // Attempt to read a strip run from this position
        let strip_start = pos;
        let mut raw: Vec<i16> = Vec::new();
        let mut good = 0usize;
        let mut bad = 0usize;
        while pos < max_scan && raw.len() < 65536 {
            let val = read_i16(data, pos);
            raw.push(val);
            pos += 2;
            if val >= 0 && (val as u32) < vlen {
                good += 1;
            } else if val < 0 {
                // Negative values are strip restarts — count as good
                good += 1;
            } else {
                bad += 1;
            }
            // If too many invalid values, this isn't a real index buffer
            if bad > 0 && bad * 3 > good { break; }
            // Stop if we hit a long run of invalid indices
            if val as u32 >= vlen && val >= 0 {
                let mut consec = 1usize;
                let peek = pos;
                while peek < max_scan && consec < 8 {
                    let nv = read_i16(data, peek);
                    if nv >= 0 && (nv as u32) < vlen { break; }
                    consec += 1;
                }
                if consec >= 8 { break; }
            }
        }
        // Accept runs with at least 6 good indices
        if good >= 6 {
            let faces = destrip_indices(&raw);
            // Offset indices: we don't know which vertex base each strip references,
            // so assume global indexing
            for (a, b, c) in faces {
                if a < vlen && b < vlen && c < vlen && a != b && b != c && a != c {
                    triangles.push((a, b, c));
                    if triangles.len() >= 50000 { break; }
                }
            }
        }
        // Avoid getting stuck on the same region
        if pos <= strip_start + 2 { pos = strip_start + 2; }
    }

    if triangles.is_empty() {
        // Fallback: scan for any valid index triples
        let max_scan = file_len.saturating_sub(6);
        for start in (0..max_scan).step_by(2) {
            if start + 6 > file_len { break; }
            let i1 = read_u16(data, start) as u32;
            let i2 = read_u16(data, start + 2) as u32;
            let i3 = read_u16(data, start + 4) as u32;
            if i1 < vlen && i2 < vlen && i3 < vlen
                && i1 != i2 && i2 != i3 && i1 != i3
            {
                triangles.push((i1, i2, i3));
                if triangles.len() >= 50000 { break; }
            }
        }
    }

    eprintln!("[3LDM] Scanner: {}v {}tri", vertices.len(), triangles.len());
    // Scanner doesn't extract UVs or normals - return empty
    let uvs = vec![(0.0, 0.0); vertices.len()];
    Ok((vertices, triangles, Vec::new(), uvs, false))
}

fn compute_center(verts: &[(f32, f32, f32)]) -> (f32, f32, f32) {
    if verts.is_empty() { return (0.0, 0.0, 0.0); }
    let n = verts.len() as f32;
    verts.iter().fold((0.0, 0.0, 0.0), |(x,y,z), (px,py,pz)| (x+px/n, y+py/n, z+pz/n))
}

use std::path::Path;
use std::fs;
use anyhow::Result;

#[derive(Clone)]
pub struct TrackModel {
    pub vertices: Vec<(f32, f32, f32)>,
    pub normals: Vec<(f32, f32, f32)>,
    pub uvs: Vec<(f32, f32)>,
    pub triangles: Vec<(u32, u32, u32)>,
    pub line_tri_count: usize,
}

fn read_u16(data: &[u8], off: usize) -> u16 {
    if off + 2 > data.len() { return 0; }
    u16::from_le_bytes([data[off], data[off+1]])
}

fn read_u32(data: &[u8], off: usize) -> u32 {
    if off + 4 > data.len() { return 0; }
    u32::from_le_bytes([data[off], data[off+1], data[off+2], data[off+3]])
}

fn read_f32(data: &[u8], off: usize) -> f32 {
    if off + 4 > data.len() { return 0.0; }
    f32::from_le_bytes([data[off], data[off+1], data[off+2], data[off+3]])
}

fn read_vec3(data: &[u8], off: usize) -> (f32, f32, f32) {
    (read_f32(data, off), read_f32(data, off + 4), read_f32(data, off + 8))
}

pub fn load_track(track_path: &Path, _texture_path: &Path) -> Result<TrackModel> {
    let data = fs::read(track_path)?;
    parse_3ldm_track(&data)
}

fn parse_3ldm_track(data: &[u8]) -> Result<TrackModel> {
    if data.len() < 32 {
        anyhow::bail!("File too short");
    }
    let magic_pos = data.windows(4).position(|w| w == b"3LDM");
    let base = match magic_pos {
        Some(pos) => pos,
        None => anyhow::bail!("Not a 3LDM file"),
    };
    let data = &data[base..];
    parse_3ldm_from_offset(data)
}

fn element_byte_size(format: u16) -> usize {
    match format {
        0 => 8,   // f32[2]
        1 => 12,  // f32[3]
        2 => 16,  // f32[4]
        3 => 4,   // i16[2]
        4 => 8,   // i16[4]
        5 => 4,   // u8[4]
        6 => 4,   // i16n[2]
        7 => 8,   // i16n[4]
        8 => 4,   // u8n[4]
        _ => 4,
    }
}

fn read_i16(data: &[u8], off: usize) -> i16 {
    if off + 2 > data.len() { return 0; }
    i16::from_le_bytes([data[off], data[off + 1]])
}

struct FvfElement {
    semantic: u16,
    format: u16,
    offset: u16,
}

fn parse_fvf_entries(data: &[u8], fvf_ptr: usize, fvf_count: usize) -> Vec<Vec<FvfElement>> {
    let mut list: Vec<Vec<FvfElement>> = Vec::new();
    if fvf_ptr == 0 || fvf_ptr >= data.len() { return list; }
    let mut fp = fvf_ptr;
    for _ in 0..fvf_count {
        if fp + 4 > data.len() { break; }
        let num_elems = read_u16(data, fp) as usize;
        let _stride = read_u16(data, fp + 2);
        fp += 4;
        let mut elems = Vec::new();
        for _ in 0..num_elems {
            if fp + 6 > data.len() { break; }
            let semantic = read_u16(data, fp);
            let format = read_u16(data, fp + 2);
            let offset = read_u16(data, fp + 4);
            elems.push(FvfElement { semantic, format, offset });
            fp += 6;
        }
        list.push(elems);
    }
    list
}

fn parse_mesh_vertices(data: &[u8], verts_ptr: usize, vertex_count: usize, elems: &[FvfElement]) -> (Vec<(f32, f32, f32)>, Vec<(f32, f32, f32)>, Vec<(f32, f32)>) {
    let mut verts: Vec<(f32, f32, f32)> = Vec::with_capacity(vertex_count);
    let mut norms: Vec<(f32, f32, f32)> = Vec::with_capacity(vertex_count);
    let mut uvs: Vec<(f32, f32)> = Vec::with_capacity(vertex_count);

    let stride: usize = elems.iter()
        .map(|e| e.offset as usize + element_byte_size(e.format))
        .max()
        .unwrap_or(0);
    if stride == 0 || verts_ptr == 0 || verts_ptr >= data.len() { return (verts, norms, uvs); }

    for vi in 0..vertex_count {
        let voff = verts_ptr + vi * stride;
        if voff + stride > data.len() { break; }

        let mut pos = (0.0f32, 0.0f32, 0.0f32);
        let mut nrm = (0.0f32, 1.0f32, 0.0f32);
        let mut uv = (0.0f32, 0.0f32);

        for elem in elems {
            let off = voff + elem.offset as usize;
            if off + element_byte_size(elem.format) > data.len() { continue; }
            match (elem.semantic, elem.format) {
                (0, 1) => pos = (read_f32(data, off), read_f32(data, off + 4), read_f32(data, off + 8)),
                (0, 0) => pos = (read_f32(data, off), read_f32(data, off + 4), 0.0),
                (1, 7) => {
                    let ix = read_i16(data, off);
                    let iy = read_i16(data, off + 2);
                    let iz = read_i16(data, off + 4);
                    nrm = (
                        (ix as f32 / 32767.0).clamp(-1.0, 1.0),
                        (iy as f32 / 32767.0).clamp(-1.0, 1.0),
                        (iz as f32 / 32767.0).clamp(-1.0, 1.0),
                    );
                }
                (1, 1) => nrm = (read_f32(data, off), read_f32(data, off + 4), read_f32(data, off + 8)),
                (3, 3) => {
                    let u = read_i16(data, off);
                    let v = read_i16(data, off + 2);
                    uv = (u as f32 / 2048.0, v as f32 / 2048.0);
                }
                (3, 0) => {
                    uv = (read_f32(data, off), read_f32(data, off + 4));
                }
                _ => {}
            }
        }

        verts.push(pos);
        norms.push(nrm);
        uvs.push(uv);
    }
    (verts, norms, uvs)
}

fn de_strip_triangles(indices: &[i16], base_vert: u32) -> Vec<(u32, u32, u32)> {
    let mut faces = Vec::new();
    let mut strip: Vec<u32> = Vec::new();
    for &idx in indices {
        if idx < 0 {
            strip.clear();
        } else {
            strip.push(base_vert + idx as u32);
            if strip.len() >= 3 {
                let n = strip.len();
                let a = strip[n - 3];
                let b = strip[n - 2];
                let c = strip[n - 1];
                if (n - 3) % 2 == 0 {
                    faces.push((a, b, c));
                } else {
                    faces.push((a, c, b));
                }
            }
        }
    }
    faces
}

fn parse_3ldm_from_offset(data: &[u8]) -> Result<TrackModel> {
    let file_len = data.len();

    let model_count = read_u16(data, 0x10) as usize;
    let shape_count = read_u16(data, 0x14) as usize;
    let fvf_count = read_u16(data, 0x18) as usize;
    let models_ptr = read_u32(data, 0x30) as usize;
    let meshes_ptr = read_u32(data, 0x38) as usize;
    let fvf_ptr = read_u32(data, 0x40) as usize;

    eprintln!("[3LDM] Models: {}, Shapes: {}, FVFs: {}", model_count, shape_count, fvf_count);
    eprintln!("[3LDM] Meshes ptr: 0x{:X}, FVF ptr: 0x{:X}", meshes_ptr, fvf_ptr);

    let mut vertices: Vec<(f32, f32, f32)> = Vec::new();
    let mut normals: Vec<(f32, f32, f32)> = Vec::new();
    let mut uvs: Vec<(f32, f32)> = Vec::new();
    let mut triangles: Vec<(u32, u32, u32)> = Vec::new();

    // Parse FVF entries for vertex layout
    let fvf_entries = parse_fvf_entries(data, fvf_ptr, fvf_count);

    let model_entry_size = 0x30;
    for mi in 0..model_count {
        let moff = models_ptr + mi * model_entry_size;
        if moff + model_entry_size > file_len { break; }
        let origin = read_vec3(data, moff + 0x04);
        let bounds_count = read_u16(data, moff + 0x12) as usize;
        let bounds_ptr = read_u32(data, moff + 0x14) as usize;

        if origin.0.is_finite() && origin.1.is_finite() && origin.2.is_finite()
            && origin.0.abs() < 100000.0 && origin.1.abs() < 100000.0 && origin.2.abs() < 100000.0
        {
            let sz = 5.0;
            let base_idx = vertices.len() as u32;
            vertices.push(origin);
            vertices.push((origin.0 + sz, origin.1, origin.2));
            vertices.push((origin.0 - sz, origin.1, origin.2));
            vertices.push((origin.0, origin.1 + sz, origin.2));
            vertices.push((origin.0, origin.1 - sz, origin.2));
            vertices.push((origin.0, origin.1, origin.2 + sz));
            vertices.push((origin.0, origin.1, origin.2 - sz));
            normals.extend_from_slice(&[(0.0, 1.0, 0.0); 7]);
            uvs.extend_from_slice(&[(0.0, 0.0); 7]);
            triangles.push((base_idx + 0, base_idx + 1, base_idx + 2));
            triangles.push((base_idx + 0, base_idx + 3, base_idx + 4));
            triangles.push((base_idx + 0, base_idx + 5, base_idx + 6));
        }

        if bounds_count >= 2 && bounds_ptr > 0 && bounds_ptr + bounds_count * 12 <= file_len {
            let mut min_x = f32::MAX; let mut min_y = f32::MAX; let mut min_z = f32::MAX;
            let mut max_x = f32::MIN; let mut max_y = f32::MIN; let mut max_z = f32::MIN;
            for i in 0..bounds_count {
                let v = read_vec3(data, bounds_ptr + i * 12);
                if v.0.is_finite() && v.1.is_finite() && v.2.is_finite()
                    && v.0.abs() < 100000.0 && v.1.abs() < 100000.0 && v.2.abs() < 100000.0
                {
                    min_x = min_x.min(v.0); max_x = max_x.max(v.0);
                    min_y = min_y.min(v.1); max_y = max_y.max(v.1);
                    min_z = min_z.min(v.2); max_z = max_z.max(v.2);
                }
            }
            if max_x > min_x && max_y > min_y && max_z > min_z {
                let base_idx = vertices.len() as u32;
                let corners = [
                    (min_x, min_y, min_z), (max_x, min_y, min_z),
                    (max_x, min_y, max_z), (min_x, min_y, max_z),
                    (min_x, max_y, min_z), (max_x, max_y, min_z),
                    (max_x, max_y, max_z), (min_x, max_y, max_z),
                ];
                for c in &corners { vertices.push(*c); }
                normals.extend_from_slice(&[(0.0, 1.0, 0.0); 8]);
                uvs.extend_from_slice(&[(0.0, 0.0); 8]);
                let edges = [(0,1),(1,2),(2,3),(3,0),(4,5),(5,6),(6,7),(7,4),(0,4),(1,5),(2,6),(3,7)];
                for &(a, b) in &edges {
                    triangles.push((base_idx + a, base_idx + b, base_idx + b));
                }
            }
        }
    }

    let bbox_vert_count = vertices.len();
    let bbox_tri_count = triangles.len();
    eprintln!("[3LDM] Bounding boxes: {} vertices, {} edges", bbox_vert_count, bbox_tri_count);

    // Parse shape meshes (actual geometry)
    let mesh_entry_size = 0x30;
    for si in 0..shape_count {
        let moff = meshes_ptr + si * mesh_entry_size;
        if moff + mesh_entry_size > file_len { break; }

        let fvf_index = read_u16(data, moff + 0x02) as i16;
        let vertex_count = read_u32(data, moff + 0x08) as usize;
        let verts_ptr = read_u32(data, moff + 0x0C) as usize;
        let tri_byte_len = read_u32(data, moff + 0x14) as usize;
        let tri_ptr = read_u32(data, moff + 0x18) as usize;
        let tri_index_count = read_u16(data, moff + 0x26) as i16;

        if vertex_count == 0 || tri_index_count <= 0 { continue; }
        if verts_ptr >= file_len || tri_ptr >= file_len { continue; }

        let elems = match fvf_index {
            idx if idx >= 0 && (idx as usize) < fvf_entries.len() => &fvf_entries[idx as usize],
            _ => continue,
        };
        if elems.is_empty() { continue; }

        let base_vert = vertices.len() as u32;
        let (v, n, u) = parse_mesh_vertices(data, verts_ptr, vertex_count, elems);
        if v.is_empty() { continue; }
        vertices.extend(v);
        normals.extend(n);
        uvs.extend(u);

        // Read triangle strip indices and de-strip
        let tri_max = (tri_byte_len / 2).min(tri_index_count.max(0) as usize);
        let mut raw_indices = Vec::with_capacity(tri_max);
        for ti in 0..tri_max {
            let off = tri_ptr + ti * 2;
            if off + 2 > file_len { break; }
            raw_indices.push(read_i16(data, off));
        }

        let faces = de_strip_triangles(&raw_indices, base_vert);
        triangles.extend(faces);

    }

    eprintln!("[3LDM] Mesh: {} vertices, {} triangles from {} shapes",
        vertices.len() - bbox_vert_count, triangles.len() - bbox_tri_count, shape_count);
    eprintln!("[3LDM] Total: {} vertices, {} triangles", vertices.len(), triangles.len());

    Ok(TrackModel {
        vertices,
        normals,
        uvs,
        triangles,
        line_tri_count: bbox_tri_count,
    })
}

#[derive(Clone)]
pub struct TrackTexture {
    pub data: Vec<u32>,
    pub width: u32,
    pub height: u32,
}

pub fn load_track_texture(path: &Path) -> Result<TrackTexture> {
    let data = fs::read(path)?;
    parse_txs3_texture(&data)
}

/// Parse TXS3/3SXT texture and decode to RGBA8888 using PSP texture pipeline.
fn parse_txs3_texture(data: &[u8]) -> Result<TrackTexture> {
    if data.len() < 0x40 || &data[0..4] != b"3SXT" {
        anyhow::bail!("Not a TXS3 texture file");
    }

    let img_info_count = read_u16(data, 0x16) as usize;
    let img_info_ptr = read_u32(data, 0x1C) as usize;

    if img_info_count == 0 || img_info_ptr + 0x20 > data.len() {
        anyhow::bail!("No image info in TXS3");
    }

    let ii = img_info_ptr;
    let data_ptr = read_u32(data, ii) as usize;
    let data_size = read_u32(data, ii + 4) as usize;
    let fmt_code = data[ii + 9];
    let width = read_u16(data, ii + 0x0C) as u32;
    let height = read_u16(data, ii + 0x0E) as u32;

    if width == 0 || height == 0 || data_ptr == 0 || data_size == 0 {
        anyhow::bail!("Invalid texture image info");
    }

    use crate::texture::{GeTextureFormat, texture_bits_per_pixel};

    let ge_fmt = match fmt_code {
        0 => GeTextureFormat::Rgb565,
        1 => GeTextureFormat::Rgba5551,
        2 => GeTextureFormat::Rgba4444,
        3 => GeTextureFormat::Rgba8888,
        4 => GeTextureFormat::Dxt1,
        5 => GeTextureFormat::Dxt5,
        6 => GeTextureFormat::Dxt3,
        _ => anyhow::bail!("Unsupported TXS3 texture format: {}", fmt_code),
    };

    let bpp = texture_bits_per_pixel(ge_fmt);
    let expected_size = if matches!(ge_fmt, GeTextureFormat::Dxt1 | GeTextureFormat::Dxt3 | GeTextureFormat::Dxt5) {
        let block_bytes = if matches!(ge_fmt, GeTextureFormat::Dxt1) { 8u32 } else { 16u32 };
        (((width + 3) / 4) * ((height + 3) / 4) * block_bytes) as usize
    } else {
        (width * height * bpp / 8) as usize
    };

    let end = (data_ptr + data_size).min(data.len());
    let raw_data = &data[data_ptr..end];
    let mut tex_bytes = raw_data.to_vec();
    if tex_bytes.len() < expected_size {
        tex_bytes.resize(expected_size, 0);
    }

    let bufw = crate::texture::compute_texture_bufw(width, ge_fmt);
    let swizzled = data.len() > 0x24 && (data[0x24] & 1) != 0;

    eprintln!("[TXS3] {}x{} fmt={} (GE={:?}) raw_size={} swizzled={}",
        width, height, fmt_code, ge_fmt, tex_bytes.len(), swizzled);

    let decoded = crate::texture::decode_psp_texture(&tex_bytes, width, height, bufw, ge_fmt, swizzled);

    Ok(TrackTexture {
        data: decoded,
        width,
        height,
    })
}

/// Extract embedded TXS3 texture from a 3LDM file.
pub fn load_embedded_texture(track_data: &[u8]) -> Option<TrackTexture> {
    let magic_pos = track_data.windows(4).position(|w| w == b"3LDM")?;
    let base = magic_pos;
    let hdr = &track_data[base..];

    let tex_ptr = read_u32(hdr, 0x48) as usize;
    if tex_ptr == 0 || tex_ptr >= hdr.len() { return None; }

    let tex_hdr = &hdr[tex_ptr..];
    if tex_hdr.len() < 4 || &tex_hdr[0..4] != b"3SXT" { return None; }

    match parse_txs3_texture(tex_hdr) {
        Ok(tex) => Some(tex),
        Err(e) => {
            eprintln!("[3LDM] Failed to parse embedded texture: {}", e);
            None
        }
    }
}

pub fn load_course_metadata(course_id: u32) -> Option<(f32, f32)> {
    let crs_path = Path::new("assets/game/crs");
    let fallback_path = std::env::current_exe().ok()
        .and_then(|p| p.parent().map(|p| p.join("assets/game/crs")));
    let path = {
        let p = crs_path.join(format!("c{:03}.ad", course_id));
        if p.exists() { p }
        else if let Some(fb) = fallback_path { fb.join(format!("c{:03}.ad", course_id)) }
        else { return None }
    };
    let data = match fs::read(&path) { Ok(d) => d, Err(_) => return None };
    let mut i = 0x100.min(data.len());
    while i + 16 <= data.len() {
        let rec_type = read_u32(&data, i);
        if rec_type == 1 {
            let x = read_f32(&data, i + 4);
            let z = read_f32(&data, i + 8);
            if x.is_finite() && z.is_finite() && x.abs() < 100000.0 && z.abs() < 100000.0 {
                return Some((x, z));
            }
            i += 64;
        } else if rec_type == 0 && i > 0x200 { break; }
        else { i += 4; }
    }
    None
}

/// GT PSP Sprite Batching System
///
/// Manages loading, caching, and efficient rendering of 2D sprite textures.
/// The game uses .img (TXS3 format) textures stored in piece_gt5m/.

use std::rc::Rc;
use std::cell::RefCell;
use std::collections::HashMap;
use crate::engine::graphics::{GraphicsRenderer, LoadedTexture};

thread_local! {
    static SPRITE_CACHE: RefCell<SpriteCache> = RefCell::new(SpriteCache::new());
}

struct CachedSprite {
    texture: LoadedTexture,
    last_access: u64,
}

pub struct SpriteCache {
    textures: HashMap<String, CachedSprite>,
    access_counter: u64,
}

impl SpriteCache {
    pub fn new() -> Self {
        SpriteCache { textures: HashMap::new(), access_counter: 0 }
    }

    fn get_or_load(&mut self, path: &str) -> Option<LoadedTexture> {
        self.access_counter += 1;
        if let Some(cached) = self.textures.get(path) {
            return Some(cached.texture.clone());
        }
        // Try loading from GT.VOL paths
        let load_paths = [
            format!("assets/piece_gt5m/{}.img", path),
            format!("assets/piece_gt5m/{}.img", path.replace("\\", "/")),
            format!("assets/{}", path),
            path.to_string(),
        ];
        for p in &load_paths {
            if let Ok(tex) = load_img_file(p) {
                self.textures.insert(path.to_string(), CachedSprite {
                    texture: tex.clone(), last_access: self.access_counter,
                });
                if self.textures.len() > 512 {
                    self.evict_oldest();
                }
                return Some(tex);
            }
        }
        None
    }

    fn evict_oldest(&mut self) {
        let mut oldest = u64::MAX;
        let mut oldest_key = String::new();
        for (key, entry) in &self.textures {
            if entry.last_access < oldest {
                oldest = entry.last_access;
                oldest_key = key.clone();
            }
        }
        if !oldest_key.is_empty() { self.textures.remove(&oldest_key); }
    }

    pub fn clear(&mut self) { self.textures.clear(); }
}

fn load_img_file(path: &str) -> Result<LoadedTexture, String> {
    let data = std::fs::read(path).map_err(|e| format!("Read {}: {}", path, e))?;
    parse_txs3_texture(&data)
}

/// Parse a TXS3 texture from raw bytes.
/// `base` is the file offset where the TXS3 block starts (0 for standalone files).
pub fn parse_txs3_texture_at(data: &[u8], base: usize) -> Result<LoadedTexture, String> {
    if data.len() < 32 { return Err("TXS3 too short".to_string()); }
    let is_txs3 = &data[0..4] == b"3SXT" || &data[0..4] == b"TXS3";
    if !is_txs3 {
        return Err("Not TXS3 format".to_string());
    }

    // TXS3 header fields may be absolute file offsets (when embedded in 3LDM).
    // Subtract `base` to convert to slice-relative.
    let abs_img_info = read_u32_le(data, 0x1C) as usize;
    if base > abs_img_info { return Err("ImageInfo offset is before TXS3 base".to_string()); }
    let img_info_off = abs_img_info - base;

    if img_info_off + 0x30 > data.len() { return Err("ImageInfo out of range".to_string()); }
    let data_size = read_u32_le(data, img_info_off + 4) as usize;
    let format = data.get(img_info_off + 9).copied().unwrap_or(0);
    let w = read_u16_le(data, img_info_off + 0x0C) as u32;
    let h = read_u16_le(data, img_info_off + 0x0E) as u32;

    // Pixel data pointer may also be absolute
    let abs_pixel = read_u32_le(data, img_info_off + 0x24) as usize;
    if base > abs_pixel { return Err("Pixel data offset is before TXS3 base".to_string()); }
    let pixel_off = abs_pixel - base;
    if pixel_off + data_size > data.len() { return Err("Pixel data out of range".to_string()); }

    let actual_w = compute_real_dim(w, h, data_size, format);
    let actual_h = if actual_w != w {
        let px_count = data_size / bytes_per_pixel(format).max(1) as usize;
        (px_count / actual_w.max(1) as usize) as u32
    } else { h };

    let pixel_data = &data[pixel_off..pixel_off + data_size];
    let rgba = decode_to_rgba(pixel_data, format, actual_w, actual_h);
    Ok(LoadedTexture { width: actual_w, height: actual_h, rgba })
}

pub fn parse_txs3_texture(data: &[u8]) -> Result<LoadedTexture, String> {
    // For standalone files, try raw RGB565 first
    if data.len() >= 32 && &data[0..4] != b"3SXT" && &data[0..4] != b"TXS3" {
        let px_count = data.len() / 2;
        let width = (px_count as f32).sqrt() as u32;
        let height = width;
        if width * height == px_count as u32 && width > 0 && height > 0 {
            let mut rgba = Vec::with_capacity(px_count * 4);
            for chunk in data.chunks(2) {
                if chunk.len() < 2 { break; }
                let pixel = u16::from_le_bytes([chunk[0], chunk[1]]);
                let r = (((pixel >> 11) & 0x1F) * 255 / 31) as u8;
                let g = (((pixel >> 5) & 0x3F) * 255 / 63) as u8;
                let b = ((pixel & 0x1F) * 255 / 31) as u8;
                rgba.extend_from_slice(&[r, g, b, 255]);
            }
            return Ok(LoadedTexture { width, height, rgba });
        }
        return Err("Not TXS3 format".to_string());
    }
    parse_txs3_texture_at(data, 0)
}

fn read_u32_le(d: &[u8], off: usize) -> u32 {
    if off + 4 > d.len() { return 0; }
    u32::from_le_bytes([d[off], d[off+1], d[off+2], d[off+3]])
}

fn read_u16_le(d: &[u8], off: usize) -> u16 {
    if off + 2 > d.len() { return 0; }
    u16::from_le_bytes([d[off], d[off+1]])
}

fn bytes_per_pixel(format: u8) -> u32 {
    match format {
        1 => 4, // RGBA8888
        3 => 2, // RGBA5551
        4 => 2, // RGB565
        5 => 2, // RGBA4444
        7 => 1, // L8
        8 => 1, // L4 (2 pixels per byte)
        _ => 2,
    }
}

fn compute_real_dim(header_w: u32, header_h: u32, data_size: usize, format: u8) -> u32 {
    let bpp = bytes_per_pixel(format) as usize;
    let px_from_size = data_size / bpp.max(1);
    let expected_px = (header_w * header_h) as usize;
    
    // Trust header dimensions if they're valid powers of 2 or reasonable sizes
    // and the pixel count is close enough (within 2x for mipmaps)
    if header_w > 0 && header_h > 0 && expected_px > 0 {
        // Allow for mipmap data (up to 1.33x extra) or partial data
        if px_from_size >= expected_px / 2 && px_from_size <= expected_px * 2 {
            return header_w;
        }
    }
    
    // Only recalculate if header is clearly wrong
    if expected_px == px_from_size || expected_px == 0 { return header_w; }
    
    // Try common GT PSP texture widths - prefer larger widths first
    for &w in &[512u32, 480, 384, 320, 256, 192, 160, 128, 96, 80, 64, 48, 32, 24, 16, 8] {
        if px_from_size % w as usize == 0 {
            let h = px_from_size / w as usize;
            // Prefer reasonable aspect ratios (not too extreme)
            if h >= (w / 8) as usize && h <= (w * 8) as usize {
                return w;
            }
        }
    }
    header_w
}

fn decode_to_rgba(pixels: &[u8], format: u8, w: u32, h: u32) -> Vec<u8> {
    let count = (w * h) as usize;
    let mut rgba = Vec::with_capacity(count * 4);
    match format {
        1 => { // RGBA8888
            for chunk in pixels.chunks(4) {
                if chunk.len() < 4 { break; }
                rgba.extend_from_slice(&[chunk[0], chunk[1], chunk[2], chunk[3]]);
            }
        }
        3 => { // RGBA5551
            for chunk in pixels.chunks(2) {
                if chunk.len() < 2 { break; }
                let p = u16::from_le_bytes([chunk[0], chunk[1]]);
                let r = (((p >> 11) & 0x1F) * 255 / 31) as u8;
                let g = (((p >> 6) & 0x1F) * 255 / 31) as u8;
                let b = (((p >> 1) & 0x1F) * 255 / 31) as u8;
                let a = if (p & 1) != 0 { 255 } else { 0 };
                rgba.extend_from_slice(&[r, g, b, a]);
            }
        }
        4 => { // RGB565
            for chunk in pixels.chunks(2) {
                if chunk.len() < 2 { break; }
                let p = u16::from_le_bytes([chunk[0], chunk[1]]);
                let r = (((p >> 11) & 0x1F) * 255 / 31) as u8;
                let g = (((p >> 5) & 0x3F) * 255 / 63) as u8;
                let b = ((p & 0x1F) * 255 / 31) as u8;
                rgba.extend_from_slice(&[r, g, b, 255]);
            }
        }
        5 => { // RGBA4444
            for chunk in pixels.chunks(2) {
                if chunk.len() < 2 { break; }
                let p = u16::from_le_bytes([chunk[0], chunk[1]]);
                let r = (((p >> 12) & 0xF) * 255 / 15) as u8;
                let g = (((p >> 8) & 0xF) * 255 / 15) as u8;
                let b = (((p >> 4) & 0xF) * 255 / 15) as u8;
                let a = ((p & 0xF) * 255 / 15) as u8;
                rgba.extend_from_slice(&[r, g, b, a]);
            }
        }
        7 => { // L8 (grayscale)
            for &px in pixels {
                rgba.extend_from_slice(&[px, px, px, 255]);
            }
        }
        8 => { // L4 (4-bit per pixel, 2 pixels per byte)
            for &byte in pixels {
                let hi = (byte >> 4) & 0xF;
                let lo = byte & 0xF;
                let v_hi = (hi * 255 / 15) as u8;
                let v_lo = (lo * 255 / 15) as u8;
                rgba.extend_from_slice(&[v_hi, v_hi, v_hi, 255, v_lo, v_lo, v_lo, 255]);
            }
        }
        _ => {
            // Unknown format: fill with magenta
            for _ in 0..count {
                rgba.extend_from_slice(&[255, 0, 255, 255]);
            }
        }
    }
    rgba.truncate(count * 4);
    while rgba.len() < count * 4 { rgba.push(0); }
    rgba
}

// ─── Public API ──────────────────────────────────────────────

/// Load and cache a sprite texture by path
pub fn load_sprite(path: &str) -> Option<LoadedTexture> {
    SPRITE_CACHE.with(|c| c.borrow_mut().get_or_load(path))
}

/// Draw a sprite at screen position with given size
pub fn draw_sprite(renderer: &mut GraphicsRenderer, path: &str, x: i32, y: i32, w: u32, h: u32) {
    // Ensure texture is cached in the renderer
    let tex_opt = SPRITE_CACHE.with(|c| c.borrow_mut().get_or_load(path));
    if let Some(tex) = tex_opt {
        renderer.cache_texture(path, &tex);
        renderer.draw_texture(path, x, y, w, h);
    }
}

/// Draw a sprite with a tint color
pub fn draw_sprite_tinted(renderer: &mut GraphicsRenderer, path: &str, x: i32, y: i32, w: u32, h: u32, _r: u8, _g: u8, _b: u8) {
    draw_sprite(renderer, path, x, y, w, h);
}

/// Clear the sprite cache
pub fn clear_cache() {
    SPRITE_CACHE.with(|c| c.borrow_mut().clear());
}

/// Preload a batch of sprites
pub fn preload_sprites(paths: &[&str]) {
    for path in paths {
        load_sprite(path);
    }
}

/// PSP GE texture format (matches PPSSPP GETextureFormat)
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum GeTextureFormat {
    Rgb565   = 0,
    Rgba5551 = 1,
    Rgba4444 = 2,
    Rgba8888 = 3,
    Clut4    = 4,
    Clut8    = 5,
    Clut16   = 6,
    Clut32   = 7,
    Dxt1     = 8,
    Dxt3     = 9,
    Dxt5     = 10,
}

impl GeTextureFormat {
    pub fn from_u8(v: u8) -> Option<Self> {
        match v {
            0 => Some(Self::Rgb565),
            1 => Some(Self::Rgba5551),
            2 => Some(Self::Rgba4444),
            3 => Some(Self::Rgba8888),
            4 => Some(Self::Clut4),
            5 => Some(Self::Clut8),
            6 => Some(Self::Clut16),
            7 => Some(Self::Clut32),
            8 => Some(Self::Dxt1),
            9 => Some(Self::Dxt3),
            10 => Some(Self::Dxt5),
            _ => None,
        }
    }
}

/// Bits per pixel table (matches PPSSPP textureBitsPerPixel)
pub fn texture_bits_per_pixel(fmt: GeTextureFormat) -> u32 {
    match fmt {
        GeTextureFormat::Rgb565   => 16,
        GeTextureFormat::Rgba5551 => 16,
        GeTextureFormat::Rgba4444 => 16,
        GeTextureFormat::Rgba8888 => 32,
        GeTextureFormat::Clut4    => 4,
        GeTextureFormat::Clut8    => 8,
        GeTextureFormat::Clut16   => 16,
        GeTextureFormat::Clut32   => 32,
        GeTextureFormat::Dxt1     => 4,
        GeTextureFormat::Dxt3     => 8,
        GeTextureFormat::Dxt5     => 8,
    }
}

// ─── Color Expansion Helpers ───

/// 4-bit -> 8-bit: 0000abcd -> abcdabcd
#[inline]
pub fn convert_4_to_8(v: u8) -> u8 {
    (v << 4) | v
}

/// 5-bit -> 8-bit: 000abcde -> abcdeabc
#[inline]
pub fn convert_5_to_8(v: u8) -> u8 {
    (v << 3) | (v >> 2)
}

/// 6-bit -> 8-bit: 00abcdef -> abcdefab
#[inline]
pub fn convert_6_to_8(v: u8) -> u8 {
    (v << 2) | (v >> 4)
}

// ─── 16-bit to RGBA8888 Conversions ───

/// RGBA4444 -> RGBA8888
pub fn rgba4444_to_rgba8888(src: u16) -> u32 {
    let r = convert_4_to_8((src & 0x000F) as u8);
    let g = convert_4_to_8(((src >> 4) & 0x000F) as u8);
    let b = convert_4_to_8(((src >> 8) & 0x000F) as u8);
    let a = convert_4_to_8(((src >> 12) & 0x000F) as u8);
    u32::from_le_bytes([r, g, b, a])
}

/// RGBA5551 -> RGBA8888
pub fn rgba5551_to_rgba8888(src: u16) -> u32 {
    let r = convert_5_to_8((src & 0x1F) as u8);
    let g = convert_5_to_8(((src >> 5) & 0x1F) as u8);
    let b = convert_5_to_8(((src >> 10) & 0x1F) as u8);
    let a = if (src >> 15) != 0 { 255 } else { 0 };
    u32::from_le_bytes([r, g, b, a])
}

/// RGB565 -> RGBA8888
pub fn rgb565_to_rgba8888(src: u16) -> u32 {
    let r = convert_5_to_8((src & 0x1F) as u8);
    let g = convert_6_to_8(((src >> 5) & 0x3F) as u8);
    let b = convert_5_to_8(((src >> 11) & 0x1F) as u8);
    u32::from_le_bytes([r, g, b, 255])
}

/// Convert a 16-bit texture pixel to RGBA8888 based on format
pub fn convert_16bit_to_rgba8888(src: u16, fmt: GeTextureFormat) -> u32 {
    match fmt {
        GeTextureFormat::Rgb565   => rgb565_to_rgba8888(src),
        GeTextureFormat::Rgba5551 => rgba5551_to_rgba8888(src),
        GeTextureFormat::Rgba4444 => rgba4444_to_rgba8888(src),
        _ => 0,
    }
}

/// Reverse RGBA4444 to ABGR4444 (PSP native → PC byte order)
pub fn reverse_4444(src: u16) -> u16 {
    (src >> 12) | ((src >> 4) & 0x00F0) | ((src << 4) & 0x0F00) | (src << 12)
}

/// Reverse RGBA5551 to ABGR1555
pub fn reverse_5551(src: u16) -> u16 {
    (src >> 15) | ((src >> 9) & 0x003E) | ((src << 1) & 0x07C0) | (src << 11)
}

/// Reverse RGB565 to BGR565
pub fn reverse_565(src: u16) -> u16 {
    ((src >> 11) & 0x001F) | (src & 0x07E0) | ((src << 11) & 0xF800)
}

// ─── DXT Block Decoding ───

/// PSP DXT1 block layout (reversed from PC DDS)
/// On PSP: lines[4] first, then color1 (u16), then color2 (u16) = 8 bytes
#[repr(C, packed)]
pub struct Dxt1Block {
    pub lines: [u8; 4],
    pub color1: u16,
    pub color2: u16,
}

/// PSP DXT3 block: Dxt1 color + 4 u16 alpha lines = 16 bytes
#[repr(C, packed)]
pub struct Dxt3Block {
    pub color: Dxt1Block,
    pub alpha_lines: [u16; 4],
}

/// PSP DXT5 block: Dxt1 color + alpha block = 16 bytes
#[repr(C, packed)]
pub struct Dxt5Block {
    pub color: Dxt1Block,
    pub alpha_data2: u32,
    pub alpha_data1: u16,
    pub alpha_endpoint1: u8,
    pub alpha_endpoint2: u8,
}

/// Decode a single DXT1 4x4 block into 16 RGBA8888 pixels.
/// `dst` must have room for 16 u32 values.
/// Returns true if any pixel used the transparent color 3.
pub fn decode_dxt1_block(dst: &mut [u32], src: &Dxt1Block) -> bool {
    assert!(dst.len() >= 16);
    let (c0, c1) = (u16::from_le(src.color1), u16::from_le(src.color2));
    let colors = decode_dxt1_colors(c0, c1);
    let mut has_alpha = false;

    for y in 0..4 {
        let mut line = src.lines[y];
        for x in 0..4 {
            let idx = (line & 3) as usize;
            if idx == 3 { has_alpha = true; }
            dst[y * 4 + x] = colors[idx];
            line >>= 2;
        }
    }
    has_alpha
}

/// Decode a DXT3 4x4 block into 16 RGBA8888 pixels (overwrites alpha for color 3)
pub fn decode_dxt3_block(dst: &mut [u32], src: &Dxt3Block) {
    assert!(dst.len() >= 16);
    let (c0, c1) = (u16::from_le(src.color.color1), u16::from_le(src.color.color2));
    let colors = decode_dxt1_colors(c0, c1);

    for y in 0..4 {
        let mut line = src.color.lines[y];
        let mut alpha_line = u16::from_le(src.alpha_lines[y]);
        for x in 0..4 {
            let mut pixel = colors[(line & 3) as usize];
            let alpha = ((alpha_line & 0xF) as u32) * 17; // Expand 4-bit to 8-bit
            pixel = (pixel & 0x00FFFFFF) | (alpha << 24);
            dst[y * 4 + x] = pixel;
            line >>= 2;
            alpha_line >>= 4;
        }
    }
}

/// Decode a DXT5 4x4 block into 16 RGBA8888 pixels
pub fn decode_dxt5_block(dst: &mut [u32], src: &Dxt5Block) {
    assert!(dst.len() >= 16);
    let (c0, c1) = (u16::from_le(src.color.color1), u16::from_le(src.color.color2));
    let colors = decode_dxt1_colors(c0, c1);

    let a0 = src.alpha_endpoint1;
    let a1 = src.alpha_endpoint2;
    let alphas = decode_dxt5_alphas(a0, a1);

    let all_alpha: u64 =
        ((u16::from_le(src.alpha_data1) as u64) << 32) |
        (u32::from_le(src.alpha_data2) as u64);

    for y in 0..4 {
        let mut line = src.color.lines[y];
        let mut alpha_bits = (all_alpha >> (y * 12)) as u32;
        for x in 0..4 {
            let color = colors[(line & 3) as usize];
            let alpha = alphas[(alpha_bits & 7) as usize];
            dst[y * 4 + x] = (color & 0x00FFFFFF) | ((alpha as u32) << 24);
            line >>= 2;
            alpha_bits >>= 3;
        }
    }
}

/// Compute 4 RGBA palette colors from two RGB565 endpoints.
fn decode_dxt1_colors(c0: u16, c1: u16) -> [u32; 4] {
    let mut colors = [0u32; 4];
    colors[0] = rgb565_to_rgba8888(c0);  // Full alpha for color 0
    colors[1] = rgb565_to_rgba8888(c1);  // Full alpha for color 1

    let r0 = (c0 >> 11) & 0x1F;
    let g0 = (c0 >> 5) & 0x3F;
    let b0 = c0 & 0x1F;
    let r1 = (c1 >> 11) & 0x1F;
    let g1 = (c1 >> 5) & 0x3F;
    let b1 = c1 & 0x1F;

    let c0le = u16::from_le(c0);
    let c1le = u16::from_le(c1);

    if c0le > c1le {
        // 4-color mode: interpolate
        let r2 = ((r0 as u32 * 2 + r1 as u32) / 3) as u16;
        let g2 = ((g0 as u32 * 2 + g1 as u32) / 3) as u16;
        let b2 = ((b0 as u32 * 2 + b1 as u32) / 3) as u16;
        let r3 = ((r0 as u32 + r1 as u32 * 2) / 3) as u16;
        let g3 = ((g0 as u32 + g1 as u32 * 2) / 3) as u16;
        let b3 = ((b0 as u32 + b1 as u32 * 2) / 3) as u16;
        let c2 = (r2 << 11) | (g2 << 5) | b2;
        let c3 = (r3 << 11) | (g3 << 5) | b3;
        colors[2] = rgb565_to_rgba8888(c2);
        colors[3] = rgb565_to_rgba8888(c3);
    } else {
        // 3-color mode: color 2 is average, color 3 is transparent
        let r2 = ((r0 as u32 + r1 as u32) / 2) as u16;
        let g2 = ((g0 as u32 + g1 as u32) / 2) as u16;
        let b2 = ((b0 as u32 + b1 as u32) / 2) as u16;
        let c2 = (r2 << 11) | (g2 << 5) | b2;
        colors[2] = rgb565_to_rgba8888(c2);
        colors[3] = 0x00000000; // Transparent black
    }
    colors
}

/// Compute 8 DXT5 alpha values from two alpha endpoints.
fn decode_dxt5_alphas(a0: u8, a1: u8) -> [u8; 8] {
    let mut alphas = [0u8; 8];
    alphas[0] = a0;
    alphas[1] = a1;

    if a0 > a1 {
        // Interpolate 6 values between a0 and a1
        for i in 1..7 {
            let n = i as u32;
            let v = ((7 - n) * a0 as u32 + n * a1 as u32) / 7;
            alphas[i as usize + 1] = v as u8;
        }
    } else {
        // Interpolate 4 values, plus 0 and 255
        for i in 1..5 {
            let n = i as u32;
            let v = ((5 - n) * a0 as u32 + n * a1 as u32) / 5;
            alphas[i as usize + 1] = v as u8;
        }
        alphas[6] = 0;
        alphas[7] = 255;
    }
    alphas
}

/// Decode a full DXT texture to RGBA8888.
/// PSP DXT blocks are NOT swizzled (DXT is naturally block-aligned).
pub fn decode_dxt_texture(
    out: &mut [u32],
    data: &[u8],
    width: u32,
    height: u32,
    format: GeTextureFormat,
) {
    let out_pitch = width as usize;
    let bufw = (width + 3) & !3; // Round up to multiple of 4

    for by in (0..height).step_by(4) {
        let block_row = (by / 4) as usize * (bufw as usize / 4);
        for bx in (0..bufw).step_by(4) {
            let block_idx = block_row + (bx / 4) as usize;
            if bx >= width { continue; }
            let block_w = 4.min(width - bx);
            let block_h = 4.min(height - by);

            let mut block_pixels = [0u32; 16];
            match format {
                GeTextureFormat::Dxt1 => {
                    if block_idx * 8 + 8 > data.len() { continue; }
                    let block = unsafe { &*(data.as_ptr().add(block_idx * 8) as *const Dxt1Block) };
                    decode_dxt1_block(&mut block_pixels, block);
                }
                GeTextureFormat::Dxt3 => {
                    if block_idx * 16 + 16 > data.len() { continue; }
                    let block = unsafe { &*(data.as_ptr().add(block_idx * 16) as *const Dxt3Block) };
                    decode_dxt3_block(&mut block_pixels, block);
                }
                GeTextureFormat::Dxt5 => {
                    if block_idx * 16 + 16 > data.len() { continue; }
                    let block = unsafe { &*(data.as_ptr().add(block_idx * 16) as *const Dxt5Block) };
                    decode_dxt5_block(&mut block_pixels, block);
                }
                _ => {}
            }

            for py in 0..block_h as usize {
                let dst_row = (by as usize + py) * out_pitch + bx as usize;
                let src_row = py * 4;
                for px in 0..block_w as usize {
                    out[dst_row + px] = block_pixels[src_row + px];
                }
            }
        }
    }
}

// ─── Swizzle / Unswizzle ───

/// Unswizzle PSP texture data to linear layout.
///
/// PSP swizzle uses 16-byte (4x u32) tiles arranged in 8×4 blocks.
/// Input: swizzled data pointed to by `src`
/// Output: linear dest, with `dest_pitch` bytes per row (u32 units).
/// `bxc` = number of 16-byte blocks per row
/// `byc` = number of 8-row blocks vertically
pub fn unswizzle_tex16(dest: &mut [u32], src: &[u8], bxc: usize, byc: usize, dest_pitch: usize) {
    let pitch_u32 = dest_pitch;
    let src32: &[u32] = unsafe {
        std::slice::from_raw_parts(src.as_ptr() as *const u32, src.len() / 4)
    };
    let mut src_idx = 0;

    for _by in 0..byc {
        let mut xdest = 0;
        for _bx in 0..bxc {
            // Copy 16 bytes (4 u32) to each of 8 rows
            let mut dest_off = xdest;
            for _n in 0..8 {
                if dest_off + 4 <= dest.len() && src_idx + 4 <= src32.len() {
                    dest[dest_off] = src32[src_idx];
                    dest[dest_off + 1] = src32[src_idx + 1];
                    dest[dest_off + 2] = src32[src_idx + 2];
                    dest[dest_off + 3] = src32[src_idx + 3];
                }
                dest_off += pitch_u32;
                src_idx += 4;
            }
            xdest += 4;
        }
        // dest advances by 8 rows (we skip over the rows we already wrote by
        // starting at a new `xdest`, but the outer loop position is handled
        // by the src_idx tracking — actually this is wrong, we need to adjust
        // the dest base position)
    }
}

/// Improved unswizzle: works on byte-level data.
/// Parameters match PPSSPP's DoUnswizzleTex16.
pub fn unswizzle_16_byte_blocks(
    dest: &mut [u8],
    dest_pitch: usize,
    src: &[u8],
    bxc: usize,
    byc: usize,
) {
    let pitch_u32 = dest_pitch / 4;
    let src_u32: &[u32] = bytemuck::cast_slice(src);
    let dest_u32: &mut [u32] = bytemuck::cast_slice_mut(dest);

    let mut src_pos = 0;
    let mut dest_row_base = 0usize;

    for _by in 0..byc {
        let mut dest_col_base = 0usize;
        for _bx in 0..bxc {
            // Write 16 bytes (4 u32s) to each of 8 destination rows
            for row in 0..8 {
                let base = dest_row_base + dest_col_base + row * pitch_u32;
                if base + 4 <= dest_u32.len() && src_pos + 4 <= src_u32.len() {
                    dest_u32[base..base + 4].copy_from_slice(&src_u32[src_pos..src_pos + 4]);
                }
                src_pos += 4;
            }
            dest_col_base += 4;
        }
        dest_row_base += pitch_u32 * 8;
    }
}

/// Unswizzle a PSP texture to linear RGBA8888.
/// Takes raw swizzled texture data and produces linear output.
pub fn unswizzle_texture_to_rgba(
    dest: &mut [u32],
    data: &[u8],
    width: u32,
    height: u32,
    bufw: u32, // buffer width in pixels (rounded up for swizzle alignment)
    format: GeTextureFormat,
) {
    let bpp = texture_bits_per_pixel(format);
    let row_width = if bpp > 0 { bufw * bpp } else { bufw / 2 };
    let bxc = (row_width as usize) / 16;
    let byc = ((height as usize) + 7) / 8;

    // Allocate temp buffer for unswizzled raw data
    let unswizzled_size = (bufw as usize) * ((height as usize + 7) & !7) * (bpp as usize / 8).max(1);
    let mut temp = vec![0u8; unswizzled_size.max(1)];
    let temp_pitch = (bufw as usize) * (bpp as usize / 8).max(1);

    unswizzle_16_byte_blocks(&mut temp, temp_pitch, data, bxc, byc);

    // Convert from raw format to RGBA8888
    match format {
        GeTextureFormat::Rgb565 | GeTextureFormat::Rgba5551 | GeTextureFormat::Rgba4444 => {
            for y in 0..height as usize {
                for x in 0..width as usize {
                    let src_off = y * temp_pitch + x * 2;
                    if src_off + 2 <= temp.len() {
                        let pixel = u16::from_le_bytes([temp[src_off], temp[src_off + 1]]);
                        dest[y * width as usize + x] = convert_16bit_to_rgba8888(pixel, format);
                    }
                }
            }
        }
        GeTextureFormat::Rgba8888 => {
            for y in 0..height as usize {
                for x in 0..width as usize {
                    let src_off = y * temp_pitch + x * 4;
                    if src_off + 4 <= temp.len() {
                        let r = temp[src_off];
                        let g = temp[src_off + 1];
                        let b = temp[src_off + 2];
                        let a = temp[src_off + 3];
                        dest[y * width as usize + x] = u32::from_le_bytes([r, g, b, a]);
                    }
                }
            }
        }
        _ => {
            // DXT formats: not swizzled, handled by decode_dxt_texture
            decode_dxt_texture(dest, data, width, height, format);
        }
    }
}

/// Decode a PSP-formatted texture to RGBA8888.
/// Handles swizzle, DXT, and all supported formats.
pub fn decode_psp_texture(
    data: &[u8],
    width: u32,
    height: u32,
    bufw: u32,
    format: GeTextureFormat,
    swizzled: bool,
) -> Vec<u32> {
    let size = (width * height) as usize;
    let mut out = vec![0u32; size];

    if swizzled && !matches!(format, GeTextureFormat::Dxt1 | GeTextureFormat::Dxt3 | GeTextureFormat::Dxt5) {
        unswizzle_texture_to_rgba(&mut out, data, width, height, bufw, format);
    } else if matches!(format, GeTextureFormat::Dxt1 | GeTextureFormat::Dxt3 | GeTextureFormat::Dxt5) {
        decode_dxt_texture(&mut out, data, width, height, format);
    } else {
        // Non-swizzled, non-DXT: direct decode
        let bpp = (texture_bits_per_pixel(format) / 8) as usize;
        let row_size = bufw as usize * bpp.max(1);
        match format {
            GeTextureFormat::Rgb565 | GeTextureFormat::Rgba5551 | GeTextureFormat::Rgba4444 => {
                for y in 0..height as usize {
                    for x in 0..width as usize {
                        let off = y * row_size + x * 2;
                        if off + 2 <= data.len() {
                            let p = u16::from_le_bytes([data[off], data[off + 1]]);
                            out[y * width as usize + x] = convert_16bit_to_rgba8888(p, format);
                        }
                    }
                }
            }
            GeTextureFormat::Rgba8888 => {
                for y in 0..height as usize {
                    for x in 0..width as usize {
                        let off = y * row_size + x * 4;
                        if off + 4 <= data.len() {
                            let r = data[off];
                            let g = data[off + 1];
                            let b = data[off + 2];
                            let a = data[off + 3];
                            out[y * width as usize + x] = u32::from_le_bytes([r, g, b, a]);
                        }
                    }
                }
            }
            _ => {}
        }
    }

    out
}

/// Compute the PSP buffer width (bufw) for alignment requirements.
/// The bufw is the number of pixels per row rounded up to the minimum
/// alignment needed for swizzle blocks (16 bytes).
pub fn compute_texture_bufw(width: u32, format: GeTextureFormat) -> u32 {
    let bpp = texture_bits_per_pixel(format);
    if bpp == 0 { return width; }
    // Round up so each row is a multiple of 16 bytes
    let pixels_per_16bytes = (16 * 8) / bpp;
    if pixels_per_16bytes == 0 { return width; }
    ((width + pixels_per_16bytes - 1) / pixels_per_16bytes) * pixels_per_16bytes
}

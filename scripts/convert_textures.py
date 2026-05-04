#!/usr/bin/env python3
"""GT PSP Texture Conversion Tool - Bidirectional TXS3/PNG converter.

Based on reverse-engineering findings:
- TXS3 format with '3SXT' magic (little-endian) or 'TXS3' (big-endian)
- ImageInfo struct at img_ptr (from header offset 0x1C)
  - Bytes 0-3: data_ptr (absolute offset to pixel data)
  - Bytes 4-7: data_size
  - Byte 8: flags (0x08 = swizzled, 0x00 = linear)
  - Byte 9: format ID
  - Byte 10: mipmap count
  - Byte 11: unknown
  - Bytes 12-13: width (uint16)
  - Bytes 14-15: height (uint16)
  - Bytes 16-31: zero/unused (reserved for mip chain)

Format IDs:
  0x05 = PSMT8 (8-bit paletted, 256 colors, swizzled with 16x8 blocks)
         - Width padded to next power of 2
         - Height stays as-is
         - Palette: 1024 bytes RGBA8888 in trailing data after pixel data
         - Trailing header: [2 bytes: pal_size_words] [2 bytes: num_colors] [4 bytes: pal_ptr] [8 bytes: zero]
         - Palette offset in trailing: 0x10
  0x08 = PSMT4 (4-bit paletted, 16 colors, linear/unswizzled)
         - Width padded to next power of 2
         - Height stays as-is
         - Palette: 64 bytes in trailing data
         - No additional trailing header before palette (palette starts at offset 0)
  0x04 = RGB565 (raw 16-bit RGB)
  0x03 = RGBA5551 (raw 16-bit RGBA)
  0x01 = RGBA8888 (raw 32-bit RGBA)
  0x07 = L8 (8-bit grayscale)
  0x09 = A8 (8-bit alpha)

Note: The raw formats (RGB565, RGBA5551, etc.) are stored with power-of-2
dimensions calculated from data_size / bpp. The header dimensions may be wrong.
Paletted formats use the header dimensions as truth (with width padded to pow2).
"""

import os
import struct
import io
import numpy as np
from pathlib import Path
from typing import Dict, Optional, Tuple, List
from enum import IntEnum

try:
    from PIL import Image
    HAS_PIL = True
except ImportError:
    HAS_PIL = False
    print("Warning: PIL/Pillow not installed.")


class TextureFormat(IntEnum):
    RGBA8888 = 0x01
    RGB888 = 0x02
    RGBA5551 = 0x03
    PSMT4_SWIZZLED = 0x04   # 4-bit paletted, 16 colors, swizzled, palette at trailing[0x10]
    PSMT8 = 0x05
    LA88 = 0x06
    L8 = 0x07
    PSMT4 = 0x08            # 4-bit paletted, linear (no swizzle), DDS palette filename in trailing
    A8 = 0x09
    DXT1 = 0x0A
    DXT3 = 0x0B
    DXT5 = 0x0C
    BC7 = 0x1B


# BPP lookup: format_byte -> bytes_per_pixel
FORMAT_BPP = {
    1: 4,    # RGBA8888
    2: 3,    # RGB888
    3: 2,    # RGBA5551
    4: 0.5,  # PSMT4_SWIZZLED (4-bit indexed)
    5: 1,    # PSMT8 (8-bit indexed)
    6: 2,    # LA88
    7: 1,    # L8
    8: 0.5,  # PSMT4 (4-bit indexed)
    9: 1,    # A8
}

# Formats that are paletted (indexed) — palette follows pixel data
PALETTED_FORMATS = {0x04, 0x05, 0x08}

# Swizzle block sizes per format (used when byte 8 & 0x08)
SWIZZLE_BLOCKS = {
    0x04: (16, 8),   # PSMT4_SWIZZLED: 16x8 pixel blocks
    0x05: (16, 8),   # PSMT8: 16x8 pixel blocks
    0x03: (16, 8),   # RGBA5551 swizzled: 16x8 pixel blocks (16-bit per pixel)
    0x08: None,       # PSMT4: typically linear (no swizzle) in these files
}


class TXS3Converter:
    """Bidirectional converter between TXS3 (GT PSP texture format) and PNG."""

    SUPPORTED_DECODE_FORMATS = {
        TextureFormat.RGBA5551: ('RGBA', 2),
        TextureFormat.PSMT4_SWIZZLED: ('RGBA', 0.5),
        TextureFormat.PSMT8: ('RGBA', 1),
        TextureFormat.PSMT4: ('RGBA', 0.5),
        TextureFormat.RGBA8888: ('RGBA', 4),
        TextureFormat.L8: ('L', 1),
    }

    @staticmethod
    def is_txs3_file(filepath: str) -> bool:
        try:
            with open(filepath, 'rb') as f:
                magic = f.read(4)
                return magic in [b'TXS3', b'3SXT']
        except:
            return False

    @staticmethod
    def _find_actual_dimensions(hdr_w: int, hdr_h: int, data_size: int, fmt_id: int) -> Tuple[int, int, int, int]:
        """Returns (real_width, real_height, padded_width, padded_height)."""
        if fmt_id in PALETTED_FORMATS:
            bpp = FORMAT_BPP.get(fmt_id, 1)
            exact_pixels = int(data_size / bpp)
            if hdr_w * hdr_h == exact_pixels:
                return hdr_w, hdr_h, hdr_w, hdr_h
            padded_w = 1 << (hdr_w - 1).bit_length()
            padded_h = hdr_h
            expected = int(padded_w * padded_h * bpp)
            if expected == data_size:
                return hdr_w, hdr_h, padded_w, padded_h
            padded_h_alt = int(data_size / bpp / padded_w)
            if padded_h_alt > 0 and int(padded_w * padded_h_alt * bpp) == data_size:
                return hdr_w, hdr_h, padded_w, padded_h_alt
            return hdr_w, hdr_h, hdr_w, hdr_h

        bpp = FORMAT_BPP.get(fmt_id, 2)
        if bpp == 0:
            return hdr_w, hdr_h, hdr_w, hdr_h

        pixel_count = int(data_size / bpp)

        if hdr_w * hdr_h == pixel_count:
            return hdr_w, hdr_h, hdr_w, hdr_h

        best_w, best_h = hdr_w, hdr_h
        best_dist = abs(hdr_w * hdr_h - pixel_count)

        for tw in range(1, 4097):
            if pixel_count % tw == 0:
                th = pixel_count // tw
                if th > 4096:
                    continue
                dist = abs(tw - hdr_w) + abs(th - hdr_h)
                if dist < best_dist:
                    best_w, best_h = tw, th
                    best_dist = dist

        return best_w, best_h, best_w, best_h

    @staticmethod
    def parse_header(filepath: str) -> Optional[Dict]:
        try:
            with open(filepath, 'rb') as f:
                magic = f.read(4)
                if magic not in [b'TXS3', b'3SXT']:
                    return None

                endian = 'little' if magic == b'3SXT' else 'big'
                fmt = '<' if endian == 'little' else '>'

                file_size = struct.unpack(fmt + 'I', f.read(4))[0]
                f.read(4)
                f.read(4)
                f.read(4)
                pglue_count = struct.unpack(fmt + 'H', f.read(2))[0]
                img_count = struct.unpack(fmt + 'H', f.read(2))[0]
                pglue_ptr = struct.unpack(fmt + 'I', f.read(4))[0]
                img_ptr = struct.unpack(fmt + 'I', f.read(4))[0]

            actual_file_size = os.path.getsize(filepath)

            with open(filepath, 'rb') as f:
                f.seek(img_ptr)
                img_info = f.read(32)

            data_ptr = struct.unpack(fmt + 'I', img_info[0:4])[0]
            data_size = struct.unpack(fmt + 'I', img_info[4:8])[0]
            byte8 = img_info[8]
            format_id = img_info[9]
            mipmaps = img_info[10]
            hdr_w = struct.unpack(fmt + 'H', img_info[12:14])[0]
            hdr_h = struct.unpack(fmt + 'H', img_info[14:16])[0]

            is_swizzled = bool(byte8 & 0x08)

            real_w, real_h, padded_w, padded_h = TXS3Converter._find_actual_dimensions(
                hdr_w, hdr_h, data_size, format_id
            )

            has_padding = False
            actual_data_ptr = data_ptr

            return {
                'magic': magic.decode('ascii', errors='ignore'),
                'endian': endian,
                'file_size': file_size,
                'actual_file_size': actual_file_size,
                'pglue_count': pglue_count,
                'img_count': img_count,
                'pglue_ptr': pglue_ptr,
                'img_ptr': img_ptr,
                'data_ptr': data_ptr,
                'actual_data_ptr': actual_data_ptr,
                'data_size': data_size,
                'format_id': format_id,
                'format_name': TextureFormat(format_id).name if format_id in TextureFormat.__members__.values() else f'Unknown_{format_id:02X}',
                'mipmaps': mipmaps,
                'hdr_w': hdr_w,
                'hdr_h': hdr_h,
                'width': real_w,
                'height': real_h,
                'padded_w': padded_w,
                'padded_h': padded_h,
                'has_padding': has_padding,
                'is_swizzled': is_swizzled,
                'valid': (real_w > 0 and real_h > 0 and real_w <= 4096 and real_h <= 4096)
            }
        except Exception as e:
            print(f"Error parsing header: {e}")
            return None

    @staticmethod
    def _deswizzle_psmt8(data: bytes, padded_w: int, padded_h: int) -> np.ndarray:
        sb_w, sb_h = 16, 8
        output = np.zeros((padded_h, padded_w), dtype=np.uint8)
        idx = 0
        for sy in range(0, padded_h, sb_h):
            for sx in range(0, padded_w, sb_w):
                wb = min(sb_w, padded_w - sx)
                hb = min(sb_h, padded_h - sy)
                for by in range(hb):
                    for bx in range(wb):
                        output[sy + by, sx + bx] = data[idx]
                        idx += 1
        return output

    @staticmethod
    def _find_palette(data: bytes, data_ptr: int, data_size: int, fmt_id: int) -> Optional[np.ndarray]:
        trailing = data[data_ptr + data_size:]
        if fmt_id in (0x04, 0x05):
            if len(trailing) < 16:
                return None
            num_colors = struct.unpack('<H', trailing[2:4])[0]
            if num_colors < 1 or num_colors > 256:
                num_colors = 256
            pal_data_len = num_colors * 4
            if len(trailing) < 0x10 + pal_data_len:
                pal_data_len = min(num_colors, 256) * 4
                if len(trailing) < 0x10 + pal_data_len:
                    return None
            pal_raw = trailing[0x10:0x10 + pal_data_len]
            palette = np.frombuffer(pal_raw, dtype=np.uint8).reshape(num_colors, 4)
            if num_colors < 256:
                full = np.zeros((256, 4), dtype=np.uint8)
                full[:num_colors] = palette
                return full
            return palette
        elif fmt_id == 0x08:
            trailing_str = trailing[:32].decode('ascii', errors='replace').rstrip('\x00')
            if '.dds' in trailing_str:
                return None
            if len(trailing) < 64:
                return None
            pal_raw = trailing[:64]
            return np.frombuffer(pal_raw, dtype=np.uint8).reshape(16, 4)
        return None

    @staticmethod
    def decode_psmt8(data: bytes, width: int, height: int, is_swizzled: bool,
                     palette: np.ndarray, padded_w: int, padded_h: int) -> np.ndarray:
        expected = padded_w * padded_h
        data = data.ljust(expected, b'\x00')[:expected]
        if is_swizzled:
            indices = TXS3Converter._deswizzle_psmt8(data, padded_w, padded_h)
        else:
            indices = np.frombuffer(data, dtype=np.uint8).reshape(padded_h, padded_w)
        result = palette[indices]
        return result[:height, :width]

    @staticmethod
    def decode_psmt4(data: bytes, width: int, height: int, palette: np.ndarray,
                     padded_w: int, padded_h: int) -> np.ndarray:
        expected = padded_w * padded_h // 2
        data = data.ljust(expected, b'\x00')[:expected]
        arr = np.frombuffer(data, dtype=np.uint8)
        high = (arr >> 4) & 0xF
        low = arr & 0xF
        pixels = np.empty(padded_h * padded_w, dtype=np.uint8)
        pixels[0::2] = high
        pixels[1::2] = low
        indices = pixels.reshape(padded_h, padded_w)
        result = palette[indices]
        return result[:height, :width]

    @staticmethod
    def decode_rgb565(data: bytes, width: int, height: int) -> np.ndarray:
        arr = np.frombuffer(data, dtype=np.uint16).reshape(height, width)
        r = ((arr >> 11) & 0x1F) * 255 // 31
        g = ((arr >> 5) & 0x3F) * 255 // 63
        b = (arr & 0x1F) * 255 // 31
        return np.stack([r, g, b], axis=-1).astype(np.uint8)

    @staticmethod
    def decode_rgba5551(data: bytes, width: int, height: int) -> np.ndarray:
        arr = np.frombuffer(data, dtype=np.uint16).reshape(height, width)
        r = ((arr >> 11) & 0x1F) * 255 // 31
        g = ((arr >> 6) & 0x1F) * 255 // 31
        b = ((arr >> 1) & 0x1F) * 255 // 31
        a = (arr & 0x1) * 255
        return np.stack([r, g, b, a], axis=-1).astype(np.uint8)

    @staticmethod
    def decode_l8(data: bytes, width: int, height: int) -> np.ndarray:
        expected = width * height
        data = data.ljust(expected, b'\x00')[:expected]
        gray = np.frombuffer(data, dtype=np.uint8).reshape(height, width)
        return np.stack([gray, gray, gray], axis=-1)

    @staticmethod
    def decode_rgba8888(data: bytes, width: int, height: int) -> np.ndarray:
        return np.frombuffer(data, dtype=np.uint8).reshape(height, width, 4)

    @staticmethod
    def _deswizzle_psmt4_swizzled(data: bytes, padded_w: int, padded_h: int,
                                   sb_w: int = 16, sb_h: int = 8) -> np.ndarray:
        expected = padded_w * padded_h // 2
        data = data.ljust(expected, b'\x00')[:expected]
        arr = np.frombuffer(data, dtype=np.uint8)
        high = (arr >> 4) & 0xF
        low = arr & 0xF
        pixels = np.empty(padded_h * padded_w, dtype=np.uint8)
        pixels[0::2] = high
        pixels[1::2] = low
        all_nibbles = pixels.reshape(padded_h, padded_w)
        output = np.zeros((padded_h, padded_w), dtype=np.uint8)
        idx = 0
        for sy in range(0, padded_h, sb_h):
            for sx in range(0, padded_w, sb_w):
                wb = min(sb_w, padded_w - sx)
                hb = min(sb_h, padded_h - sy)
                for by in range(hb):
                    for bx in range(wb):
                        if idx < padded_h * padded_w:
                            output[sy + by, sx + bx] = all_nibbles.ravel()[idx]
                        idx += 1
        return output

    @staticmethod
    def _apply_generic_swizzle(data: bytes, padded_w: int, padded_h: int,
                                sb_w: int, sb_h: int, bpp: float) -> np.ndarray:
        if bpp < 1:
            result_w = padded_w
            block_w = sb_w
        else:
            result_w = padded_w * int(bpp)
            block_w = sb_w * int(bpp)
        result = np.zeros((padded_h, result_w), dtype=np.uint8)
        block_bytes = block_w * sb_h
        idx = 0
        for sy in range(0, padded_h, sb_h):
            for sx in range(0, result_w, block_w):
                for by in range(min(sb_h, padded_h - sy)):
                    for bx in range(min(block_w, result_w - sx)):
                        if idx < len(data):
                            result[sy + by, sx + bx] = data[idx]
                        idx += 1
                        if idx >= len(data):
                            return result
        return result

    @staticmethod
    def decode_texture(data: bytes, width: int, height: int, format_id: int,
                       is_swizzled: bool = False,
                       palette: Optional[np.ndarray] = None,
                       padded_w: Optional[int] = None,
                       padded_h: Optional[int] = None) -> Optional[np.ndarray]:
        if format_id == 0x05:
            if palette is None:
                return None
            pw = padded_w if padded_w else 1 << (width - 1).bit_length()
            ph = padded_h if padded_h else height
            return TXS3Converter.decode_psmt8(data, width, height, is_swizzled, palette, pw, ph)
        elif format_id == 0x08:
            if palette is None:
                return None
            pw = padded_w if padded_w else 1 << (width - 1).bit_length()
            ph = padded_h if padded_h else height
            return TXS3Converter.decode_psmt4(data, width, height, palette, pw, ph)
        elif format_id == 0x04:
            if palette is None:
                return None
            pw = padded_w if padded_w else 1 << (width - 1).bit_length()
            ph = padded_h if padded_h else height
            if is_swizzled:
                sb_w, sb_h = SWIZZLE_BLOCKS.get(0x04, (16, 8))
                indices = TXS3Converter._deswizzle_psmt4_swizzled(data, pw, ph, sb_w, sb_h)
            else:
                arr = np.frombuffer(data, dtype=np.uint8).reshape(ph, pw // 2)
                high = (arr >> 4) & 0xF
                low = arr & 0xF
                pixels = np.empty(ph * pw, dtype=np.uint8)
                pixels[0::2] = high.ravel()
                pixels[1::2] = low.ravel()
                indices = pixels.reshape(ph, pw)
            result = palette[indices.astype(int)]
            return result[:height, :width]
        elif format_id == 0x03:
            if is_swizzled:
                pw = padded_w if padded_w else 1 << (width - 1).bit_length()
                ph = padded_h if padded_h else height
                sb_w, sb_h = SWIZZLE_BLOCKS.get(0x03, (16, 8))
                data_np = np.frombuffer(data, dtype=np.uint8)
                indices = TXS3Converter._apply_generic_swizzle(
                    data_np, pw, ph, sb_w, sb_h, 2)
                arr = np.frombuffer(indices.tobytes(), dtype=np.uint16).reshape(ph, pw)
            else:
                arr = np.frombuffer(data, dtype=np.uint16).reshape(height, width)
            r = ((arr >> 11) & 0x1F) * 255 // 31
            g = ((arr >> 6) & 0x1F) * 255 // 31
            b = ((arr >> 1) & 0x1F) * 255 // 31
            a = (arr & 0x1) * 255
            result = np.stack([r, g, b, a], axis=-1).astype(np.uint8)
            return result[:height, :width]
        elif format_id == 0x01:
            return TXS3Converter.decode_rgba8888(data, width, height)
        elif format_id == 0x07:
            return TXS3Converter.decode_l8(data, width, height)
        return None

    @staticmethod
    def convert_txs3_to_png(img_path: str, png_path: str, force: bool = False,
                            interactive: bool = False) -> bool:
        if not HAS_PIL:
            print("Error: PIL/Pillow required")
            return False

        if os.path.exists(png_path) and not force:
            return False

        try:
            header = TXS3Converter.parse_header(img_path)
            if not header or not header['valid']:
                print(f"Invalid header: {img_path}")
                return False

            fmt_id = header['format_id']
            width = header['width']
            height = header['height']
            padded_w = header['padded_w']
            padded_h = header['padded_h']
            hdr_w = header['hdr_w']
            hdr_h = header['hdr_h']
            data_size = header['data_size']
            data_ptr = header['data_ptr']
            actual_data_ptr = header['actual_data_ptr']
            is_swizzled = header['is_swizzled']

            with open(img_path, 'rb') as f:
                f.seek(actual_data_ptr)
                data = f.read(data_size)

            palette = None
            if fmt_id in PALETTED_FORMATS:
                with open(img_path, 'rb') as f:
                    all_data = f.read()
                palette = TXS3Converter._find_palette(all_data, data_ptr, data_size, fmt_id)
                if palette is None:
                    print(f"Could not find palette for {os.path.basename(img_path)}")
                    return False

            pixels = TXS3Converter.decode_texture(
                data, width, height, fmt_id, is_swizzled, palette, padded_w, padded_h
            )

            if pixels is None:
                print(f"Unsupported format: {header['format_name']}")
                return False

            img = Image.fromarray(pixels)
            os.makedirs(os.path.dirname(png_path), exist_ok=True)
            img.save(png_path, 'PNG')

            if interactive:
                dim_note = ""
                if hdr_w != width or hdr_h != height:
                    dim_note = f" (header said {hdr_w}x{hdr_h})"
                pad_note = " [padded]" if header['has_padding'] else ""
                swz_note = " [swizzled]" if is_swizzled else ""
                print(f"  Converted: {os.path.basename(png_path)}")
                print(f"    {width}x{height}{dim_note} {header['format_name']}{pad_note}{swz_note}")
                print(f"    File: {img_path}")
                ok = input(f"  Does this look correct? [Y/n/q]: ").strip().lower()
                if ok == 'q':
                    print("  Quitting conversion.")
                    return False
                elif ok == 'n':
                    print("  Marked as potentially corrupted.")
                    return False

            return True

        except Exception as e:
            print(f"Error converting {img_path}: {e}")
            return False

    @staticmethod
    def create_txs3_header(fmt: str, width: int, height: int, data_size: int,
                           endian: str = 'little') -> bytes:
        struct_fmt = '<' if endian == 'little' else '>'
        magic = b'3SXT' if endian == 'little' else b'TXS3'
        format_id = getattr(TextureFormat, fmt.upper(), TextureFormat.L8).value

        header = b''
        header += magic
        header += struct.pack(struct_fmt + 'I', 0x100 + data_size)
        header += struct.pack(struct_fmt + 'I', 0)
        header += struct.pack(struct_fmt + 'I', 0)
        header += struct.pack(struct_fmt + 'I', 0)
        header += struct.pack(struct_fmt + 'H', 1)
        header += struct.pack(struct_fmt + 'H', 1)
        header += struct.pack(struct_fmt + 'I', 0x40)
        header += struct.pack(struct_fmt + 'I', 0x100)

        header += b'\x00' * (0x100 - len(header))

        header += struct.pack(struct_fmt + 'I', 0x200)
        header += struct.pack(struct_fmt + 'I', data_size)
        header += b'\x08'
        header += struct.pack('B', format_id)
        header += b'\x01'
        header += b'\x00'
        header += struct.pack(struct_fmt + 'H', width)
        header += struct.pack(struct_fmt + 'H', height)

        header += b'\x00' * (0x200 - len(header))

        return header

    @staticmethod
    def convert_png_to_txs3(png_path: str, txs3_path: str, format_name: str = 'RGB565',
                           endian: str = 'little', force: bool = False) -> bool:
        if not HAS_PIL:
            print("Error: PIL/Pillow required")
            return False

        if os.path.exists(txs3_path) and not force:
            print(f"Output exists: {txs3_path}")
            return False

        try:
            img = Image.open(png_path)
            pixels = np.array(img)

            if len(pixels.shape) == 2:
                pixels = np.stack([pixels, pixels, pixels], axis=-1)

            if format_name.upper() == 'RGBA4444' and pixels.shape[2] == 3:
                a = np.full((pixels.shape[0], pixels.shape[1], 1), 255, dtype=np.uint8)
                pixels = np.concatenate([pixels, a], axis=-1)

            raw_data = TXS3Converter._encode_pixels(pixels, format_name)
            if raw_data is None:
                print(f"Unsupported format: {format_name}")
                return False

            width, height = pixels.shape[1], pixels.shape[0]
            # For PSMT8/PSMT4, pad width to power of 2
            if format_name.upper() in ('PSMT8', 'PSMT4'):
                padded_w = 1 << (width - 1).bit_length()
                padded_h = height
                expected = int(padded_w * padded_h * FORMAT_BPP.get(
                    getattr(TextureFormat, format_name.upper(), 0x05).value, 1))
                data_size = expected
            else:
                data_size = len(raw_data)

            header = TXS3Converter.create_txs3_header(
                format_name, width, height, data_size, endian
            )

            os.makedirs(os.path.dirname(txs3_path), exist_ok=True)
            with open(txs3_path, 'wb') as f:
                f.write(header)
                f.write(raw_data)
                # Write palette data if it's a paletted format

            print(f"Created: {txs3_path} ({width}x{height} {format_name})")
            return True

        except Exception as e:
            print(f"Error converting {png_path}: {e}")
            return False

    @staticmethod
    def _encode_pixels(pixels: np.ndarray, format_name: str) -> Optional[bytes]:
        fmt = format_name.upper()

        if fmt == 'RGB565':
            r = (pixels[:, :, 0].astype(np.uint16) * 31 // 255)
            g = (pixels[:, :, 1].astype(np.uint16) * 63 // 255)
            b = (pixels[:, :, 2].astype(np.uint16) * 31 // 255)
            arr = (r << 11) | (g << 5) | b
            return arr.astype(np.uint16).tobytes()

        elif fmt == 'RGBA5551':
            r = (pixels[:, :, 0].astype(np.uint16) * 31 // 255)
            g = (pixels[:, :, 1].astype(np.uint16) * 31 // 255)
            b = (pixels[:, :, 2].astype(np.uint16) * 31 // 255)
            a = (pixels[:, :, 3].astype(np.uint16) // 255)
            arr = (r << 11) | (g << 6) | (b << 1) | a
            return arr.astype(np.uint16).tobytes()

        elif fmt == 'RGBA4444':
            r = (pixels[:, :, 0].astype(np.uint16) // 17)
            g = (pixels[:, :, 1].astype(np.uint16) // 17)
            b = (pixels[:, :, 2].astype(np.uint16) // 17)
            a = (pixels[:, :, 3].astype(np.uint16) // 17)
            arr = (r << 12) | (g << 8) | (b << 4) | a
            return arr.astype(np.uint16).tobytes()

        elif fmt == 'L8':
            gray = (0.299 * pixels[:, :, 0] + 0.587 * pixels[:, :, 1] + 0.114 * pixels[:, :, 2])
            return gray.astype(np.uint8).tobytes()

        elif fmt == 'L4':
            gray = (0.299 * pixels[:, :, 0] + 0.587 * pixels[:, :, 1] + 0.114 * pixels[:, :, 2])
            gray = (gray / 17).astype(np.uint8)
            high = (gray >> 4) & 0xF
            low = gray & 0x0F
            packed = (high[:, 0::2] << 4) | low[:, 1::2]
            return packed.astype(np.uint8).tobytes()

        return None


def batch_convert(input_dir: str, output_dir: str, mode: str = 'txs3_to_png',
                  formats: List[str] = None, recursive: bool = True,
                  force: bool = False, interactive: bool = False):
    input_path = Path(input_dir)
    output_path = Path(output_dir)

    if mode == 'txs3_to_png':
        output_path.mkdir(parents=True, exist_ok=True)
        patterns = ['**/*.img', '**/*.txs', '**/*.txs3'] if recursive else ['*.img', '*.txs', '*.txs3']

        converted = 0
        failed = 0

        for pattern in patterns:
            for tex_file in input_path.glob(pattern):
                if tex_file.stat().st_size < 64:
                    continue
                if not TXS3Converter.is_txs3_file(str(tex_file)):
                    continue

                rel_path = tex_file.relative_to(input_path)
                png_file = output_path / rel_path.with_suffix('.png')
                png_file.parent.mkdir(parents=True, exist_ok=True)

                if TXS3Converter.convert_txs3_to_png(
                    str(tex_file), str(png_file), force=force, interactive=interactive
                ):
                    converted += 1
                else:
                    failed += 1

        print(f"\nTXS3 -> PNG: {converted} converted, {failed} failed")
        return converted, failed

    elif mode == 'png_to_txs3':
        output_path.mkdir(parents=True, exist_ok=True)
        format_name = formats[0].upper() if formats else 'RGB565'
        patterns = ['**/*.png'] if recursive else ['*.png']

        converted = 0
        failed = 0

        for pattern in patterns:
            for png_file in input_path.glob(pattern):
                rel_path = png_file.relative_to(input_path)
                txs3_file = output_path / rel_path.with_suffix('.img')
                txs3_file.parent.mkdir(parents=True, exist_ok=True)

                if TXS3Converter.convert_png_to_txs3(
                    str(png_file), str(txs3_file), format_name
                ):
                    converted += 1
                else:
                    failed += 1

        print(f"\nPNG -> TXS3: {converted} converted, {failed} failed")
        return converted, failed

    return 0, 0


def main():
    import argparse

    parser = argparse.ArgumentParser(description='GT PSP Texture Converter')
    parser.add_argument('mode', choices=['txs3_to_png', 'png_to_txs3', 'analyze', 'verify'],
                        help='Conversion/analysis mode')
    parser.add_argument('--input', '-i', default='files/decompiled',
                        help='Input file or directory')
    parser.add_argument('--output', '-o', default='converted/textures',
                        help='Output file or directory')
    parser.add_argument('--format', '-f', default='RGB565',
                        help='Target format: RGB565, RGBA5551, PSMT8, PSMT4, L8')
    parser.add_argument('--recursive', '-r', action='store_true', default=True,
                        help='Process recursively')
    parser.add_argument('--force', action='store_true',
                        help='Overwrite existing files')
    parser.add_argument('--interactive', '-y', action='store_true',
                        help='Prompt for confirmation on each conversion')

    args = parser.parse_args()

    if args.mode == 'txs3_to_png':
        print("Converting TXS3 -> PNG...")
        batch_convert(args.input, args.output, 'txs3_to_png',
                     recursive=args.recursive, force=args.force,
                     interactive=args.interactive)

    elif args.mode == 'png_to_txs3':
        print(f"Converting PNG -> TXS3 ({args.format})...")
        batch_convert(args.input, args.output, 'png_to_txs3',
                     formats=[args.format], recursive=args.recursive)

    elif args.mode == 'analyze':
        print("Analyzing texture files...")
        tex_count = 0
        for ext in ['*.img', '*.txs', '*.txs3']:
            for tex_file in Path(args.input).glob(f'**/{ext}' if args.recursive else ext):
                if not TXS3Converter.is_txs3_file(str(tex_file)):
                    continue
                header = TXS3Converter.parse_header(str(tex_file))
                if header:
                    tex_count += 1
                    dim_note = ""
                    if header['hdr_w'] != header['width'] or header['hdr_h'] != header['height']:
                        dim_note = f" (hdr: {header['hdr_w']}x{header['hdr_h']})"
                    pad_note = " [padded]" if header['has_padding'] else ""
                    swz_note = " [swizzled]" if header['is_swizzled'] else ""
                    print(f"  {tex_file.name}: {header['width']}x{header['height']}{dim_note} "
                          f"{header['format_name']}{pad_note}{swz_note}")
        print(f"\nTotal: {tex_count} textures analyzed")

    elif args.mode == 'verify':
        print("Verifying texture conversion (interactive)...")
        batch_convert(args.input, args.output, 'txs3_to_png',
                     recursive=args.recursive, force=args.force,
                     interactive=True)


if __name__ == '__main__':
    main()

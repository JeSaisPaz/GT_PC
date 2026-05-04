import os
import struct
import numpy as np
from pathlib import Path
from typing import Dict, Optional, Tuple, List, Any
from enum import IntEnum
from PIL import Image

class TextureFormat(IntEnum):
    RGBA8888 = 0x01
    RGB888 = 0x02
    RGBA5551 = 0x03
    PSMT4_SWIZZLED = 0x04
    PSMT8 = 0x05
    LA88 = 0x06
    L8 = 0x07
    PSMT4 = 0x08
    A8 = 0x09
    DXT1 = 0x0A
    DXT3 = 0x0B
    DXT5 = 0x0C
    BC7 = 0x1B

FORMAT_BPP = {
    1: 4,    # RGBA8888
    2: 3,    # RGB888
    3: 2,    # RGBA5551
    4: 0.5,  # PSMT4_SWIZZLED
    5: 1,    # PSMT8
    6: 2,    # LA88
    7: 1,    # L8
    8: 0.5,  # PSMT4
    9: 1,    # A8
}

PALETTED_FORMATS = {0x04, 0x05, 0x08}
SWIZZLE_BLOCKS = {
    0x04: (16, 8),
    0x05: (16, 8),
    0x03: (16, 8),
}

class TXS3Patcher:
    def __init__(self, vol_dir: str, tool_path: str = "workflow/GTPSPVolTools/GTPSPVolTools.exe"):
        self.vol_dir = Path(vol_dir)
        # Make tool path relative to the script's parent directory (moddingTools)
        self.tool_path = Path(__file__).parent.parent / tool_path

    @staticmethod
    def is_txs3_file(filepath: str) -> bool:
        try:
            with open(filepath, 'rb') as f:
                magic = f.read(4)
                return magic in [b'TXS3', b'3SXT']
        except:
            return False

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
                f.seek(0x1C) # img_ptr offset
                img_ptr = struct.unpack(fmt + 'I', f.read(4))[0]
            
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

            # Padded dimensions for paletted/swizzled
            pw, ph = hdr_w, hdr_h
            if format_id in PALETTED_FORMATS:
                bpp = FORMAT_BPP.get(format_id, 1)
                pw = 1 << (hdr_w - 1).bit_length()
                ph = int(data_size / bpp / pw) if pw > 0 else hdr_h

            return {
                'magic': magic,
                'endian': endian,
                'fmt': fmt,
                'img_ptr': img_ptr,
                'data_ptr': data_ptr,
                'data_size': data_size,
                'format_id': format_id,
                'format_name': TextureFormat(format_id).name if format_id in TextureFormat.__members__.values() else f'Unknown_{format_id:02X}',
                'width': hdr_w,
                'height': hdr_h,
                'padded_w': pw,
                'padded_h': ph,
                'is_swizzled': is_swizzled,
                'valid': (hdr_w > 0 and hdr_h > 0)
            }
        except Exception as e:
            print(f"Error parsing header: {e}")
            return None

    @staticmethod
    def _apply_swizzle(data: bytes, pw: int, ph: int, sb_w: int, sb_h: int, bpp: float) -> bytes:
        """Reverse of deswizzle: takes linear pixels and arranges them into blocks."""
        # data is linear pixel bytes
        # we need to create an array and fill it in the swizzled order
        pixel_count = pw * ph
        # For PSMT4, data is already nibbles, but for generic swizzle we work in pixels
        # We'll convert bytes to a flat pixel array first
        if bpp == 1: # PSMT8
            pixels = np.frombuffer(data, dtype=np.uint8)
        elif bpp == 2: # RGBA5551
            pixels = np.frombuffer(data, dtype=np.uint16)
        elif bpp == 0.5: # PSMT4
            # data is already packed nibbles, we need to unpack them to indices first
            arr = np.frombuffer(data, dtype=np.uint8)
            high = (arr >> 4) & 0xF
            low = arr & 0xF
            pixels = np.empty(pixel_count, dtype=np.uint8)
            pixels[0::2] = high
            pixels[1::2] = low
        else:
            return data

        output = np.zeros((ph, pw), dtype=pixels.dtype)
        idx = 0
        for sy in range(0, ph, sb_h):
            for sx in range(0, pw, sb_w):
                wb = min(sb_w, pw - sx)
                hb = min(sb_h, ph - sy)
                for by in range(hb):
                    for bx in range(wb):
                        if idx < len(pixels):
                            output[sy + by, sx + bx] = pixels[idx]
                            idx += 1
        
        # Now flatten and pack back to bytes
        if bpp == 1:
            return output.astype(np.uint8).tobytes()
        elif bpp == 2:
            return output.astype(np.uint16).tobytes()
        elif bpp == 0.5:
            # Pack indices back into nibbles
            flat = output.ravel()
            packed = np.empty(len(flat) // 2, dtype=np.uint8)
            for i in range(0, len(flat), 2):
                packed[i//2] = (flat[i] << 4) | (flat[i+1] & 0x0F)
            return packed.tobytes()
        return data

    def extract_texture(self, rel_path: str, output_dir: str) -> bool:
        img_path = self.vol_dir / rel_path
        header = self.parse_header(str(img_path))
        if not header or not header['valid']:
            return False
        
        # Use the logic from convert_textures.py but wrapped here
        # For brevity, I'll import the converter if available, or just implement the critical parts
        import sys
        from pathlib import Path
        sys.path.insert(0, str(Path(__file__).parent.parent))
        from scripts.convert_textures import TXS3Converter
        png_path = Path(output_dir) / Path(rel_path).with_suffix('.png')
        png_path.parent.mkdir(parents=True, exist_ok=True)
        
        return TXS3Converter.convert_txs3_to_png(str(img_path), str(png_path), force=True)

    def replace_texture(self, rel_path: str, png_path: str, dither: bool = True) -> bool:
        img_path = self.vol_dir / rel_path
        header = self.parse_header(str(img_path))
        if not header or not header['valid']:
            return False

        # Load image
        img = Image.open(png_path).convert('RGBA')
        img = img.resize((header['width'], header['height']), Image.Resampling.LANCZOS)
        
        fmt_id = header['format_id']
        pw, ph = header['padded_w'], header['padded_h']
        
        # 1. Encode Pixels
        if fmt_id == TextureFormat.RGBA8888:
            pixels = np.array(img).astype(np.uint8)
            pixel_data = pixels.tobytes()
        elif fmt_id == TextureFormat.RGBA5551:
            pixels = np.array(img).astype(np.uint16)
            r = (pixels[:, :, 0].astype(np.uint16) * 31 // 255)
            g = (pixels[:, :, 1].astype(np.uint16) * 31 // 255)
            b = (pixels[:, :, 2].astype(np.uint16) * 31 // 255)
            a = (pixels[:, :, 3].astype(np.uint16) // 255)
            arr = (r << 11) | (g << 6) | (b << 1) | a
            pixel_data = arr.astype(np.uint16).tobytes()
        elif fmt_id in {TextureFormat.PSMT8, TextureFormat.PSMT4_SWIZZLED, TextureFormat.PSMT4}:
            colors = 256 if fmt_id == TextureFormat.PSMT8 else 16
            
            # Convert to RGB for quantization (drop alpha for palette generation)
            rgb_img = img.convert('RGB')
            
            # Quantize
            if dither:
                # Convert to P mode with dithering
                p_img = rgb_img.convert('P', palette=Image.Palette.ADAPTIVE, colors=colors)
            else:
                p_img = rgb_img.quantize(colors=colors, method=Image.Quantize.MAXCOVERAGE)
            
            # Get palette (RGB triplets, need to convert to RGBA)
            pal_rgb = p_img.getpalette()  # RGBRGB...
            # Convert to RGBA by adding alpha=255
            pal_rgba = []
            for i in range(0, len(pal_rgb), 3):
                pal_rgba.extend([pal_rgb[i], pal_rgb[i+1], pal_rgb[i+2], 255])
            palette_data = bytes(pal_rgba[:colors * 4])
            
            indices = np.array(p_img).astype(np.uint8)
            # Pad indices to padded_w, padded_h
            padded_indices = np.zeros((ph, pw), dtype=np.uint8)
            padded_indices[:header['height'], :header['width']] = indices
            
            if fmt_id == TextureFormat.PSMT8:
                pixel_data = padded_indices.tobytes()
            else: # PSMT4 or PSMT4_SWIZZLED
                flat = padded_indices.ravel()
                packed = np.zeros(len(flat) // 2, dtype=np.uint8)
                for i in range(0, len(flat), 2):
                    packed[i//2] = (flat[i] << 4) | (flat[i+1] & 0x0F)
                pixel_data = packed.tobytes()
        elif fmt_id == TextureFormat.L8:
            gray = img.convert('L')
            pixels = np.array(gray).astype(np.uint8)
            pixel_data = pixels.tobytes()
        else:
            print(f"Unsupported format for replacement: {header['format_name']}")
            return False

        # 2. Swizzle if necessary
        if header['is_swizzled']:
            sb_w, sb_h = SWIZZLE_BLOCKS.get(fmt_id, (16, 8))
            bpp = FORMAT_BPP.get(fmt_id, 1)
            pixel_data = self._apply_swizzle(pixel_data, pw, ph, sb_w, sb_h, bpp)

        # 3. In-place update
        with open(img_path, 'rb') as f:
            full_file = bytearray(f.read())
        
        # Update data size in header (ImageInfo[4:8])
        # img_ptr is header offset 0x1C
        info_offset = header['img_ptr']
        data_size_offset = info_offset + 4
        
        new_data_size = len(pixel_data)
        struct.pack_into(header['fmt'] + 'I', full_file, data_size_offset, new_data_size)
        
        # Replace pixel data
        data_ptr = header['data_ptr']
        full_file[data_ptr : data_ptr + len(pixel_data)] = pixel_data
        
        # Handle case where new data is larger than old (requires shifting trailing data)
        if len(pixel_data) != header['data_size']:
            diff = len(pixel_data) - header['data_size']
            trailing = full_file[data_ptr + header['data_size']:]
            # If larger, we grow the file. If smaller, we can just pad.
            # To keep it simple and safe, we rebuild the file from the shifted parts
            head = full_file[:data_ptr]
            body = pixel_data
            tail = trailing
            full_file = bytearray(head + body + tail)

        # Palette Update: If paletted, we should update the palette in the trailing data
        if fmt_id in PALETTED_FORMATS:
            # find palette ptr from trailing header
            # Palette offset in trailing: 0x10 (based on convert_textures.py)
            # Trailing starts at data_ptr + data_size
            trailing_start = data_ptr + new_data_size
            if len(full_file) > trailing_start + 0x10:
                # PSMT8/PSMT4_SWIZZLED use palette at trailing[0x10]
                if fmt_id in {TextureFormat.PSMT8, TextureFormat.PSMT4_SWIZZLED}:
                    # Write the palette bytes
                    # We need to make sure we have the palette data calculated
                    if 'palette_data' in locals():
                        full_file[trailing_start + 0x10 : trailing_start + 0x10 + len(palette_data)] = palette_data
                elif fmt_id == TextureFormat.PSMT4:
                    # PSMT4 palette starts at offset 0 of trailing
                    if 'palette_data' in locals():
                        full_file[trailing_start : trailing_start + len(palette_data)] = palette_data

        with open(img_path, 'wb') as f:
            f.write(full_file)
            
        return True

    def list_textures(self) -> List[Dict]:
        textures = []
        for path in self.vol_dir.rglob('*.img'):
            h = self.parse_header(str(path))
            if h and h['valid']:
                textures.append({
                    'path': str(path.relative_to(self.vol_dir)),
                    'format': h['format_name'],
                    'width': h['width'],
                    'height': h['height'],
                    'swizzled': h['is_swizzled']
                })
        return textures

    def pack_vol(self, output_path: str = "GT_MOD.VOL") -> Tuple[bool, str]:
        import subprocess
        cmd = [str(self.tool_path), "pack", "-i", str(self.vol_dir), "-o", output_path]
        try:
            res = subprocess.run(cmd, capture_output=True, text=True, check=True)
            return True, res.stdout
        except subprocess.CalledProcessError as e:
            return False, e.stderr

    def unpack_vol(self, vol_path: str, output_path: str) -> Tuple[bool, str]:
        import subprocess
        cmd = [str(self.tool_path), "unpack", "-i", vol_path, "-o", output_path]
        try:
            res = subprocess.run(cmd, capture_output=True, text=True, check=True)
            return True, res.stdout
        except subprocess.CalledProcessError as e:
            return False, e.stderr

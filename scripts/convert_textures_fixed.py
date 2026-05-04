#!/usr/bin/env python3
"""GTPSP Texture Conversion Tool - Fixed version with correct TXS3 parsing."""

import os
import struct
import numpy as np
from pathlib import Path
from typing import Dict, Optional
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
    RGB565 = 0x04
    RGBA4444 = 0x05
    LA88 = 0x06
    L8 = 0x07
    L4 = 0x08
    A8 = 0x09
    DXT1 = 0x0A
    DXT3 = 0x0B
    DXT5 = 0x0C
    BC7 = 0x1B

class TXS3Converter:
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
        """Parse TXS3/IMG header for GT PSP textures.
        
        TXS3 Header Structure (little-endian):
        - Offset 0x00: Magic '3SXT' (4 bytes)
        - Offset 0x04: File Size (4 bytes)
        - Offset 0x08: Relocation Pointer
        - Offset 0x0C: Unknown
        - Offset 0x10: Unknown
        - Offset 0x14: PGLUTextureInfo Count (2 bytes)
        - Offset 0x16: Image Info Count (2 bytes)
        - Offset 0x18: PGLUTextureInfo Pointer (4 bytes)
        - Offset 0x1C: Image Info Pointer (4 bytes)
        
        ImageInfo Structure:
        - Offset 0x00: Data Pointer (4 bytes)
        - Offset 0x04: Data Size (4 bytes)
        - Offset 0x08: Unknown (1 byte)
        - Offset 0x09: Format (1 byte) - 4=RGB565, 7=L8, 8=L4
        - Offset 0x0A: Mipmap Count (1 byte)
        - Offset 0x0B: Unknown (1 byte)
        - Offset 0x0C: Width (2 bytes)
        - Offset 0x0E: Height (2 bytes)
        """
        try:
            with open(filepath, 'rb') as f:
                magic = f.read(4)
                if magic not in [b'TXS3', b'3SXT']:
                    return None
                
                endian = 'little' if magic == b'3SXT' else 'big'
                fmt = '<' if endian == 'little' else '>'
                
                file_size = struct.unpack(fmt + 'I', f.read(4))[0]
                f.read(4)  # reloc
                f.read(4)  # unknown1
                f.read(4)  # unknown2
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
            format_id = img_info[9]
            mipmaps = img_info[10]
            width = struct.unpack(fmt + 'H', img_info[12:14])[0]
            height = struct.unpack(fmt + 'H', img_info[14:16])[0]
            
            # Auto-detect format based on data size
            # Many GT PSP files misreport format, so check actual data size
            if width > 0 and height > 0:
                expected_l4 = width * height // 2
                expected_l8 = width * height
                expected_rgb565 = width * height * 2
                
                if data_size == expected_l4:
                    format_id = 8  # L4
                elif data_size == expected_l8:
                    format_id = 7  # L8
                elif data_size == expected_rgb565:
                    format_id = 4  # RGB565
            
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
                'data_size': data_size,
                'format_id': format_id,
                'format_name': TextureFormat(format_id).name if format_id in TextureFormat.__members__.values() else f'Unknown_{format_id:02X}',
                'mipmaps': mipmaps,
                'width': width,
                'height': height,
                'valid': (width > 0 and height > 0 and width <= 4096 and height <= 4096)
            }
        except Exception as e:
            print(f"Error parsing header: {e}")
            return None
    
    @staticmethod
    def decode_rgb565(data: bytes, width: int, height: int) -> np.ndarray:
        pixels = np.frombuffer(data, dtype=np.uint16).reshape(height, width)
        r = ((pixels >> 11) & 0x1F) * 255 // 31
        g = ((pixels >> 5) & 0x3F) * 255 // 63
        b = (pixels & 0x1F) * 255 // 31
        rgb = np.stack([r, g, b], axis=-1).astype(np.uint8)
        return rgb
    
    @staticmethod
    def decode_l4(data: bytes, width: int, height: int) -> np.ndarray:
        expected = width * height // 2
        if len(data) < expected:
            data = data.ljust(expected, b'\x00')
        elif len(data) > expected:
            data = data[:expected]
        
        arr = np.frombuffer(data, dtype=np.uint8)
        high = (arr >> 4) & 0xF
        low = arr & 0xF
        
        pixels = np.empty(height * width, dtype=np.uint8)
        pixels[0::2] = high
        pixels[1::2] = low
        
        gray = pixels.reshape(height, width)
        rgb = np.stack([gray, gray, gray], axis=-1).astype(np.uint16) * 17
        return rgb.astype(np.uint8)
    
    @staticmethod
    def decode_l8(data: bytes, width: int, height: int) -> np.ndarray:
        expected = width * height
        if len(data) < expected:
            data = data.ljust(expected, b'\x00')
        elif len(data) > expected:
            data = data[:expected]
        
        gray = np.frombuffer(data, dtype=np.uint8).reshape(height, width)
        rgb = np.stack([gray, gray, gray], axis=-1)
        return rgb
    
    @staticmethod
    def decode_rgba5551(data: bytes, width: int, height: int) -> np.ndarray:
        """Decode RGBA5551 data to RGBA8888"""
        pixels = np.frombuffer(data, dtype=np.uint16).reshape(height, width)
        r = ((pixels >> 11) & 0x1F) * 255 // 31
        g = ((pixels >> 6) & 0x1F) * 255 // 31
        b = ((pixels >> 1) & 0x1F) * 255 // 31
        a = (pixels & 0x1) * 255
        rgba = np.stack([r, g, b, a], axis=-1).astype(np.uint8)
        return rgba
    
    @staticmethod
    def decode_rgba4444(data: bytes, width: int, height: int) -> np.ndarray:
        """Decode RGBA4444 data to RGBA8888"""
        pixels = np.frombuffer(data, dtype=np.uint16).reshape(height, width)
        r = ((pixels >> 12) & 0xF) * 17
        g = ((pixels >> 8) & 0xF) * 17
        b = ((pixels >> 4) & 0xF) * 17
        a = (pixels & 0xF) * 17
        rgba = np.stack([r, g, b, a], axis=-1).astype(np.uint8)
        return rgba
    
    @staticmethod
    def decode_rgba8888(data: bytes, width: int, height: int) -> np.ndarray:
        return np.frombuffer(data, dtype=np.uint8).reshape(height, width, 4)
    
    @staticmethod
    def convert_to_png(img_path: str, png_path: str) -> bool:
        if not HAS_PIL:
            print("Error: PIL/Pillow required")
            return False
        
        try:
            header = TXS3Converter.parse_header(img_path)
            
            if not header or not header['valid']:
                print(f"Warning: Invalid header for {os.path.basename(img_path)}")
                return False
            
            print(f"Converting {os.path.basename(img_path)}: "
                  f"{header['width']}x{header['height']} {header['format_name']}")
            
            with open(img_path, 'rb') as f:
                f.seek(header['data_ptr'])
                actual_data_size = header['actual_file_size'] - header['data_ptr']
                data = f.read(actual_data_size)
            
            if header['format_id'] == TextureFormat.RGB565:
                pixels = TXS3Converter.decode_rgb565(data, header['width'], header['height'])
            elif header['format_id'] == TextureFormat.RGBA5551:
                pixels = TXS3Converter.decode_rgba5551(data, header['width'], header['height'])
            elif header['format_id'] == TextureFormat.RGBA4444:
                pixels = TXS3Converter.decode_rgba4444(data, header['width'], header['height'])
            elif header['format_id'] == TextureFormat.RGBA8888:
                pixels = TXS3Converter.decode_rgba8888(data, header['width'], header['height'])
            elif header['format_id'] == TextureFormat.L4:
                pixels = TXS3Converter.decode_l4(data, header['width'], header['height'])
            elif header['format_id'] == TextureFormat.L8:
                pixels = TXS3Converter.decode_l8(data, header['width'], header['height'])
            else:
                print(f"Unsupported format: {header['format_name']}")
                return False
            
            img = Image.fromarray(pixels)
            img.save(png_path, 'PNG')
            print(f"Saved to {png_path}")
            return True
            
        except Exception as e:
            print(f"Error converting {img_path}: {e}")
            return False

def batch_convert_textures(input_dir: str, output_dir: str, recursive: bool = True):
    input_path = Path(input_dir)
    output_path = Path(output_dir)
    output_path.mkdir(parents=True, exist_ok=True)
    
    converted = 0
    failed = 0
    
    patterns = ["**/*.img"] if recursive else ["*.img"]
    
    for pattern in patterns:
        for tex_file in input_path.glob(pattern):
            if tex_file.stat().st_size < 64:
                continue
            
            rel_path = tex_file.relative_to(input_path)
            png_file = output_path / rel_path.with_suffix('.png')
            png_file.parent.mkdir(parents=True, exist_ok=True)
            
            if TXS3Converter.is_txs3_file(str(tex_file)):
                if TXS3Converter.convert_to_png(str(tex_file), str(png_file)):
                    converted += 1
                else:
                    failed += 1
    
    print(f"\nTexture conversion complete:")
    print(f"  Converted: {converted}")
    print(f"  Failed: {failed}")
    return converted, failed

def main():
    import argparse
    
    parser = argparse.ArgumentParser(description='GTPSP Texture Conversion Tool')
    parser.add_argument('command', choices=['convert', 'test', 'analyze'],
                       help='Command to execute')
    parser.add_argument('--input', '-i', default='files/decompiled',
                       help='Input directory or file')
    parser.add_argument('--output', '-o', default='converted/textures',
                       help='Output directory for conversions')
    parser.add_argument('--recursive', '-r', action='store_true', default=True,
                       help='Process directories recursively')
    
    args = parser.parse_args()
    
    if args.command == 'convert':
        print("Converting texture files to PNG...")
        batch_convert_textures(args.input, args.output, args.recursive)
    
    elif args.command == 'analyze':
        print("Analyzing texture files...")
        tex_files = list(Path(args.input).glob("**/*.img" if args.recursive else "*.img"))
        for tex_file in tex_files[:10]:
            if TXS3Converter.is_txs3_file(str(tex_file)):
                header = TXS3Converter.parse_header(str(tex_file))
                if header:
                    print(f"  {tex_file.name}: {header['width']}x{header['height']} {header['format_name']}")
    
    elif args.command == 'test':
        print("Testing texture conversion...")
        test_files = list(Path(args.input).glob("**/*.img" if args.recursive else "*.img"))
        if test_files:
            test_file = str(test_files[0])
            print(f"Testing with: {test_file}")
            output_dir = Path(args.output) / "test"
            output_dir.mkdir(parents=True, exist_ok=True)
            png_file = output_dir / Path(test_file).name.replace('.img', '.png')
            success = TXS3Converter.convert_to_png(test_file, str(png_file))
            print(f"Conversion successful: {success}")

if __name__ == '__main__':
    main()
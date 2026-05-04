#!/usr/bin/env python3
"""
Test texture conversion with different approaches
"""

import os
import struct
import numpy as np
from PIL import Image

def try_decode_as_texture(filepath, width, height, format_type='RGB565'):
    """Try to decode file as texture with given dimensions"""
    try:
        with open(filepath, 'rb') as f:
            data = f.read()
        
        file_size = len(data)
        expected_size = width * height * (2 if format_type in ['RGB565', 'RGBA5551'] else 4)
        
        if file_size != expected_size:
            print(f"  Size mismatch: expected {expected_size}, got {file_size}")
            return False
        
        # Try to decode
        if format_type == 'RGB565':
            # Decode RGB565
            pixels = np.frombuffer(data, dtype=np.uint16).reshape(height, width)
            
            # Convert to RGB888
            r = ((pixels >> 11) & 0x1F) * 255 // 31
            g = ((pixels >> 5) & 0x3F) * 255 // 63
            b = (pixels & 0x1F) * 255 // 31
            
            # Create image
            rgb = np.stack([r, g, b], axis=-1).astype(np.uint8)
            img = Image.fromarray(rgb, 'RGB')
            
        elif format_type == 'RGBA5551':
            # Decode RGBA5551
            pixels = np.frombuffer(data, dtype=np.uint16).reshape(height, width)
            
            # Convert to RGBA8888
            r = ((pixels >> 11) & 0x1F) * 255 // 31
            g = ((pixels >> 6) & 0x1F) * 255 // 31
            b = ((pixels >> 1) & 0x1F) * 255 // 31
            a = (pixels & 0x1) * 255
            
            rgba = np.stack([r, g, b, a], axis=-1).astype(np.uint8)
            img = Image.fromarray(rgba, 'RGBA')
        
        else:
            print(f"  Unsupported format: {format_type}")
            return False
        
        # Save test image
        test_dir = 'test_output'
        os.makedirs(test_dir, exist_ok=True)
        
        filename = os.path.basename(filepath)
        output_path = os.path.join(test_dir, f'{filename}_{width}x{height}_{format_type}.png')
        img.save(output_path)
        
        print(f"  Saved test image: {output_path}")
        return True
        
    except Exception as e:
        print(f"  Error: {e}")
        return False

def analyze_and_test(filepath):
    """Analyze file and test different texture interpretations"""
    print(f"\nAnalyzing: {os.path.basename(filepath)}")
    
    with open(filepath, 'rb') as f:
        data = f.read()
    
    file_size = len(data)
    print(f"File size: {file_size} bytes")
    
    # Common PSP texture dimensions
    test_dimensions = [
        (128, 128),   # 16KB (RGB565) / 32KB (RGBA8888)
        (256, 256),   # 64KB / 128KB
        (512, 256),   # 128KB / 256KB
        (256, 512),   # 128KB / 256KB
        (512, 512),   # 256KB / 512KB
        (64, 64),     # 4KB / 8KB
        (32, 32),     # 1KB / 2KB
        (16, 16),     # 256B / 512B
    ]
    
    # Try different interpretations
    print("\nTrying different interpretations:")
    
    for width, height in test_dimensions:
        # Try RGB565 (2 bytes per pixel)
        expected_rgb565 = width * height * 2
        if file_size == expected_rgb565:
            print(f"\n  {width}x{height} RGB565 (exact match):")
            try_decode_as_texture(filepath, width, height, 'RGB565')
        
        # Try RGBA5551 (2 bytes per pixel)
        expected_rgba5551 = width * height * 2
        if file_size == expected_rgba5551:
            print(f"\n  {width}x{height} RGBA5551 (exact match):")
            try_decode_as_texture(filepath, width, height, 'RGBA5551')
        
        # Try RGBA8888 (4 bytes per pixel)
        expected_rgba8888 = width * height * 4
        if file_size == expected_rgba8888:
            print(f"\n  {width}x{height} RGBA8888 (exact match):")
            # Note: Would need different decoder
    
    # Also check if it's multiple of a common size
    print("\nChecking for multiple textures or mipmaps:")
    
    base_sizes = [128*128*2, 256*256*2, 512*512*2,  # RGB565
                  128*128*4, 256*256*4, 512*512*4]   # RGBA8888
    
    for base_size in base_sizes:
        if base_size > 0 and file_size % base_size == 0:
            count = file_size // base_size
            if 1 <= count <= 10:
                print(f"  Could be {count} textures of size {base_size} bytes each")
                
                # Try to extract first texture
                if count > 1:
                    # Extract first texture
                    first_texture = data[:base_size]
                    
                    # Guess dimensions based on base_size
                    if base_size == 128*128*2:
                        w, h = 128, 128
                        fmt = 'RGB565'
                    elif base_size == 256*256*2:
                        w, h = 256, 256
                        fmt = 'RGB565'
                    elif base_size == 512*512*2:
                        w, h = 512, 512
                        fmt = 'RGB565'
                    else:
                        continue
                    
                    # Save first texture for testing
                    test_dir = 'test_output'
                    os.makedirs(test_dir, exist_ok=True)
                    
                    temp_file = os.path.join(test_dir, 'first_texture.bin')
                    with open(temp_file, 'wb') as f:
                        f.write(first_texture)
                    
                    print(f"  Testing first texture as {w}x{h} {fmt}:")
                    try_decode_as_texture(temp_file, w, h, fmt)

def main():
    """Test texture conversion"""
    test_files = [
        r'files\decompiled\Gran Turismo\PSP_GAME\USRDIR\GT.VOL\piece_gt5m\tunner_logo_S\audi.img',
        r'files\decompiled\Gran Turismo\PSP_GAME\USRDIR\GT.VOL\piece_gt5m\tunner_logo_S\bmw.img',
    ]
    
    for test_file in test_files:
        if os.path.exists(test_file):
            analyze_and_test(test_file)
        else:
            print(f"File not found: {test_file}")

if __name__ == '__main__':
    main()
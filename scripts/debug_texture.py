#!/usr/bin/env python3
"""
Debug texture file format
"""

import struct
import os

def analyze_txs3(filepath):
    print(f"Analyzing: {os.path.basename(filepath)}")
    
    with open(filepath, 'rb') as f:
        data = f.read(128)
    
    # Check magic
    magic = data[0:4]
    print(f"Magic: {magic} ({'3SXT (little-endian)' if magic == b'3SXT' else 'TXS3 (big-endian)' if magic == b'TXS3' else 'unknown'})")
    
    if magic not in [b'TXS3', b'3SXT']:
        print("Not a TXS3 file")
        return
    
    # Determine endianness
    if magic == b'3SXT':
        endian = '<'  # little-endian
    else:
        endian = '>'  # big-endian
    
    # Parse header fields
    try:
        file_size = struct.unpack(endian + 'I', data[4:8])[0]
        unknown1 = struct.unpack(endian + 'I', data[8:12])[0]
        format_id = struct.unpack(endian + 'I', data[12:16])[0]
        width = struct.unpack(endian + 'I', data[16:20])[0]
        height = struct.unpack(endian + 'I', data[20:24])[0]
        data_size = struct.unpack(endian + 'I', data[24:28])[0]
        data_offset = struct.unpack(endian + 'I', data[28:32])[0]
        
        print(f"\nParsed header:")
        print(f"  File size in header: {file_size} (actual: {os.path.getsize(filepath)})")
        print(f"  Unknown1: 0x{unknown1:08x}")
        print(f"  Format ID: 0x{format_id:08x}")
        print(f"  Width: {width} (0x{width:08x})")
        print(f"  Height: {height} (0x{height:08x})")
        print(f"  Data size: {data_size}")
        print(f"  Data offset: {data_offset} (0x{data_offset:04x})")
        
        # Check if width/height might be 16-bit values
        print(f"\nAs 16-bit values:")
        width16 = struct.unpack('H', data[16:18])[0]
        height16 = struct.unpack('H', data[18:20])[0]
        print(f"  Width (16-bit): {width16}")
        print(f"  Height (16-bit): {height16}")
        
        # Check other possible interpretations
        print(f"\nOther possible interpretations:")
        
        # Bytes at offset 16-23: 01 00 01 00 40 00 00 00
        # This could be: width=0x0001, height=0x0001, something else=0x00000040
        # Or: width=0x0001, height=0x0001, data_size=0x00000040?
        
        print(f"  Bytes 16-23: {data[16:24].hex()}")
        print(f"    As 4x uint16: {struct.unpack('HHHH', data[16:24])}")
        print(f"    As 2x uint32: {struct.unpack('II', data[16:24])}")
        
        # Check if data makes sense as texture
        if data_size > 0 and data_offset + data_size <= os.path.getsize(filepath):
            print(f"\nTexture data appears valid:")
            print(f"  Data range: 0x{data_offset:04x} to 0x{data_offset+data_size:04x}")
            
            # Try to guess format from data_size
            if width16 > 0 and height16 > 0:
                pixels = width16 * height16
                print(f"  If {width16}x{height16}: {pixels} pixels")
                
                for bpp, fmt in [(2, 'RGB565/RGBA5551'), (4, 'RGBA8888'), (1, 'L8/A8'), (8, 'DXT/BC')]:
                    expected = pixels * bpp
                    if expected == data_size:
                        print(f"    Matches {fmt} ({bpp} bytes per pixel)")
                    elif abs(expected - data_size) < 10:
                        print(f"    Close to {fmt}: expected {expected}, got {data_size}")
        
        else:
            print(f"\nWarning: Data size/offset doesn't make sense")
            print(f"  data_offset + data_size = {data_offset + data_size}")
            print(f"  file size = {os.path.getsize(filepath)}")
    
    except Exception as e:
        print(f"Error parsing header: {e}")

if __name__ == '__main__':
    test_files = [
        r'files\decompiled\Gran Turismo\PSP_GAME\USRDIR\GT.VOL\piece_gt5m\tunner_logo_S\audi.img',
        r'files\decompiled\Gran Turismo\PSP_GAME\USRDIR\GT.VOL\piece_gt5m\tunner_logo_S\bmw.img',
        r'files\decompiled\Gran Turismo\PSP_GAME\USRDIR\GT.VOL\piece_gt5m\env\env2.txs',
    ]
    
    for test_file in test_files:
        if os.path.exists(test_file):
            print("\n" + "="*80)
            analyze_txs3(test_file)
        else:
            print(f"\nFile not found: {test_file}")
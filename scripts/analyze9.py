import struct
from pathlib import Path
from collections import defaultdict

def deep_scan():
    files = [
        'files/decompiled/Gran Turismo/PSP_GAME/USRDIR/GT.VOL/piece_gt5m/license_bg/0.img',
        'files/decompiled/Gran Turismo/PSP_GAME/USRDIR/GT.VOL/piece_gt5m/course_image/akasaka.img',
        'files/decompiled/Gran Turismo/PSP_GAME/USRDIR/GT.VOL/piece_gt5m/tunner_logo_S/ferrari.img',
    ]
    
    for filepath in files:
        with open(filepath, 'rb') as f:
            data = f.read()
        
        name = Path(filepath).name
        
        magic = data[0:4]
        is_le = magic == b'3SXT'
        fmts = '<' if is_le else '>'
        
        img_ptr = struct.unpack(fmts + 'I', data[0x1C:0x20])[0]
        img_info = data[img_ptr:img_ptr+32]
        
        width = struct.unpack(fmts + 'H', img_info[0x0C:0x0E])[0]
        height = struct.unpack(fmts + 'H', img_info[0x0E:0x10])[0]
        
        expected_size = width * height
        print(f"\n{name}: {width}x{height} = {expected_size} bytes")
        
        regions = []
        
        for i in range(len(data) - expected_size):
            sample = data[i:i+expected_size]
            unique = len(set(sample))
            if unique > 20:
                regions.append((i, unique))
        
        regions.sort(key=lambda x: x[1], reverse=True)
        print(f"  Top 5 data regions:")
        for pos, unique in regions[:5]:
            sample = data[pos:pos+32]
            print(f"    0x{pos:04X}: unique={unique}, first={sample[:16].hex()}")

deep_scan()
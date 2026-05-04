import struct
from pathlib import Path

def search_for_non_zero(filepath):
    with open(filepath, 'rb') as f:
        data = f.read()
    
    name = Path(filepath).name
    magic = data[0:4]
    
    if magic not in [b'TXS3', b'3SXT']:
        return
    
    print(f"\n{name}:")
    print(f"  Total size: {len(data)}")
    
    zero_regions = []
    non_zero_regions = []
    
    i = 0
    while i < len(data):
        if data[i] == 0:
            start = i
            while i < len(data) and data[i] == 0:
                i += 1
            if i - start > 64:
                zero_regions.append((start, i))
        else:
            start = i
            while i < len(data) and data[i] != 0:
                i += 1
            non_zero_regions.append((start, i))
            if i - start < 32:
                print(f"  Non-zero @ 0x{start:04X} len={i-start}: {data[start:start+(i-start)].hex()}")
    
    print(f"  Zero regions: {len(zero_regions)}")
    for start, end in zero_regions[:5]:
        print(f"    0x{start:04X} - 0x{end:04X} ({end-start} bytes)")
    
    print(f"  Non-zero regions: {len(non_zero_regions)}")
    for start, end in non_zero_regions[:10]:
        print(f"    0x{start:04X} - 0x{end:04X} ({end-start} bytes)")
        first16 = data[start:start+16]
        unique = len(set(first16))
        print(f"      First bytes: {list(first16)}, unique={unique}")

textures = [
    'files/decompiled/Gran Turismo/PSP_GAME/USRDIR/GT.VOL/piece_gt5m/tunner_logo_S/ferrari.img',
    'files/decompiled/Gran Turismo/PSP_GAME/USRDIR/GT.VOL/piece_gt5m/tunner_logo_M/ferrari.img',
    'files/decompiled/Gran Turismo/PSP_GAME/USRDIR/GT.VOL/piece_gt5m/license_bg/0.img',
    'files/decompiled/Gran Turismo/PSP_GAME/USRDIR/GT.VOL/piece_gt5m/course_image/akasaka.img',
    'files/decompiled/Gran Turismo/PSP_GAME/USRDIR/GT.VOL/piece_gt5m/tunner_logo_S/nissan.img',
]

for tex in textures:
    search_for_non_zero(tex)
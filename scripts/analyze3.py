import struct
from pathlib import Path

def analyze_txs3_detail(filepath):
    with open(filepath, 'rb') as f:
        data = f.read()
    
    magic = data[0:4]
    name = Path(filepath).name
    
    if magic not in [b'TXS3', b'3SXT']:
        return
    
    is_le = magic == b'3SXT'
    fmts = '<' if is_le else '>'
    
    file_size = struct.unpack(fmts + 'I', data[4:8])[0]
    pglue_ptr = struct.unpack(fmts + 'I', data[0x18:0x1C])[0]
    img_ptr = struct.unpack(fmts + 'I', data[0x1C:0x20])[0]
    
    print(f"\n{name}:")
    print(f"  File size: {file_size}, ImageInfo ptr: 0x{img_ptr:04X}")
    
    img_info = data[img_ptr:img_ptr+32]
    data_ptr = struct.unpack(fmts + 'I', img_info[0:4])[0]
    data_size = struct.unpack(fmts + 'I', img_info[4:8])[0]
    unk08 = img_info[8]
    fmt = img_info[9]
    width = struct.unpack(fmts + 'H', img_info[0x0C:0x0E])[0]
    height = struct.unpack(fmts + 'H', img_info[0x0E:0x10])[0]
    
    print(f"  {width}x{height}, format byte: 0x{fmt:02X}, data_size: {data_size}")
    print(f"  Data at 0x{data_ptr:04X}, raw size: {file_size - data_ptr}")
    
    if data_ptr < len(data):
        actual_data = data[data_ptr:data_ptr+data_size]
        print(f"  First 64 data bytes: {actual_data[:64].hex()}")
        
        unique_bytes = len(set(actual_data))
        print(f"  Unique byte values: {unique_bytes}")
        
        first_vals = list(actual_data[:16])
        print(f"  First 16 bytes: {first_vals}")

textures = [
    'files/decompiled/Gran Turismo/PSP_GAME/USRDIR/GT.VOL/piece_gt5m/tunner_logo_S/ferrari.img',
    'files/decompiled/Gran Turismo/PSP_GAME/USRDIR/GT.VOL/piece_gt5m/tunner_logo_M/ferrari.img',
    'files/decompiled/Gran Turismo/PSP_GAME/USRDIR/GT.VOL/piece_gt5m/license_bg/0.img',
    'files/decompiled/Gran Turismo/PSP_GAME/USRDIR/GT.VOL/piece_gt5m/course_image/akasaka.img',
    'files/decompiled/Gran Turismo/PSP_GAME/USRDIR/GT.VOL/piece_gt5m/course_map_menu/akasaka.img',
    'files/decompiled/Gran Turismo/PSP_GAME/USRDIR/GT.VOL/piece_gt5m/tunner_logo_S/nissan.img',
]

for tex in textures:
    analyze_txs3_detail(tex)
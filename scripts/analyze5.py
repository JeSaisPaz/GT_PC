import struct
from pathlib import Path

def find_pglu_data(filepath):
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
    
    print(f"\n{name}: file_size={file_size}")
    print(f"  PGLUE ptr: 0x{pglue_ptr:04X}, Img ptr: 0x{img_ptr:04X}")
    
    if pglue_ptr > 0 and pglue_ptr < len(data):
        pglu = data[pglue_ptr:pglue_ptr+64]
        data_ptr = struct.unpack(fmts + 'I', pglu[0:4])[0]
        data_size = struct.unpack(fmts + 'I', pglu[4:8])[0]
        unk08 = pglu[8]
        fmt = pglu[9]
        width = struct.unpack(fmts + 'H', pglu[0x0C:0x0E])[0]
        height = struct.unpack(fmts + 'H', pglu[0x0E:0x10])[0]
        
        print(f"  PGLU info @ 0x{pglue_ptr:04X}:")
        print(f"    Data ptr: 0x{data_ptr:04X}, size: {data_size}")
        print(f"    Width: {width}, Height: {height}, fmt: 0x{fmt:02X}")
        
        if data_ptr < len(data):
            actual = data[data_ptr:data_ptr+min(32, data_size)]
            unique = len(set(actual))
            first = list(actual[:16])
            print(f"    First 16 data bytes: {first}, unique={unique}")

textures = [
    'files/decompiled/Gran Turismo/PSP_GAME/USRDIR/GT.VOL/piece_gt5m/tunner_logo_S/ferrari.img',
    'files/decompiled/Gran Turismo/PSP_GAME/USRDIR/GT.VOL/piece_gt5m/tunner_logo_M/ferrari.img',
    'files/decompiled/Gran Turismo/PSP_GAME/USRDIR/GT.VOL/piece_gt5m/license_bg/0.img',
    'files/decompiled/Gran Turismo/PSP_GAME/USRDIR/GT.VOL/piece_gt5m/course_image/akasaka.img',
    'files/decompiled/Gran Turismo/PSP_GAME/USRDIR/GT.VOL/piece_gt5m/course_map_menu/akasaka.img',
    'files/decompiled/Gran Turismo/PSP_GAME/USRDIR/GT.VOL/piece_gt5m/tunner_logo_S/nissan.img',
]

for tex in textures:
    find_pglu_data(tex)
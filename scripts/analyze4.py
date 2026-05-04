import struct
from pathlib import Path

def find_all_images(filepath):
    with open(filepath, 'rb') as f:
        data = f.read()
    
    magic = data[0:4]
    name = Path(filepath).name
    
    if magic not in [b'TXS3', b'3SXT']:
        return
    
    is_le = magic == b'3SXT'
    fmts = '<' if is_le else '>'
    
    file_size = struct.unpack(fmts + 'I', data[4:8])[0]
    img_count = struct.unpack(fmts + 'H', data[0x16:0x18])[0]
    pglue_ptr = struct.unpack(fmts + 'I', data[0x18:0x1C])[0]
    img_ptr = struct.unpack(fmts + 'I', data[0x1C:0x20])[0]
    
    print(f"\n{name}: file_size={file_size}, img_count={img_count}")
    print(f"  PGLUE ptr: 0x{pglue_ptr:04X}, Img ptr: 0x{img_ptr:04X}")
    
    if img_ptr == 0 or img_ptr < 0x100:
        print("  Using embedded ImageInfo")
        img_info = data[0x20:0x40]
    else:
        img_info = data[img_ptr:img_ptr+32]
    
    for i in range(img_count):
        offset = img_ptr + (i * 32)
        if offset + 32 > len(data):
            break
        img_info = data[offset:offset+32]
        
        data_ptr = struct.unpack(fmts + 'I', img_info[0:4])[0]
        data_size = struct.unpack(fmts + 'I', img_info[4:8])[0]
        unk08 = img_info[8]
        fmt = img_info[9]
        width = struct.unpack(fmts + 'H', img_info[0x0C:0x0E])[0]
        height = struct.unpack(fmts + 'H', img_info[0x0E:0x10])[0]
        
        print(f"  [{i}] @ 0x{offset:04X}: {width}x{height}, fmt=0x{fmt:02X}, data_size={data_size}, raw sz={file_size - data_ptr}")
        
        if data_ptr < len(data):
            actual = data[data_ptr:data_ptr+min(64, data_size)]
            unique = len(set(actual))
            first = list(actual[:8])
            print(f"      First 8 bytes: {first}, unique={unique}")

textures = [
    'files/decompiled/Gran Turismo/PSP_GAME/USRDIR/GT.VOL/piece_gt5m/tunner_logo_S/ferrari.img',
    'files/decompiled/Gran Turismo/PSP_GAME/USRDIR/GT.VOL/piece_gt5m/tunner_logo_M/ferrari.img',
    'files/decompiled/Gran Turismo/PSP_GAME/USRDIR/GT.VOL/piece_gt5m/license_bg/0.img',
    'files/decompiled/Gran Turismo/PSP_GAME/USRDIR/GT.VOL/piece_gt5m/course_image/akasaka.img',
    'files/decompiled/Gran Turismo/PSP_GAME/USRDIR/GT.VOL/piece_gt5m/course_map_menu/akasaka.img',
    'files/decompiled/Gran Turismo/PSP_GAME/USRDIR/GT.VOL/piece_gt5m/tunner_logo_S/nissan.img',
]

for tex in textures:
    find_all_images(tex)
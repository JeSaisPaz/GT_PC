import struct
from pathlib import Path

def analyze_txs3(filepath):
    with open(filepath, 'rb') as f:
        data = f.read()
    
    magic = data[0:4]
    print(f"\n{Path(filepath).name}:")
    print(f"  Magic: {magic}")
    
    if magic not in [b'TXS3', b'3SXT']:
        print("  NOT TXS3 format")
        return
    
    is_le = magic == b'3SXT'
    fmts = '<' if is_le else '>'
    
    file_size = struct.unpack(fmts + 'I', data[4:8])[0]
    pglue_cnt = struct.unpack(fmts + 'H', data[0x14:0x16])[0]
    img_cnt = struct.unpack(fmts + 'H', data[0x16:0x18])[0]
    pglue_ptr = struct.unpack(fmts + 'I', data[0x18:0x1C])[0]
    img_ptr = struct.unpack(fmts + 'I', data[0x1C:0x20])[0]
    
    print(f"  File size: {file_size}, PGLUE cnt: {pglue_cnt}, Img cnt: {img_cnt}")
    print(f"  PGLUE ptr: 0x{pglue_ptr:04X}, Img ptr: 0x{img_ptr:04X}")
    
    if img_ptr < len(data) - 32:
        img_info = data[img_ptr:img_ptr+32]
        data_ptr = struct.unpack(fmts + 'I', img_info[0:4])[0]
        data_size = struct.unpack(fmts + 'I', img_info[4:8])[0]
        unk08 = img_info[8]
        fmt = img_info[9]
        mipmap = img_info[10]
        unk0b = img_info[0x0B]
        width = struct.unpack(fmts + 'H', img_info[0x0C:0x0E])[0]
        height = struct.unpack(fmts + 'H', img_info[0x0E:0x10])[0]
        
        print(f"  ImageInfo @ 0x{img_ptr:04X}:")
        print(f"    Data ptr: 0x{data_ptr:04X}, size: {data_size}")
        print(f"    Unknown08: 0x{unk08:02X}, Format: 0x{fmt:02X}, Mipmap: {mipmap}, Unk0B: 0x{unk0b:02X}")
        print(f"    Width: {width}, Height: {height}")
        
        calc_size = width * height
        size_l8 = width * height
        size_l4 = width * height // 2
        size_rgb565 = width * height * 2
        
        detected = "UNKNOWN"
        if data_size == size_l4: detected = "L4"
        elif data_size == size_l8: detected = "L8"
        elif data_size == size_rgb565: detected = "RGB565"
        
        expected = calc_size
        print(f"    Expected L4={size_l4}, L8={size_l8}, RGB565={size_rgb565}")
        print(f"    DETECTED: {detected}")
        
        if data_ptr < len(data):
            raw_data = data[data_ptr:data_ptr+min(32, data_size)]
            print(f"    Raw data first 32 bytes: {raw_data.hex()}")
            
            if data_size >= 4:
                val0 = struct.unpack(fmts + 'H', raw_data[0:2])[0]
                print(f"    First 16bit: 0x{val0:04X}")

textures = [
    'files/decompiled/Gran Turismo/PSP_GAME/USRDIR/GT.VOL/piece_gt5m/tunner_logo_S/ferrari.img',
    'files/decompiled/Gran Turismo/PSP_GAME/USRDIR/GT.VOL/piece_gt5m/tunner_logo_M/ferrari.img',
    'files/decompiled/Gran Turismo/PSP_GAME/USRDIR/GT.VOL/piece_gt5m/license_bg/0.img',
    'files/decompiled/Gran Turismo/PSP_GAME/USRDIR/GT.VOL/piece_gt5m/course_image/akasaka.img',
]

for tex in textures:
    analyze_txs3(tex)
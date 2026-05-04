import struct
from pathlib import Path

def find_real_data_location():
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
        
        file_size = struct.unpack(fmts + 'I', data[4:8])[0]
        img_ptr = struct.unpack(fmts + 'I', data[0x1C:0x20])[0]
        img_info = data[img_ptr:img_ptr+32]
        
        width = struct.unpack(fmts + 'H', img_info[0x0C:0x0E])[0]
        height = struct.unpack(fmts + 'H', img_info[0x0E:0x10])[0]
        header_data_ptr = struct.unpack(fmts + 'I', img_info[0:4])[0]
        
        w, h = width, height
        
        print(f"\n{name}: {width}x{height}")
        
        for size, label in [(w*h//2, 'L4'), (w*h, 'L8'), (w*h*2, 'RGB565')]:
            for pos in range(max(header_data_ptr - 256, 0x100), min(header_data_ptr + 256, len(data) - size)):
                sample = data[pos:pos+min(64, size)]
                if len(set(sample)) > 20:
                    full_sample = data[pos:pos+size]
                    unique = len(set(full_sample))
                    if unique > 50:
                        print(f"  {label} data at 0x{pos:04X} (header said 0x{header_data_ptr:04X}), unique={unique}")
                        print(f"    First 32 bytes: {data[pos:pos+32].hex()}")
                        break

find_real_data_location()
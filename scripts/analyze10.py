import struct
from pathlib import Path

def check_working_format():
    files = [
        ('files/decompiled/Gran Turismo/PSP_GAME/USRDIR/GT.VOL/piece_gt5m/license_bg/0.img', 'L4'),
        ('files/decompiled/Gran Turismo/PSP_GAME/USRDIR/GT.VOL/piece_gt5m/course_image/akasaka.img', '?'),
        ('files/decompiled/Gran Turismo/PSP_GAME/USRDIR/GT.VOL/piece_gt5m/tunner_logo_S/ferrari.img', '?'),
    ]
    
    for filepath, expected_fmt in files:
        with open(filepath, 'rb') as f:
            data = f.read()
        
        name = Path(filepath).name
        
        magic = data[0:4]
        is_le = magic == b'3SXT'
        fmts = '<' if is_le else '>'
        
        img_ptr = struct.unpack(fmts + 'I', data[0x1C:0x20])[0]
        img_info = data[img_ptr:img_ptr+32]
        
        header_data_ptr = struct.unpack(fmts + 'I', img_info[0:4])[0]
        header_data_size = struct.unpack(fmts + 'I', img_info[4:8])[0]
        format_byte = img_info[9]
        width = struct.unpack(fmts + 'H', img_info[0x0C:0x0E])[0]
        height = struct.unpack(fmts + 'H', img_info[0x0E:0x10])[0]
        
        print(f"\n{name}:")
        print(f"  Header: ptr=0x{header_data_ptr:04X}, size={header_data_size}, {width}x{height}, fmt=0x{format_byte:02X}")
        
        data_at_header = data[header_data_ptr:header_data_ptr+32]
        unique_at_header = len(set(data_at_header))
        print(f"  Data at header ptr: unique={unique_at_header}")
        print(f"    First 32: {data_at_header.hex()}")
        
        w, h = width, height
        for test_fmt, bpp in [('L4', 0.5), ('L8', 1), ('RGB565', 2)]:
            test_size = int(w * h * bpp)
            if test_size <= len(data) - header_data_ptr:
                sample = data[header_data_ptr:header_data_ptr + test_size]
                unique = len(set(sample))
                print(f"  As {test_fmt}: unique={unique}, expected={test_size}")
        
        if unique_at_header < 10:
            print(f"  Searching for non-zero data...")
            for i in range(0x100, len(data) - w*h*2, 0x10):
                sample = data[i:i+min(256, w*h)]
                if len(set(sample)) > 50:
                    print(f"    Found at 0x{i:04X}: unique={len(set(sample))}")
                    break

check_working_format()
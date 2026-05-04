import struct
from pathlib import Path

def analyze_working_vs_broken():
    files = [
        ('files/decompiled/Gran Turismo/PSP_GAME/USRDIR/GT.VOL/piece_gt5m/license_bg/0.img', 'WORKING'),
        ('files/decompiled/Gran Turismo/PSP_GAME/USRDIR/GT.VOL/piece_gt5m/course_image/akasaka.img', 'BROKEN'),
        ('files/decompiled/Gran Turismo/PSP_GAME/USRDIR/GT.VOL/piece_gt5m/tunner_logo_S/ferrari.img', 'BROKEN'),
    ]
    
    for filepath, status in files:
        with open(filepath, 'rb') as f:
            data = f.read()
        
        name = Path(filepath).name
        print(f"\n{'='*60}")
        print(f"{name} [{status}]")
        print(f"{'='*60}")
        
        magic = data[0:4]
        is_le = magic == b'3SXT'
        fmts = '<' if is_le else '>'
        
        file_size = struct.unpack(fmts + 'I', data[4:8])[0]
        img_ptr = struct.unpack(fmts + 'I', data[0x1C:0x20])[0]
        
        print(f"File size: {file_size}, Img ptr: 0x{img_ptr:04X}")
        
        if img_ptr < len(data) - 32:
            img_info = data[img_ptr:img_ptr+32]
            data_ptr = struct.unpack(fmts + 'I', img_info[0:4])[0]
            data_size = struct.unpack(fmts + 'I', img_info[4:8])[0]
            width = struct.unpack(fmts + 'H', img_info[0x0C:0x0E])[0]
            height = struct.unpack(fmts + 'H', img_info[0x0E:0x10])[0]
            
            print(f"Header says: data_ptr=0x{data_ptr:04X}, size={data_size}, {width}x{height}")
        
        print("\nFirst 64 bytes at data_ptr location:")
        if data_ptr < len(data):
            chunk = data[data_ptr:data_ptr+64]
            print(f"  {chunk.hex()}")
        
        print("\nTrying to find actual image data by size:")
        w, h = width, height
        expected_l8 = w * h
        expected_l4 = w * h // 2
        expected_rgb565 = w * h * 2
        
        for test_size, label in [(expected_l8, 'L8'), (expected_l4, 'L4'), (expected_rgb565, 'RGB565')]:
            found_positions = []
            for i in range(len(data) - test_size):
                if test_size > 0 and test_size < len(data) - i:
                    sample = data[i:i+min(32, test_size)]
                    if len(set(sample)) > 2:
                        found_positions.append(i)
            
            if found_positions:
                print(f"  {label} (size {test_size}) found at positions: {found_positions[:5]}")

analyze_working_vs_broken()
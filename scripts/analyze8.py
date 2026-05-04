import struct
from pathlib import Path

def find_actual_data():
    files = [
        'files/decompiled/Gran Turismo/PSP_GAME/USRDIR/GT.VOL/piece_gt5m/license_bg/0.img',
        'files/decompiled/Gran Turismo/PSP_GAME/USRDIR/GT.VOL/piece_gt5m/course_image/akasaka.img',
        'files/decompiled/Gran Turismo/PSP_GAME/USRDIR/GT.VOL/piece_gt5m/tunner_logo_S/ferrari.img',
        'files/decompiled/Gran Turismo/PSP_GAME/USRDIR/GT.VOL/piece_gt5m/tunner_logo_M/ferrari.img',
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
        
        print(f"\n{name}: {width}x{height}")
        
        scan_start = max(0x100, img_ptr + 32)
        
        best_pos = None
        best_count = 0
        
        for i in range(scan_start, len(data) - 256):
            chunk = data[i:i+256]
            unique = len(set(chunk))
            if unique > 10:
                if best_pos is None:
                    best_pos = i
                    best_count = unique
                elif unique > best_count:
                    best_pos = i
                    best_count = unique
        
        if best_pos:
            print(f"  Best data at 0x{best_pos:04X}, unique bytes: {best_count}")
            print(f"  First 32 bytes: {data[best_pos:best_pos+32].hex()}")

find_actual_data()
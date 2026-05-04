import sys
sys.path.insert(0, 'scripts')
from convert_textures import TXS3Converter
from pathlib import Path

textures = [
    'files/decompiled/Gran Turismo/PSP_GAME/USRDIR/GT.VOL/piece_gt5m/tunner_logo_S/ferrari.img',
    'files/decompiled/Gran Turismo/PSP_GAME/USRDIR/GT.VOL/piece_gt5m/tunner_logo_M/ferrari.img',
    'files/decompiled/Gran Turismo/PSP_GAME/USRDIR/GT.VOL/piece_gt5m/license_bg/0.img',
    'files/decompiled/Gran Turismo/PSP_GAME/USRDIR/GT.VOL/piece_gt5m/course_image/akasaka.img',
    'files/decompiled/Gran Turismo/PSP_GAME/USRDIR/GT.VOL/piece_gt5m/course_map_menu/akasaka.img',
    'files/decompiled/Gran Turismo/PSP_GAME/USRDIR/GT.VOL/piece_gt5m/course_logo_S/ferrari.img',
    'files/decompiled/Gran Turismo/PSP_GAME/USRDIR/GT.VOL/piece_gt5m/tunner_logo_S/nissan.img',
]

print("Analyzing texture headers:")
print("=" * 80)
for tex in textures:
    header = TXS3Converter.parse_header(tex)
    if header:
        print(f"{Path(tex).name}: {header['width']}x{header['height']} {header['format_name']} fmt={header['format_id']} data={header['data_size']} endian={header['endian']}")

print("\n\nRaw header bytes analysis:")
print("=" * 80)

import struct

def dump_header(filepath):
    with open(filepath, 'rb') as f:
        data = f.read(0x500)
    
    magic = data[0:4]
    print(f"\n{Path(filepath).name}:")
    print(f"  Magic: {magic}")
    print(f"  Main header at 0: {data[0:32].hex()}")
    print(f"  ImageInfo at 0x300: {data[0x300:0x320].hex()}")
    print(f"  Data starts at: 0x{data[0x304]:02X}{data[0x305]:02X}{data[0x306]:02X}{data[0x307]:02X} = ", end="")
    data_ptr = struct.unpack('<I', data[0x304:0x308])[0]
    print(f"0x{data_ptr:04X}")
    
    img_ptr = struct.unpack('<I', data[0x31C:0x320])[0]
    print(f"  ImageInfo ptr = 0x{img_ptr:04X}")
    
    img_info = data[img_ptr:img_ptr+32]
    print(f"  ImageInfo @ 0x{img_ptr:04X}: {img_info.hex()}")
    
    data_ptr2 = struct.unpack('<I', img_info[0:4])[0]
    data_sz = struct.unpack('<I', img_info[4:8])[0]
    fmt_byte = img_info[9]
    w = struct.unpack('<H', img_info[12:14])[0]
    h = struct.unpack('<H', img_info[14:16])[0]
    print(f"  Data ptr=0x{data_ptr2:04X} size={data_sz} fmt_byte=0x{fmt_byte:02X} {w}x{h}")

for tex in textures[:3]:
    dump_header(tex)
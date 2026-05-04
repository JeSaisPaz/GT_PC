import numpy as np
from PIL import Image

img_l8 = np.array(Image.open('converted/akasaka_map_80x64_L8.png'))
img_rgb565 = np.array(Image.open('converted/akasaka_80x32_RGB565.png'))

print(f"L8 80x64: shape={img_l8.shape}, unique={len(np.unique(img_l8))}")
print(f"RGB565 80x32: shape={img_rgb565.shape}")

# Check if 80x64 L8 matches the format/data_size better
# The format byte is 0x04 (RGB565) but data_size=5120
# For RGB565 80x32: 80*32*2 = 5120 ✓
# For L8 80x64: 80*64*1 = 5120 ✓
# Both fit! But dimensions 80x64 vs 80x32...

# The PGLU section stores UV coords
# 0x3f480000 = 0.78125 * 128 = 100 (header width hint)
# 0x3f180000 = 0.59375 * 128 = 76 (header height hint)
# These suggest max UV space of 128x128
# 80/128 = 0.625, 64/128 = 0.5
# Not matching the PGLU values directly...

# Let's check what makes more sense visually by looking at edge detection
print("\nChecking which interpretation has more natural image statistics...")

# For RGB565 80x32: check if rows look like valid scanlines
for y in [0, 10, 20, 30]:
    row = img_rgb565[y, :, 0]
    changes = np.sum(np.abs(np.diff(row.astype(int))) > 10)
    print(f"  RGB565 row {y}: {changes} significant changes")

# For L8 80x64
for y in [0, 20, 40, 60]:
    row = img_l8[y, :]
    changes = np.sum(np.abs(np.diff(row.astype(int))) > 10)
    print(f"  L8 row {y}: {changes} significant changes")

# Also check: what does the old test_map_80x64_L8 look like
img_test_l8 = np.array(Image.open('converted/test_map_80x64_L8.png'))
print(f"\ntest_map_80x64_L8: shape={img_test_l8.shape}, unique={len(np.unique(img_test_l8))}")

# The test_map was decoded from raw data at 0x154 offset (not 0x150)
# But the actual data is at 0x150 (which starts with 4 bytes of 0xFF)
# So our FIRST 4 bytes are wrong!
print("\n*** IMPORTANT: data starts at 0x150 with 0xFFFFFFFF (4 bytes of 255) ***")
print("Those 4 bytes are likely PADDING, not pixel data!")
print("So the data should start at 0x154, not 0x150!")
print("Let me verify this...")

import struct

fp = r'files/decompiled/Gran Turismo/PSP_GAME/USRDIR/GT.VOL/piece_gt5m/course_map_menu/akasaka.img'
with open(fp, 'rb') as f:
    data = f.read()

img_info = data[0x12c:0x12c+32]
data_ptr = struct.unpack('<I', img_info[0:4])[0]
data_size = struct.unpack('<I', img_info[4:8])[0]
print(f"\ndata_ptr from ImageInfo: 0x{data_ptr:x}")
print(f"First 8 bytes at data_ptr: {data[data_ptr:data_ptr+8].hex()}")
print(f"First 8 bytes at data_ptr+4: {data[data_ptr+4:data_ptr+12].hex()}")

# If we skip the first 4 bytes (padding), we get different decoded data
raw_skipped = data[data_ptr+4:data_ptr+4+data_size-4]
print(f"Raw with skip: {len(raw_skipped)} bytes")

# For files that have 0x00 or 0xFF padding at data_ptr:
# course_map_menu/akasaka: 0xFF FF FF FF
# tunner_logo_S/ferrari: 0x00 00 00 00
# course_image/akasaka: 0xAD B3 AD AD (VALID, no padding!)

# So some files have 4-byte padding before pixel data, some don't!
# Let's check ALL the files

files_to_check = [
    'tunner_logo_S/ferrari.img',
    'tunner_logo_M/ferrari.img',
    'course_image/akasaka.img',
    'course_map_menu/akasaka.img',
    'mission_flyer/mission_a.img',
    'course_map_race/20r60r_ps2.img',
    'course_logo_S/20r60r.img',
    'license_course_map_menu/license_User000.img',
]

base = r'files/decompiled/Gran Turismo/PSP_GAME/USRDIR/GT.VOL/piece_gt5m'
print("\n\nChecking for 4-byte padding at data_ptr:")
for rel in files_to_check:
    fp = os.path.join(base, rel)
    with open(fp, 'rb') as f:
        data = f.read()
    
    import os
    name = os.path.basename(fp)
    magic = data[0:4]
    is_le = magic == b'3SXT'
    fmts = '<' if is_le else '>'
    img_ptr = struct.unpack(fmts + 'I', data[28:32])[0]
    img_info = data[img_ptr:img_ptr+32]
    data_ptr = struct.unpack(fmts + 'I', img_info[0:4])[0]
    
    first_4 = data[data_ptr:data_ptr+4]
    has_padding = first_4 in [b'\x00\x00\x00\x00', b'\xff\xff\xff\xff']
    print(f"  {name}: first 4 bytes = {[hex(b) for b in first_4]} padding={has_padding}")

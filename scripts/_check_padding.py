import struct, os

files_to_check = [
    'tunner_logo_S/ferrari.img',
    'tunner_logo_M/ferrari.img',
    'course_image/akasaka.img',
    'course_map_menu/akasaka.img',
    'mission_flyer/mission_a.img',
    'course_map_race/20r60r_ps2.img',
    'course_logo_S/20r60r.img',
    'license_bg/0.img',
    'license_course_map_menu/license_User000.img',
    'env/env2.txs',
]

base = r'files/decompiled/Gran Turismo/PSP_GAME/USRDIR/GT.VOL/piece_gt5m'
print("Checking for 4-byte padding at data_ptr:")
for rel in files_to_check:
    fp = os.path.join(base, rel)
    with open(fp, 'rb') as f:
        data = f.read()
    
    name = os.path.basename(fp)
    magic = data[0:4]
    is_le = magic == b'3SXT'
    fmts = '<' if is_le else '>'
    img_ptr = struct.unpack(fmts + 'I', data[28:32])[0]
    img_info = data[img_ptr:img_ptr+32]
    data_ptr = struct.unpack(fmts + 'I', img_info[0:4])[0]
    
    first_4 = data[data_ptr:data_ptr+4]
    is_all_zero = all(b == 0 for b in first_4)
    is_all_ff = all(b == 0xFF for b in first_4)
    has_padding = is_all_zero or is_all_ff
    label = "ZEROS" if is_all_zero else ("0xFF" if is_all_ff else str([hex(b) for b in first_4]))
    print(f"  {rel}: {label} padding={has_padding}")

# The key finding: files with img_ptr=0x11c have NO padding
# Files with img_ptr=0x12c have 4-byte padding
# Let's verify this pattern
print("\n\nChecking img_ptr value vs padding:")
for rel in files_to_check:
    fp = os.path.join(base, rel)
    with open(fp, 'rb') as f:
        data = f.read()
    
    name = os.path.basename(fp)
    magic = data[0:4]
    is_le = magic == b'3SXT'
    fmts = '<' if is_le else '>'
    pglue_ptr = struct.unpack(fmts + 'I', data[24:28])[0]
    img_ptr = struct.unpack(fmts + 'I', data[28:32])[0]
    img_info = data[img_ptr:img_ptr+32]
    data_ptr = struct.unpack(fmts + 'I', img_info[0:4])[0]
    
    first_4 = data[data_ptr:data_ptr+4]
    is_all_zero = all(b == 0 for b in first_4)
    is_all_ff = all(b == 0xFF for b in first_4)
    
    # data_ptr relative to img_ptr
    data_rel = data_ptr - img_ptr
    print(f"  {rel}: img_ptr=0x{img_ptr:x} data_rel={data_rel} pad={is_all_zero or is_all_ff}")

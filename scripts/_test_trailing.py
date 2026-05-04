import sys, os, struct
sys.path.insert(0, os.getcwd())
from scripts.convert_textures import TXS3Converter
import numpy as np
from PIL import Image

base = r'D:\GTPSP-decompile\files\decompiled\Gran Turismo\PSP_GAME\USRDIR\GT.VOL\piece_gt5m'
out_dir = r'D:\GTPSP-decompile\converted\textures\all'

# Check volvo.img trailing data
fp = os.path.join(base, 'car_image', 'volvo.img')
with open(fp, 'rb') as f:
    d = f.read()

img_ptr = struct.unpack('<I', d[28:32])[0]
dp = struct.unpack('<I', d[img_ptr:img_ptr+4])[0]
ds = struct.unpack('<I', d[img_ptr+4:img_ptr+8])[0]
fmt = d[img_ptr+9]
w = struct.unpack('<H', d[img_ptr+12:img_ptr+14])[0]
h = struct.unpack('<H', d[img_ptr+14:img_ptr+16])[0]
byte8 = d[img_ptr+8]

trailing = d[dp+ds:]
print(f"volvo.img: fmt=0x{fmt:02x} {w}x{h} data_ptr=0x{dp:x} data_size={ds} byte8=0x{byte8:02x}")
print(f"Trailing: {len(trailing)} bytes")
print(f"First 64 bytes: {trailing[:64].hex()}")

# The 150x114 and 100x76 versions are DIFFERENT images within the same file!
# Actually no — both are parsed from the same file? No, there are TWO volvo.img entries
# Let me check: the PGLU contains multiple ImageInfo entries
# Our parse_header only reads the FIRST ImageInfo

# Actually, let me check: there might be multiple textures in one VOL file entry
# The PGLU pointer list might have multiple entries
# Or: there are two car_image directories: one inside another?

# Let me look for ALL volvo.img files
import glob
for fp2 in sorted(glob.glob(os.path.join(base, '**', 'volvo.img'), recursive=True)):
    print(f"\n{os.path.relpath(fp2, base)}")
    with open(fp2, 'rb') as f:
        d2 = f.read()
    tx_ptr = struct.unpack('<I', d2[16:20])[0]
    img_ptr = struct.unpack('<I', d2[28:32])[0]
    
    # Read all ImageInfo entries in the PGLU
    # Byte 0x1A = img_count
    img_count = struct.unpack('<H', d2[0x1A:0x1C])[0]
    pglue_ptr = struct.unpack('<I', d2[24:28])[0]
    
    print(f"  img_count={img_count}, pglue_ptr=0x{pglue_ptr:x}, img_ptr=0x{img_ptr:x}")
    
    # Check if there are multiple ImageInfos
    # Read consecutive ImageInfo structs starting from img_ptr
    for i in range(img_count if img_count > 0 else 1):
        off = img_ptr + i * 32
        ii = d2[off:off+32]
        dp2 = struct.unpack('<I', ii[0:4])[0]
        ds2 = struct.unpack('<I', ii[4:8])[0]
        fmt2 = ii[9]
        w2 = struct.unpack('<H', ii[12:14])[0]
        h2 = struct.unpack('<H', ii[14:16])[0]
        trailing2 = d2[dp2+ds2:]
        print(f"  [{i}] fmt=0x{fmt2:02x} {w2}x{h2} dp=0x{dp2:x} ds={ds2} trailing={len(trailing2)} trailing_start={trailing2[:16].hex()}")

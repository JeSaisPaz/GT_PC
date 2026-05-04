import sys, os, struct
sys.path.insert(0, os.getcwd())
import numpy as np
from PIL import Image

base = r'D:\GTPSP-decompile\files\decompiled\Gran Turismo\PSP_GAME\USRDIR\GT.VOL\piece_gt5m'

# Check tunner_logo_M/volvo.img — it has img_count > 1
fp = os.path.join(base, 'tunner_logo_M', 'volvo.img')
with open(fp, 'rb') as f:
    d = f.read()

# Full header structure
print(f"File: volvo.img ({len(d)} bytes)")
print(f"First 32 bytes: {d[:32].hex()}")

magic = d[0:4]
file_size = struct.unpack('<I', d[4:8])[0]
img_count = struct.unpack('<H', d[0x1A:0x1C])[0]
pglue_ptr = struct.unpack('<I', d[24:28])[0]
img_ptr = struct.unpack('<I', d[28:32])[0]

print(f"Magic: {magic}, file_size=0x{file_size:x}, img_count={img_count}")
print(f"pglue_ptr=0x{pglue_ptr:x}, img_ptr=0x{img_ptr:x}")

# The PGLU has entries that point to ImageInfos
# Read the PGLU entries
pglue_data = d[pglue_ptr:pglue_ptr+32]
print(f"\nPGLU data: {pglue_data.hex()}")

# The pglue_count at offset 0x18
pglue_count = struct.unpack('<H', d[0x18:0x1A])[0]
print(f"pglue_count={pglue_count}, img_count={img_count}")

# Read each ImageInfo (each 32 bytes)
for i in range(max(pglue_count, img_count)):
    off = img_ptr + i * 32
    if off + 32 > len(d):
        break
    ii = d[off:off+32]
    dp = struct.unpack('<I', ii[0:4])[0]
    ds = struct.unpack('<I', ii[4:8])[0]
    b8 = ii[8]
    fmt = ii[9]
    w = struct.unpack('<H', ii[12:14])[0]
    h = struct.unpack('<H', ii[14:16])[0]
    trailing = d[dp+ds:]
    print(f"\n[{i}] ImageInfo at 0x{off:x}")
    print(f"    data_ptr=0x{dp:x} data_size={ds} byte8=0x{b8:02x} fmt=0x{fmt:02x} {w}x{h}")
    print(f"    trailing={len(trailing)} bytes")
    print(f"    trailing start: {trailing[:32].hex()}")
    
    # Try to find palette for each
    if fmt == 0x05:
        if len(trailing) >= 0x10 + 4:
            # Check what's at different offsets
            for po in range(0, min(256, len(trailing)-4), 4):
                val = struct.unpack('<I', trailing[po:po+4])[0]
                if val == dp + 0x10:  # Matches the expected pattern (palette_ptr = data_ptr + 0x10)
                    print(f"    PALETTE POINTER FOUND at trailing offset 0x{po:x}: value=0x{val:08x}")
                if po == 0x10 and len(trailing) >= 0x10 + 1024:
                    pal = trailing[0x10:0x10+1024]
                    unique = len(set(tuple(pal[j:j+4]) for j in range(0, 1024, 4)))
                    print(f"    Palette at offset 0x10: {unique} unique colors")
                if po == 0:
                    # Check if first 4 bytes look like palette size
                    w0 = struct.unpack('<H', trailing[0:2])[0]
                    w1 = struct.unpack('<H', trailing[2:4])[0]
                    print(f"    First word: 0x{w0:04x} ({w0}), Second word: 0x{w1:04x} ({w1})")

import sys, os, struct
sys.path.insert(0, os.getcwd())
import numpy as np
from PIL import Image

base = r'D:\GTPSP-decompile\files\decompiled\Gran Turismo\PSP_GAME\USRDIR\GT.VOL\piece_gt5m'
out_dir = r'D:\GTPSP-decompile\converted\textures\all'
os.makedirs(out_dir, exist_ok=True)

fp = os.path.join(base, 'tunner_logo_M', 'volvo.img')
with open(fp, 'rb') as f:
    d = f.read()

img_ptr = struct.unpack('<I', d[28:32])[0]
dp = struct.unpack('<I', d[img_ptr:img_ptr+4])[0]
ds = struct.unpack('<I', d[img_ptr+4:img_ptr+8])[0]
fmt = d[img_ptr+9]
w = struct.unpack('<H', d[img_ptr+12:img_ptr+14])[0]
h = struct.unpack('<H', d[img_ptr+14:img_ptr+16])[0]

trailing = d[dp+ds:]
print(f"volvo.img (M): fmt=0x{fmt:02x} {w}x{h} dp=0x{dp:x} ds={ds} trailing={len(trailing)}")

# Parse trailing header
pal_size_flag = struct.unpack('<H', trailing[0:2])[0]  # 0x0300
num_colors = struct.unpack('<H', trailing[2:4])[0]      # 0x0011 = 17
pal_ptr = struct.unpack('<I', trailing[4:8])[0]         # 0x7960
print(f"Trailing header: pal_size=0x{pal_size_flag:04x} colors={num_colors} pal_ptr=0x{pal_ptr:08x}")

# The palette is at trailing offset 0x10, but only num_colors entries
pal_size_bytes = num_colors * 4
pal_data = trailing[0x10:0x10 + pal_size_bytes]
palette = np.frombuffer(pal_data, dtype=np.uint8).reshape(num_colors, 4)
print(f"Palette at trailing+0x10: {pal_size_bytes} bytes, {num_colors} colors")
print(f"First 3 colors: {palette[:3].tolist()}")

# Decode as PSMT8 with 17-color palette (indices 0..16, rest are 0)
# First: padded_w = 2^ceil(log2(150)) = 256
# padded_h = 114
import math
padded_w = 1 << (w - 1).bit_length()
padded_h = h
print(f"Padded: {padded_w}x{padded_h}")

pixel_data = d[dp:dp+ds]
expected = padded_w * padded_h
data = pixel_data[:expected].ljust(expected, b'\x00')

# De-swizzle (16x8 blocks)
output = np.zeros((padded_h, padded_w), dtype=np.uint8)
idx = 0
for sy in range(0, padded_h, 8):
    for sx in range(0, padded_w, 16):
        wb = min(16, padded_w - sx)
        hb = min(8, padded_h - sy)
        for by in range(hb):
            for bx in range(wb):
                output[sy + by, sx + bx] = data[idx]
                idx += 1

# Apply palette - remap any index >= num_colors to 0
indices = output.copy()
indices[indices >= num_colors] = 0

# Full palette: extend to 256 entries by repeating black
full_palette = np.zeros((256, 4), dtype=np.uint8)
full_palette[:num_colors] = palette

result = full_palette[indices]
result_cropped = result[:h, :w]
Image.fromarray(result_cropped).save(os.path.join(out_dir, 'volvo_M_TEST.png'))
print(f"Saved: volvo_M_TEST.png ({w}x{h})")
print(f"Unique indices used: {len(np.unique(output))}")

import struct, os, numpy as np
from PIL import Image

base = r'files/decompiled/Gran Turismo/PSP_GAME/USRDIR/GT.VOL/piece_gt5m'

# mission_flyer: 160x112 fmt=0x05 data_sz=28672
# 28672/2 = 14336 pixels
# Possible: 112x128 (112*128=14336) or 128x112 (128*112=14336)
# Which one is correct? The header says 160x112, so height=112 is likely correct.
# Then width = 14336/112 = 128. So 128x112 is the correct interpretation.
# Let's visually compare

fp = os.path.join(base, 'mission_flyer/mission_a.img')
with open(fp, 'rb') as f:
    data = f.read()

img_info = data[0x12c:0x12c+32]
data_ptr = struct.unpack('<I', img_info[0:4])[0]
data_size = struct.unpack('<I', img_info[4:8])[0]
raw = data[data_ptr:data_ptr+data_size]

# 128x112 is more likely (keeps height from header)
arr = np.frombuffer(raw, dtype=np.uint16).reshape(112, 128)
r = ((arr >> 12) & 0xF) * 17
g = ((arr >> 8) & 0xF) * 17
b = ((arr >> 4) & 0xF) * 17
a = (arr & 0xF) * 17
rgba = np.stack([r, g, b, a], axis=-1).astype(np.uint8)
Image.fromarray(rgba).save('converted/mission_a_128x112_RGBA4444_correct.png')
print("Saved mission_a as 128x112 (preserving header height)")

# Check course_map_menu/akasaka: 100x76 fmt=0x04 data_sz=5120  
# 5120/2 = 2560 pixels
# Possible: 80x32 (80*32=2560) - this seems very wrong
# What about data_size=5120 with bpp=1? L8: 5120 = 80x64 or 64x80  
# But header says fmt=0x04 (RGB565, 2bpp)
# Let's check if the format byte could be wrong
fp = os.path.join(base, 'course_map_menu/akasaka.img')
with open(fp, 'rb') as f:
    data = f.read()

img_info = data[0x12c:0x12c+32]
data_ptr = struct.unpack('<I', img_info[0:4])[0]
data_size = struct.unpack('<I', img_info[4:8])[0]
w = struct.unpack('<H', img_info[12:14])[0]
h = struct.unpack('<H', img_info[14:16])[0]
fmt_byte = img_info[9]
raw = data[data_ptr:data_ptr+data_size]

print(f"\ncourse_map_menu/akasaka: header={w}x{h} fmt=0x{fmt_byte:02X} data_sz={data_size}")
print(f"  L4 would be: {w*h//2} = expected {w*h//2}, is {data_size}? {w*h//2==data_size if w*h//2==data_size else 'no'}")
print(f"  L8 would be: {w*h} = expected {w*h}, is {data_size}? {w*h==data_size if w*h==data_size else 'no'}")

# Try all formats
for fmt_name, bpp, fmt_val in [('L4', 0.5, 8), ('L8', 1, 7), ('RGB565', 2, 4), ('RGBA4444', 2, 5)]:
    expected = int(w * h * bpp)
    if expected == data_size:
        print(f"  MATCH: {w}x{h} {fmt_name} (bpp={bpp})")
        if fmt_name == 'L8':
            arr = np.frombuffer(raw, dtype=np.uint8).reshape(h, w)
            rgb = np.stack([arr, arr, arr], axis=-1).astype(np.uint8)
            Image.fromarray(rgb).save(f'converted/akasaka_map_{w}x{h}_L8.png')
            print(f"    Saved as L8")
        elif fmt_name == 'L4':
            arr = np.frombuffer(raw, dtype=np.uint8)
            high = (arr >> 4) & 0xF
            low = arr & 0xF
            pixels = np.empty(h*w, dtype=np.uint8)
            pixels[0::2] = high
            pixels[1::2] = low
            gray = pixels.reshape(h, w) * 17
            rgb = np.stack([gray, gray, gray], axis=-1).astype(np.uint8)
            Image.fromarray(rgb).save(f'converted/akasaka_map_{w}x{h}_L4.png')
            print(f"    Saved as L4")

# Since fmt=0x04 but data doesn't match 100x76x2, maybe the format byte is wrong
# 5120 = 100*76*0.673... hmm
# 5120 = 80*64*1 = L8 of 80x64
# Let's also try: maybe data is 80x64 L8
for tw in range(1, 200):
    if data_size % tw == 0:
        for bpp in [0.5, 1, 2]:
            px = int(data_size / bpp)
            if px == tw * (px // tw):
                th = px // tw
                if 1 <= th <= 200 and (abs(tw - w) < 50 or abs(th - h) < 50):
                    bpp_name = {0.5: 'L4', 1: 'L8', 2: 'RGB565/RGBA4444'}[bpp]
                    print(f"  Alt: {tw}x{th} {bpp_name} (bpp={bpp})")

# Also, the test_map_80x64_L8.png from earlier looked somewhat reasonable (107 colors)
# Let's verify: 80*64 = 5120 = data_size. So it COULD be L8 80x64 with wrong format byte!
print(f"\n  Trying 80x64 L8...")
arr = np.frombuffer(raw, dtype=np.uint8).reshape(64, 80)
print(f"  As 80x64 L8: min={arr.min()} max={arr.max()} unique={len(np.unique(arr))}")
rgb = np.stack([arr, arr, arr], axis=-1).astype(np.uint8)
Image.fromarray(rgb).save('converted/akasaka_map_80x64_L8.png')
print(f"  Saved akasaka_map_80x64_L8.png")

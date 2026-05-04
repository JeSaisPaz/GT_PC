from scripts.convert_textures import TXS3Converter
import numpy as np
from PIL import Image

base = r'D:\GTPSP-decompile\files\decompiled\Gran Turismo\PSP_GAME\USRDIR\GT.VOL\piece_gt5m'
out_dir = r'D:\GTPSP-decompile\converted\textures'
import os
os.makedirs(out_dir, exist_ok=True)

# Test akasaka
print("=== Testing akasaka.img ===")
ok = TXS3Converter.convert_txs3_to_png(
    os.path.join(base, 'course_image/akasaka.img'),
    os.path.join(out_dir, '_verify_akasaka.png'),
    force=True
)
print(f"Result: {ok}")

# Compare with official
official = np.array(Image.open(os.path.join(out_dir, 'akasaka_OFFICIAL.png')))
our = np.array(Image.open(os.path.join(out_dir, '_verify_akasaka.png')))
diff = np.abs(our.astype(int) - official.astype(int)).mean()
print(f"akasaka diff from official: {diff:.1f}")
if diff == 0:
    print("*** PERFECT MATCH ***")
else:
    wrong = (np.abs(our.astype(int) - official.astype(int)).max(axis=2) > 0).sum()
    total = 160 * 128
    print(f"Wrong pixels: {wrong}/{total} ({100*wrong/total:.1f}%)")

# Test cathedralRocks1
print("\n=== Testing cathedralRocks1.img ===")
ok = TXS3Converter.convert_txs3_to_png(
    os.path.join(base, 'course_image/cathedralRocks1.img'),
    os.path.join(out_dir, '_verify_cathedralRocks1.png'),
    force=True
)
print(f"Result: {ok}")

# Test license_bg format 0x08
print("\n=== Testing license_bg/0.img ===")
ok = TXS3Converter.convert_txs3_to_png(
    os.path.join(base, 'license_bg/0.img'),
    os.path.join(out_dir, '_verify_license0.png'),
    force=True
)
print(f"Result: {ok}")

# Test license_bg/1.img too
print("\n=== Testing license_bg/1.img ===")
ok = TXS3Converter.convert_txs3_to_png(
    os.path.join(base, 'license_bg/1.img'),
    os.path.join(out_dir, '_verify_license1.png'),
    force=True
)
print(f"Result: {ok}")

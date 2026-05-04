import struct, os, numpy as np
from PIL import Image

converted = 'converted'
print("Texture analysis report:")
print("="*70)
for f in sorted(os.listdir(converted)):
    if not f.endswith('.png'):
        continue
    fp = os.path.join(converted, f)
    img = Image.open(fp)
    arr = np.array(img)
    h, w = arr.shape[:2]
    mode = img.mode
    channels = 1 if len(arr.shape) == 2 else arr.shape[2]
    
    if channels > 1:
        flat = arr.reshape(-1, channels)
        unique_colors = len(np.unique(flat.view(dtype=np.void(flat.dtype.itemsize * flat.shape[1])))
                          .view(dtype=flat.dtype).reshape(-1, channels))
    else:
        unique_colors = len(np.unique(arr))
    
    total_pixels = h * w
    
    issues = []
    if unique_colors <= 2:
        issues.append("ONLY 2 COLORS")
    if unique_colors == 1:
        issues.append("SOLID COLOR")
    if total_pixels <= 100:
        issues.append("VERY SMALL")
    if w > 2048 or h > 2048:
        issues.append("UNUSUALLY LARGE")
    
    status = "OK" if not issues else " | ".join(issues)
    print(f"  {f:50s} {w:4}x{h:<4} {mode:5s} colors={unique_colors:5d} {status}")

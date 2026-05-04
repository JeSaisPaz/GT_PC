import struct
from pathlib import Path
import numpy as np
from PIL import Image

def test_decode():
    fp = 'files/decompiled/Gran Turismo/PSP_GAME/USRDIR/GT.VOL/piece_gt5m/tunner_logo_S/ferrari.img'
    
    with open(fp, 'rb') as f:
        data = f.read()
    
    print(f"File size: {len(data)}")
    print(f"Magic: {data[0:4]}")
    
    magic = data[0:4]
    is_le = magic == b'3SXT'
    fmt = '<' if is_le else '>'
    
    img_ptr = struct.unpack(fmt + 'I', data[0x1C:0x20])[0]
    print(f"ImageInfo ptr: {img_ptr}")
    
    img_info = data[img_ptr:img_ptr+32]
    w = struct.unpack(fmt + 'H', img_info[12:14])[0]
    h = struct.unpack(fmt + 'H', img_info[14:16])[0]
    print(f"Dimensions: {w}x{h}")
    
    data_start = 0x100
    raw = data[data_start:data_start + w*h]
    unique = len(set(raw))
    print(f"L8 unique values: {unique}")
    print(f"First 32 bytes: {list(raw[:32])}")
    
    arr = np.frombuffer(raw, dtype=np.uint8).reshape(h, w)
    print(f"Array shape: {arr.shape}")
    print(f"Min/Max: {arr.min()}, {arr.max()}")
    
    rgb = np.stack([arr, arr, arr], axis=-1)
    img = Image.fromarray(rgb.astype(np.uint8))
    img.save('converted/test_ferrari.png')
    print("Saved!")

test_decode()
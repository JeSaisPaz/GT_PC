import struct
import numpy as np
from PIL import Image
from pathlib import Path

def decode_with_l4():
    base = 'files/decompiled/Gran Turismo/PSP_GAME/USRDIR/GT.VOL/'
    
    files = [
        'piece_gt5m/license_bg/0.img',
        'piece_gt5m/tunner_logo_S/ferrari.img',
        'piece_gt5m/course_image/akasaka.img',
    ]
    
    for path in files:
        fp = base + path
        with open(fp, 'rb') as f:
            data = f.read()
        
        name = Path(fp).name
        
        magic = data[0:4]
        fmt = '<' if magic == b'3SXT' else '>'
        
        img_ptr = struct.unpack(fmt + 'I', data[0x1C:0x20])[0]
        img_info = data[img_ptr:img_ptr+32]
        
        w = struct.unpack(fmt + 'H', img_info[12:14])[0]
        h = struct.unpack(fmt + 'H', img_info[14:16])[0]
        data_size = struct.unpack(fmt + 'I', img_info[4:8])[0]
        
        print(f'\n{name}: {w}x{h}, header_data_size={data_size}')
        
        # Use data_size from header to decode
        raw = data[0x100:0x100 + data_size]
        
        if data_size == w * h // 2:
            # L4
            arr = np.frombuffer(raw, dtype=np.uint8)
            high = (arr >> 4) & 0xF
            low = arr & 0xF
            pixels = np.empty(h*w, dtype=np.uint8)
            pixels[0::2] = high
            pixels[1::2] = low
            arr = pixels.reshape(h, w) * 17
            print('  Decoded as L4')
        elif data_size == w * h:
            # L8
            arr = np.frombuffer(raw, dtype=np.uint8).reshape(h, w)
            print('  Decoded as L8')
        elif data_size == w * h * 2:
            # RGB565
            arr = np.frombuffer(raw, dtype=np.uint16).reshape(h, w)
            r = ((arr >> 11) & 0x1F) * 255 // 31
            g = ((arr >> 5) & 0x3F) * 255 // 63
            b = (arr & 0x1F) * 255 // 31
            arr = np.stack([r, g, b], axis=-1).astype(np.uint8)
            print('  Decoded as RGB565')
            img = Image.fromarray(arr)
            img.save(f'converted/test_{name}.png')
            print(f'  Saved as RGB565')
            continue
        else:
            # Try with actual dimensions
            arr = np.frombuffer(raw, dtype=np.uint8).reshape(h, w)
            print(f'  Decoded raw as {h}x{w}')
        
        rgb = np.stack([arr, arr, arr], axis=-1)
        img = Image.fromarray(rgb.astype(np.uint8))
        img.save(f'converted/test_{name}.png')
        print(f'  Saved test_{name}.png')

decode_with_l4()
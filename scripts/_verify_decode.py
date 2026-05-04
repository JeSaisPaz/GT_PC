import struct, os, numpy as np
from PIL import Image

base = r'files/decompiled/Gran Turismo/PSP_GAME/USRDIR/GT.VOL/piece_gt5m'
out = 'converted'
os.makedirs(out, exist_ok=True)

def decode_save(name, raw, w, h, fmt_byte):
    try:
        if fmt_byte == 8:  # L4
            arr = np.frombuffer(raw, dtype=np.uint8)
            high = (arr >> 4) & 0xF
            low = arr & 0xF
            pixels = np.empty(h * w, dtype=np.uint8)
            pixels[0::2] = high
            pixels[1::2] = low
            gray = pixels.reshape(h, w) * 17
            rgb = np.stack([gray, gray, gray], axis=-1).astype(np.uint8)
            fpath = os.path.join(out, f'{name}_{w}x{h}_L4.png')
            Image.fromarray(rgb).save(fpath)
            return fpath
        elif fmt_byte == 7:  # L8
            arr = np.frombuffer(raw, dtype=np.uint8).reshape(h, w)
            rgb = np.stack([arr, arr, arr], axis=-1).astype(np.uint8)
            fpath = os.path.join(out, f'{name}_{w}x{h}_L8.png')
            Image.fromarray(rgb).save(fpath)
            return fpath
        elif fmt_byte == 4:  # RGB565
            arr = np.frombuffer(raw, dtype=np.uint16).reshape(h, w)
            r = ((arr >> 11) & 0x1F) * 255 // 31
            g = ((arr >> 5) & 0x3F) * 255 // 63
            b = (arr & 0x1F) * 255 // 31
            rgb = np.stack([r, g, b], axis=-1).astype(np.uint8)
            fpath = os.path.join(out, f'{name}_{w}x{h}_RGB565.png')
            Image.fromarray(rgb).save(fpath)
            return fpath
        elif fmt_byte == 5:  # RGBA4444
            arr = np.frombuffer(raw, dtype=np.uint16).reshape(h, w)
            r = ((arr >> 12) & 0xF) * 17
            g = ((arr >> 8) & 0xF) * 17
            b = ((arr >> 4) & 0xF) * 17
            a = (arr & 0xF) * 17
            rgba = np.stack([r, g, b, a], axis=-1).astype(np.uint8)
            fpath = os.path.join(out, f'{name}_{w}x{h}_RGBA4444.png')
            Image.fromarray(rgba).save(fpath)
            return fpath
        elif fmt_byte == 3:  # RGBA5551
            arr = np.frombuffer(raw, dtype=np.uint16).reshape(h, w)
            r = ((arr >> 11) & 0x1F) * 255 // 31
            g = ((arr >> 6) & 0x1F) * 255 // 31
            b = ((arr >> 1) & 0x1F) * 255 // 31
            a = (arr & 0x1) * 255
            rgba = np.stack([r, g, b, a], axis=-1).astype(np.uint8)
            fpath = os.path.join(out, f'{name}_{w}x{h}_RGBA5551.png')
            Image.fromarray(rgba).save(fpath)
            return fpath
        else:
            print(f'  Unsupported fmt=0x{fmt_byte:02X}')
            return None
    except Exception as e:
        print(f'  Decode error: {e}')
        return None

textures = [
    'license_bg/0.img',
    'tunner_logo_S/ferrari.img',
    'tunner_logo_M/ferrari.img',
    'course_image/akasaka.img',
    'course_map_menu/akasaka.img',
    'mission_flyer/mission_a.img',
    'tunner_logo_S/nissan.img',
    'course_map_race/20r60r_ps2.img',
    'course_logo_S/20r60r.img',
    'env/env2.txs',
    'course_map_menu_S/20r60r.img',
    'course_logo_SS/20r60r.img',
    'license_course_map_menu/license_User000.img',
]

bpp_table = {1:4, 2:4, 3:2, 4:2, 5:2, 6:2, 7:1, 8:0.5, 9:1}

for rel in textures:
    fp = os.path.join(base, rel)
    if not os.path.exists(fp):
        print(f'\n{rel}: FILE NOT FOUND')
        continue
    
    with open(fp, 'rb') as f:
        data = f.read()
    
    name = os.path.basename(fp).replace('.img', '').replace('.txs', '')
    magic = data[0:4]
    
    if magic not in [b'TXS3', b'3SXT']:
        print(f'\n{rel}: NOT TXS3')
        continue
    
    is_le = magic == b'3SXT'
    fmts = '<' if is_le else '>'
    img_ptr = struct.unpack(fmts + 'I', data[28:32])[0]
    
    if img_ptr < 0x100 or img_ptr > len(data) - 32:
        print(f'\n{rel}: invalid img_ptr=0x{img_ptr:x}')
        continue
    
    img_info = data[img_ptr:img_ptr+32]
    data_ptr = struct.unpack(fmts + 'I', img_info[0:4])[0]
    data_size = struct.unpack(fmts + 'I', img_info[4:8])[0]
    fmt_byte = img_info[9]
    hdr_w = struct.unpack(fmts + 'H', img_info[12:14])[0]
    hdr_h = struct.unpack(fmts + 'H', img_info[14:16])[0]
    
    bpp = bpp_table.get(fmt_byte, 2)
    expected_hdr = int(hdr_w * hdr_h * bpp)
    actual_pixels = int(data_size / bpp)
    
    print(f'\n{rel} ({len(data)} bytes):')
    print(f'  HDR: {hdr_w}x{hdr_h} fmt=0x{fmt_byte:02X}({bpp}bpp) dp=0x{data_ptr:x} sz={data_size} file_sz={len(data)}')
    print(f'  Expected(hdr): {expected_hdr}, data_size: {data_size}')
    
    # Determine actual dimensions
    act_w, act_h = hdr_w, hdr_h
    if expected_hdr != data_size:
        found = False
        for tw in range(max(1, hdr_w-64), min(4097, hdr_w+64)):
            if actual_pixels % tw == 0:
                th = actual_pixels // tw
                if 1 <= th <= 4096:
                    if not found:
                        act_w, act_h = tw, th
                        found = True
                    elif abs(tw - hdr_w) + abs(th - hdr_h) < abs(act_w - hdr_w) + abs(act_h - hdr_h):
                        act_w, act_h = tw, th
        if found:
            print(f'  REAL DIMS: {act_w}x{act_h} (from data_size={data_size}, bpp={bpp})')
        else:
            print(f'  Could not determine real dimensions!')
            continue
    
    raw = data[data_ptr:data_ptr+data_size]
    fpath = decode_save(name, raw, act_w, act_h, fmt_byte)
    if fpath:
        print(f'  -> {fpath}')

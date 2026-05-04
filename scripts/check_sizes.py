import struct

files = [
    'files/decompiled/Gran Turismo/PSP_GAME/USRDIR/GT.VOL/piece_gt5m/license_bg/0.img',
    'files/decompiled/Gran Turismo/PSP_GAME/USRDIR/GT.VOL/piece_gt5m/tunner_logo_S/ferrari.img',
    'files/decompiled/Gran Turismo/PSP_GAME/USRDIR/GT.VOL/piece_gt5m/tunner_logo_M/ferrari.img',
]

for fp in files:
    with open(fp, 'rb') as f:
        data = f.read()
    
    magic = data[0:4]
    fmt = '<' if magic == b'3SXT' else '>'
    
    img_ptr = struct.unpack(fmt + 'I', data[0x1C:0x20])[0]
    img_info = data[img_ptr:img_ptr+32]
    
    data_size = struct.unpack(fmt + 'I', img_info[4:8])[0]
    fmt_byte = img_info[9]
    w = struct.unpack(fmt + 'H', img_info[12:14])[0]
    h = struct.unpack(fmt + 'H', img_info[14:16])[0]
    
    l4 = w*h//2
    l8 = w*h
    
    actual = 'L4' if data_size == l4 else ('L8' if data_size == l8 else 'OTHER')
    
    print(f'{fp.split("/")[-1]}: {w}x{h}, data={data_size}, L4={l4}, L8={l8} => {actual}')
import struct
import numpy as np
from PIL import Image
from pathlib import Path

def decode_txs3_with_palette(filepath, output_path=None):
    with open(filepath, 'rb') as f:
        data = f.read()
    
    name = Path(filepath).name
    magic = data[0:4]
    is_le = magic == b'3SXT'
    fmt = '<' if is_le else '>'
    
    file_size = struct.unpack(fmt + 'I', data[4:8])[0]
    pglue_ptr = struct.unpack(fmt + 'I', data[0x18:0x1C])[0]
    img_ptr = struct.unpack(fmt + 'I', data[0x1C:0x20])[0]
    
    img_info = data[img_ptr:img_ptr+32]
    data_ptr = struct.unpack(fmt + 'I', img_info[0:4])[0]
    data_size = struct.unpack(fmt + 'I', img_info[4:8])[0]
    format_byte = img_info[9]
    width = struct.unpack(fmt + 'H', img_info[0x0C:0x0E])[0]
    height = struct.unpack(fmt + 'H', img_info[0x0E:0x10])[0]
    
    print(f'{name}: {width}x{height}, format=0x{format_byte:02X}, data_size={data_size}')
    
    # Extract palette from PGLU section
    # PGLU pointer points to palette data
    palette = []
    palette_start = pglue_ptr + 0x40  # Skip header
    
    # Read up to 256 colors from palette area
    for i in range(256):
        palette_offset = pglue_ptr + 4 + (i * 4)
        if palette_offset + 4 <= len(data):
            try:
                # Try RGB888 format first
                r = data[palette_offset]
                g = data[palette_offset + 1]
                b = data[palette_offset + 2]
                a = data[palette_offset + 3]
                palette.append((r, g, b, a))
            except:
                palette.append((0, 0, 0, 255))
        else:
            palette.append((0, 0, 0, 255))
    
    # Count non-default colors
    valid_colors = [c for c in palette if c != (0, 0, 0, 255) and c != (0, 0, 0, 0)]
    print(f'Palette: {len(valid_colors)} valid colors found')
    
    # Read pixel indices from actual data location (0x100)
    pixel_data = data[0x100:0x100 + data_size]
    
    # Try different interpretations based on format byte
    if format_byte == 0x08:  # L4
        expected_size = width * height // 2
    elif format_byte == 0x07:  # L8
        expected_size = width * height
    elif format_byte == 0x05:  # RGBA4444
        expected_size = width * height * 2
    elif format_byte == 0x04:  # RGB565
        expected_size = width * height * 2
    else:
        # Auto-detect based on data_size
        if data_size == width * height // 2:
            expected_size = data_size
        elif data_size == width * height:
            expected_size = data_size
        else:
            expected_size = data_size
    
    if len(pixel_data) >= expected_size:
        pixel_data = pixel_data[:expected_size]
    else:
        print(f'Warning: data size {len(pixel_data)} < expected {expected_size}')
        expected_size = len(pixel_data)
    
    # Apply palette
    indices = np.frombuffer(pixel_data, dtype=np.uint8)
    indices = indices.flatten()[:width * height]
    
    # Map indices to palette colors
    rgba = np.zeros((height, width, 4), dtype=np.uint8)
    for y in range(height):
        for x in range(width):
            idx = y * width + x
            if idx < len(indices):
                palette_idx = min(indices[idx], len(palette) - 1)
                rgba[y, x] = palette[palette_idx]
    
    # Convert to RGB for PNG
    rgb = rgba[:, :, :3]
    
    # Save
    img = Image.fromarray(rgb, 'RGB')
    if output_path:
        img.save(output_path)
        print(f'Saved to {output_path}')
    else:
        img.save(f'converted/test_pal_{name}.png')
        print(f'Saved to converted/test_pal_{name}.png')
    
    # Print sample colors
    print(f'Sample colors:')
    for i in range(min(10, len(valid_colors))):
        print(f'  {valid_colors[i]}')

# Test on files
test_files = [
    'files/decompiled/Gran Turismo/PSP_GAME/USRDIR/GT.VOL/piece_gt5m/license_bg/0.img',
    'files/decompiled/Gran Turismo/PSP_GAME/USRDIR/GT.VOL/piece_gt5m/tunner_logo_S/ferrari.img',
]

for fp in test_files:
    decode_txs3_with_palette(fp)
    print()
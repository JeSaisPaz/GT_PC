import sys
sys.path.insert(0, r'D:\GTPSP-decompile\scripts')
from convert_textures import TXS3Converter
from pathlib import Path

base = r'D:\GTPSP-decompile\files\decompiled\Gran Turismo\PSP_GAME\USRDIR\GT.VOL\piece_gt5m'

# Interactive conversion for a single file
def convert_one(rel_path):
    img_path = Path(base) / rel_path
    out_dir = Path(r'D:\GTPSP-decompile\converted\textures')
    
    header = TXS3Converter.parse_header(str(img_path))
    if not header:
        print(f"Failed to parse header: {img_path}")
        return False
    
    print(f"=== {img_path.name} ===")
    print(f"  Format: {header['format_name']} (0x{header['format_id']:02x})")
    print(f"  Header dims: {header['hdr_w']}x{header['hdr_h']}")
    print(f"  Computed dims: {header['width']}x{header['height']}")
    print(f"  Data size: {header['data_size']} bytes")
    print(f"  Has 4-byte padding: {header['has_padding']}")
    print(f"  Endian: {header['endian']}")
    
    png_path = out_dir / rel_path
    converted = TXS3Converter.convert_txs3_to_png(str(img_path), str(png_path), force=True)
    
    if converted:
        print(f"  Saved to: {png_path}")
        ok = input("  Does this look correct? [Y/n/q]: ").strip().lower()
        if ok == 'q':
            return 'quit'
        elif ok == 'n':
            print("  Marked as potentially corrupted.")
            return False
        return True
    else:
        print("  Conversion failed.")
        return False

if __name__ == '__main__':
    import os
    os.makedirs(r'D:\GTPSP-decompile\converted\textures', exist_ok=True)
    
    # If a file path is provided as argument
    if len(sys.argv) > 1:
        result = convert_one(sys.argv[1])
    else:
        result = convert_one('course_image/akasaka.img')
    
    if result == 'quit':
        print("Quitting.")

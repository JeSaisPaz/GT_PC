import sys, os
sys.path.insert(0, os.getcwd())
from scripts.convert_textures import TXS3Converter

base = r'D:\GTPSP-decompile\files\decompiled\Gran Turismo\PSP_GAME\USRDIR\GT.VOL\piece_gt5m'
out_dir = r'D:\GTPSP-decompile\converted\textures\all'
os.makedirs(out_dir, exist_ok=True)

# Test specific failing files
fail_files = ['volvo.img', 'tvr.img', 'tuningparts.img', 'manufacturer.img', 'env2.txs']
for fn in fail_files:
    for root, dirs, files in os.walk(base):
        if fn in files:
            fp = os.path.join(root, fn)
            h = TXS3Converter.parse_header(fp)
            if h:
                fname = h['format_name']
                w = h['width']
                ht = h['height']
                swz = h['is_swizzled']
                print(f'{fn}: fmt=0x{h["format_id"]:02x} {fname} {w}x{ht} swz={swz}')
            ok = TXS3Converter.convert_txs3_to_png(fp, os.path.join(out_dir, fn + '.png'), force=True)
            print(f'  -> {"OK" if ok else "FAIL"}')

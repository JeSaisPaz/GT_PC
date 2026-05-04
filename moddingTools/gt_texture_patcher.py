#!/usr/bin/env python3
"""
GT PSP Texture Patcher - Main CLI Tool
"""

import argparse
import os
import sys
from pathlib import Path

# Add the current directory to Python path so we can import core
sys.path.insert(0, str(Path(__file__).parent))
from core import TXS3Patcher

def main():
    parser = argparse.ArgumentParser(description='GT PSP Texture Patcher - Edit textures in GT.VOL')
    subparsers = parser.add_subparsers(dest='command', help='Available commands')
    
    # List command
    list_parser = subparsers.add_parser('list', help='List all convertible textures')
    list_parser.add_argument('--dir', help='Path to decompiled GT.VOL directory')
    
    # Extract command
    extract_parser = subparsers.add_parser('extract', help='Extract a single texture to PNG')
    extract_parser.add_argument('img_path', help='Path to .img file relative to GT.VOL root')
    extract_parser.add_argument('--dir', help='Path to decompiled GT.VOL directory')
    extract_parser.add_argument('-o', '--output', help='Output directory (default: alongside original)')
    
    # Extract all command
    extract_all_parser = subparsers.add_parser('extract-all', help='Extract all textures to PNG')
    extract_all_parser.add_argument('--dir', help='Path to decompiled GT.VOL directory')
    extract_all_parser.add_argument('-o', '--output', default='extracted_textures', help='Output directory')
    
    # Replace command
    replace_parser = subparsers.add_parser('replace', help='Replace a texture with a PNG')
    replace_parser.add_argument('img_path', help='Path to .img file relative to GT.VOL root')
    replace_parser.add_argument('png_path', help='Path to replacement PNG file')
    replace_parser.add_argument('--dir', help='Path to decompiled GT.VOL directory')
    replace_parser.add_argument('--no-dither', action='store_true', help='Disable dithering during quantization')
    
    # Replace all command
    replace_all_parser = subparsers.add_parser('replace-all', help='Replace all textures from a directory')
    replace_all_parser.add_argument('--from', dest='src_dir', required=True, help='Source directory with PNG files')
    replace_all_parser.add_argument('--dir', help='Path to decompiled GT.VOL directory')
    replace_all_parser.add_argument('--no-dither', action='store_true', help='Disable dithering during quantization')
    
    # Pack command
    pack_parser = subparsers.add_parser('pack', help='Pack decompiled folder back into GT.VOL')
    pack_parser.add_argument('--dir', help='Path to decompiled GT.VOL directory')
    pack_parser.add_argument('-o', '--output', default='GT_MOD.VOL', help='Output VOL file path')
    
    # Unpack command
    unpack_parser = subparsers.add_parser('unpack', help='Unpack GT.VOL to folder')
    unpack_parser.add_argument('vol_path', help='Path to GT.VOL file')
    unpack_parser.add_argument('-o', '--output', help='Output directory (default: volname.extracted)')
    
    args = parser.parse_args()
    
    if not args.command:
        parser.print_help()
        return
    
    # Determine vol directory
    vol_dir = None
    if hasattr(args, 'dir') and args.dir:
        vol_dir = args.dir
    elif hasattr(args, 'vol_path') and args.vol_path:
        # For unpack command, we don't need vol_dir until after unpacking
        vol_dir = None
    else:
        print("Error: --dir is required for this command")
        return
    
    patcher = TXS3Patcher(vol_dir) if vol_dir else None
    
    if args.command == 'list':
        if not patcher:
            print("Error: No directory specified")
            return
        textures = patcher.list_textures()
        print(f"Found {len(textures)} convertible textures:")
        for tex in textures:
            swz = " [swizzled]" if tex['swizzled'] else ""
            print(f"  {tex['path']}: {tex['format']} {tex['width']}x{tex['height']}{swz}")
    
    elif args.command == 'extract':
        if not patcher:
            print("Error: No directory specified")
            return
        output_dir = args.output if args.output else os.path.dirname(args.img_path)
        success = patcher.extract_texture(args.img_path, output_dir)
        print(f"Extraction {'succeeded' if success else 'failed'}: {args.img_path}")
    
    elif args.command == 'extract-all':
        if not patcher:
            print("Error: No directory specified")
            return
        print(f"Extracting all textures to {args.output}...")
        # This would need implementation - for now just note it
        print("Extract-all not yet implemented in this prototype")
    
    elif args.command == 'replace':
        if not patcher:
            print("Error: No directory specified")
            return
        success = patcher.replace_texture(args.img_path, args.png_path, dither=not args.no_dither)
        print(f"Replacement {'succeeded' if success else 'failed'}: {args.img_path}")
    
    elif args.command == 'replace-all':
        if not patcher:
            print("Error: No directory specified")
            return
        print(f"Replace-all from {args.src_dir} not yet implemented in this prototype")
    
    elif args.command == 'pack':
        vol_dir_to_pack = args.dir if args.dir else vol_dir
        if not vol_dir_to_pack:
            print("Error: No directory specified for packing")
            return
        patcher = TXS3Patcher(vol_dir_to_pack)
        success, output = patcher.pack_vol(args.output)
        if success:
            print(f"Packing succeeded: {args.output}")
        else:
            print(f"Packing failed:\n{output}")
    
    elif args.command == 'unpack':
        print("Unpack command not yet implemented in this prototype")
        print("Use: workflow/GTPSPVolTools/GTPSPVolTools.exe unpack -i <vol_path> -o <output>")

if __name__ == '__main__':
    main()
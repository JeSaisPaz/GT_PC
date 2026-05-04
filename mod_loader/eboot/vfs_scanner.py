#!/usr/bin/env python3
"""
EBOOT VFS Function Scanner
===========================
Searches EBOOT.BIN for references to script loading strings and GT.VOL.

NOTE: The EBOOT's data sections are typically compressed. Most strings
will NOT be visible in the raw binary. You MUST use Ghidra to decompress
and analyze the executable sections.

This tool is a first pass to find any surface-level references.
For thorough analysis, follow mod_loader/eboot/analysis_guide.md.

Usage:
  python vfs_scanner.py <path_to_eboot.bin>

Output:
  - Any found string references (likely none in compressed EBOOT)
  - PPSSPP cheat patch templates for manual filling
"""

import struct
import sys
from pathlib import Path

KNOWN_STRINGS = [
    b"bootstrap", b"packed_main_loop", b"bootstrap_phase2",
    b"shutdown", b".adc", b"GT.VOL",
    b"projects/gt5m/", b"scripts/gt5m/", b"products/gt5m/",
    b"MainLoop", b"onLoad", b"onUnload",
]

GT_VOL_MAGIC = b"GT.VOL"
EBOOT_LOAD_ADDRESS = 0x08800000


def find_strings(data: bytes):
    results = []
    for pattern in KNOWN_STRINGS:
        offset = 0
        while True:
            offset = data.find(pattern, offset)
            if offset == -1:
                break
            start = offset
            while start > 0 and data[start - 1] != 0:
                start -= 1
            results.append({
                "offset": offset,
                "address": EBOOT_LOAD_ADDRESS + offset,
                "value": data[start:offset + len(pattern)].decode('ascii', errors='replace'),
            })
            offset += 1
    return results


def scan(data: bytes, eboot_path: str):
    print(f"EBOOT Scanner: {eboot_path}")
    print(f"File size: {len(data):,} bytes ({len(data)/1024/1024:.1f} MB)")
    print()

    strings = find_strings(data)
    if strings:
        print("=== String References Found ===")
        for s in sorted(strings, key=lambda x: x["offset"]):
            print(f"  [{s['value']}] at 0x{s['address']:08X}")
    else:
        print("=== No Strings Found (Expected) ===")
        print("The EBOOT.BIN data sections are compressed.")
        print("Strings like 'bootstrap', '.adc', 'GT.VOL' are packed and")
        print("only visible after Ghidra decompresses the executable.")
        print()
        print("To proceed:")
        print("  1. Launch Ghidra: workflow/ghidra_12.0_PUBLIC/ghidraRun.bat")
        print("  2. Import EBOOT.BIN as MIPS R4000 Little-Endian")
        print("  3. Auto-analyze (let it decompress all sections)")
        print("  4. Search for string 'bootstrap' -> find the VFS loader")
        print("  5. Fill addresses into: mod_loader/eboot/vfs_addresses.json")
        print()
        print("See: mod_loader/eboot/analysis_guide.md for detailed steps")

    print()
    print("=== PPSSPP Cheat Patch Templates ===")
    print("(Fill in addresses after Ghidra analysis)")
    print()
    print("_S UCES-01245")
    print("_G Gran Turismo (PSP)")
    print("")
    print("_C0 Redirect Script Load to ms0:/PSP/MODS/")
    print("_L 0x???????? 0x????????  (VFS function address + patch value)")
    print("")
    print("_C0 Extended Memory Heap (32MB)")
    print("_L 0x???????? 0x????????  (heap alloc size parameter)")


def main():
    if len(sys.argv) < 2:
        print("Usage: python vfs_scanner.py <path_to_eboot.bin>")
        print("Example: python vfs_scanner.py \"../../files/original/Gran Turismo/PSP_GAME/SYSDIR/EBOOT.BIN\"")
        sys.exit(1)

    eboot_path = Path(sys.argv[1])
    if not eboot_path.exists():
        print(f"ERROR: File not found: {eboot_path}")
        sys.exit(1)

    with open(eboot_path, "rb") as f:
        data = f.read()

    scan(data, str(eboot_path))


if __name__ == "__main__":
    main()

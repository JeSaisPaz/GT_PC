#!/usr/bin/env python3
"""
EBOOT.BIN PRX Header Analyzer
==============================
Analyzes the unencrypted header of the GT PSP EBOOT.BIN.
The EBOOT uses PSP retail PRX encryption — code/data sections 
are encrypted with PSP KIRK engine. Only the header prefix is readable.

Usage:
  python eboot_analyzer.py [path/to/EBOOT.BIN]

After analyzing this, use:
  1. Ghidra (headless or GUI) to decrypt + decompile the full EBOOT
  2. PPSSPP debugger for runtime analysis
  
See: analysis_guide.md for detailed workflows.
"""

import struct
import sys
import os
from pathlib import Path


def u32(data: bytes, offset: int) -> int:
    return struct.unpack_from("<I", data, offset)[0]

def u16(data: bytes, offset: int) -> int:
    return struct.unpack_from("<H", data, offset)[0]

def ascii_str(data: bytes, offset: int, max_len: int = 28) -> str:
    end = offset
    while end < len(data) and end - offset < max_len and data[end] != 0:
        end += 1
    return data[offset:end].decode("ascii", errors="replace")


# ─── PSP PRX Header Format ────────────────────────────────────────────────
# Offset  Size  Description
# 0x00    4     Magic: "~PSP" (0x7E 0x50 0x53 0x50) — identifies PSP PRX
# 0x04    4     Reserved / key-index for PSP KIRK decryption (usually 0)
# 0x08    28    Module name (null-padded), "PDIAPP" at +2 within this field
# 0x24    2     PRX attributes
# 0x26    1     Module version
# 0x27    1     Module attribute modifier
# 0x28    4     Size (may be BOOT.BIN size)
# 0x2C    4     Size (may be EBOOT.BIN size)
# 0x30    4     Number of entries or section count
# 0x34    4     Unknown (maybe decompressed size or data start)
# 0x38    4     Unknown (maybe encrypted data size)
# 0x3C    4     PSP PRX load address / flags
# 0x40-0x7F  Reserved / padding (zeros)
# ─── Decryption boundary ─────────────────────────────────────────────────
# 0x80    --    START OF ENCRYPTED DATA (ELF header + sections encrypted)
#               Ghidra or PPSSPP required to decrypt/access these

KNOWN_HEADER_FIELDS = {
    0x00: ("magic", "4s", "PRX magic '~PSP'"),
    0x04: ("reserved", "I", "Reserved / KIRK key index"),
    0x08: ("module_name", "28s", "Module name (padded)"),
    0x24: ("attributes", "H", "PRX attributes"),
    0x26: ("module_version", "B", "Module version"),
    0x27: ("attr_modifier", "B", "Attribute modifier"),
    0x28: ("size_boot", "I", "BOOT.BIN size?"),
    0x2C: ("size_eboot", "I", "EBOOT.BIN size?"),
    0x30: ("entry_count", "I", "Entry count"),
    0x34: ("field_34", "I", "Unknown"),
    0x38: ("field_38", "I", "Unknown (encrypted section size?)"),
    0x3C: ("field_3C", "I", "Unknown (load addr flags?)"),
}


def dump_header(data: bytes):
    """Dump and interpret the PRX header fields (offsets 0x00-0x7F)."""
    print("  PRX HEADER (readable region)")
    print("  " + "-" * 58)
    for off, (name, fmt, desc) in sorted(KNOWN_HEADER_FIELDS.items()):
        if off + struct.calcsize(fmt) > len(data):
            break
        if fmt == "4s":
            val = data[off:off+4]
            val_str = val.hex()
        elif fmt == "28s":
            val = data[off:off+28]
            val_str = repr(val.rstrip(b'\x00'))
        elif fmt == "H":
            val = u16(data, off)
            val_str = f"0x{val:04X} ({val})"
        elif fmt == "B":
            val = data[off]
            val_str = f"0x{val:02X} ({val})"
        elif fmt == "I":
            val = u32(data, off)
            val_str = f"0x{val:08X} ({val:,})"
        else:
            val_str = "?"
        print(f"  0x{off:02X} ({name:16s}) = {val_str:28s}  {desc}")


def entropy_scan(data: bytes, start: int, size: int, blocksize: int = 256):
    """Scan for entropy boundaries to find encrypted vs. non-encrypted regions."""
    results = []
    for off in range(start, min(start + size, len(data)), blocksize):
        chunk = data[off:off+blocksize]
        if len(chunk) < 4:
            break
        # Simple entropy-like heuristic: count zero bytes
        zeros = chunk.count(0)
        # Encrypted data typically has very few zeros (< 1%)
        likely_encrypted = zeros < len(chunk) * 0.01
        results.append((off, zeros, likely_encrypted))
    return results


def find_data_boundary(data: bytes):
    """Find the boundary between unencrypted header and encrypted data."""
    # The header should be readable ASCII data in the PRX prefix
    # Encrypted data will look random (high entropy, few zeros)
    boundary = 0x80  # Known PSP PRX standard
    for off in range(0x40, min(0x200, len(data))):
        if data[off] == 0 and data[off+1] == 0:
            continue
        # After the header, data switches to encrypted (random-looking)
        # We detect this by looking for runs of non-zero, non-ASCII bytes
        break
    return boundary


def analyze_eboot(filepath: str):
    data = Path(filepath).read_bytes()
    size = len(data)

    print("=" * 66)
    print("  EBOOT.BIN PRX Analyzer — Gran Turismo PSP (UCES01245)")
    print("=" * 66)
    print(f"  File:   {filepath}")
    print(f"  Size:   {size:,} bytes ({size/1024/1024:.1f} MB)")
    print(f"  Magic:  {data[0:4]}")
    print(f"  Module: {ascii_str(data, 0x0A, 28)}")
    print()

    # ─── Header Dump ────────────────────────────────────────────
    dump_header(data)
    print()

    # ─── Encryption Boundary ────────────────────────────────────
    boundary = find_data_boundary(data)
    enc_start = 0x80
    print(f"  ENCRYPTION BOUNDARY")
    print(f"  " + "-" * 58)
    print(f"  Header ends:        0x{boundary:04X}")
    print(f"  Encrypted data at:  0x{enc_start:04X} ({size - enc_start:,} bytes)")
    
    # Check if data at 0x80 is encrypted
    sample = data[0x80:0x100]
    zeros = sample.count(0)
    print(f"  Zero bytes in [0x80..0xFF]: {zeros}/{len(sample)} ({zeros/len(sample)*100:.1f}%)")
    print(f"  Status:  {'PLAINTEXT (decrypted)' if zeros > 10 else 'ENCRYPTED/CIPHERTEXT'}")
    print()

    # ─── Section Detection Heuristics ──────────────────────────
    print(f"  ENTROPY SCAN (finding section boundaries)")
    print(f"  " + "-" * 58)
    scan = entropy_scan(data, 0x80, min(0x10000, size - 0x80))
    # Group into segments
    segments = []
    current_start = None
    for off, zeros, enc in scan:
        if current_start is None:
            current_start = off
        if (off + 256 >= len(scan) or 
            (enc != (scan[min(len(scan)-1, scan.index((off+256,0,False))+1)][2] 
                     if scan.index((off,0,False)) + 1 < len(scan) else enc))):
            # Actually let's just batch the scan
            pass
    
    # Simple segment detection
    prev_enc = None
    seg_start = 0x80
    for off, zeros, enc in scan:
        if prev_enc is not None and enc != prev_enc:
            print(f"    0x{seg_start:08X} - 0x{off:08X}  {'ENC' if prev_enc else 'CLR'}  ({off - seg_start:,} bytes)")
            seg_start = off
        prev_enc = enc
    print(f"    0x{seg_start:08X} - 0x{min(seg_start + 0x10000, size):08X}  {'ENC' if prev_enc else 'CLR'}")
    print()

    # ─── PSP Syscall NID Reference ─────────────────────────────
    print(f"  PSP SYSCALL EXPECTATIONS (likely imports)")
    print(f"  " + "-" * 58)
    print(f"  GT PSP (PDIAPP module) is expected to import from:")
    print(f"    - ThreadManForUser  (threads, mutexes)")
    print(f"    - IoFileMgrForUser  (file I/O → VFS hook point)")
    print(f"    - LoadCoreForKernel (module loading)")
    print(f"    - SysMemUserForUser (memory allocation)")
    print(f"    - CtrlForUser       (controller input)")
    print(f"    - DisplayForUser    (framebuffer)")
    print(f"    - Ge_user           (3D graphics)")
    print(f"    - AudioForUser      (audio)")
    print(f"    - ATRAC3plus        (music codec)")
    print(f"    - MpegForUser       (video playback)")
    print(f"    - UtilityForUser    (save data dialogs)")
    print(f"    - sceFont           (font rendering)")
    print(f"    - UmdForUser        (UMD disc access)")
    print(f"    - PowerForUser      (clock speed control)")
    print()

    # ─── VFS Analysis Strategy ─────────────────────────────────
    print(f"  VFS FUNCTION LOCATION STRATEGY")
    print(f"  " + "-" * 58)
    print(f"")
    print(f"  The Adhoc script loader (VFS) is buried inside the encrypted")
    print(f"  EBOOT sections. Two approaches can locate it:")
    print(f"")
    print(f"  A) GHIDRA DECOMPILATION")
    print(f"     Import EBOOT in Ghidra as PSP PRX (MIPS R4000 LE)")
    print(f"     Ghidra will auto-decrypt sections using PSP KIRK keys.")
    print(f"     After analysis:")
    print(f"       - Search strings: 'bootstrap', '.adc', 'GT.VOL'")
    print(f"       - Cross-reference to find the load() function")
    print(f"     See: analysis_guide.md for detailed steps")
    print(f"")
    print(f"  B) PPSSPP RUNTIME DEBUGGER")
    print(f"     Run GT PSP in PPSSPP with debugger enabled.")
    print(f"     Set breakpoints on PSP syscalls:")
    print(f"       - sceIoOpen       → catches all file opens")
    print(f"       - sceKernelLoadModule → catches PRX loading")
    print(f"     Trace back to find the calling function address.")
    print(f"     This gives you the runtime address for cheat patches.")
    print(f"")
    print(f"  C) HYBRID APPROACH (Recommended)")
    print(f"     1. Use PPSSPP debugger to find sceIoOpen call sites")
    print(f"        that load bootstrap.adc or GT.VOL")
    print(f"     2. Note the calling function address")
    print(f"     3. Match that address in Ghidra's decompilation")
    print(f"     4. Reverse engineer the VFS load function")
    print(f"     5. Create PPSSPP cheat patches")
    print(f"")
    
    # ─── Known Addresses ────────────────────────────────────────
    print(f"  KNOWN PSP MEMORY MAP")
    print(f"  " + "-" * 58)
    print(f"  User RAM:   0x08800000 - 0x0A000000  (32 MB)")
    print(f"  EBOOT load: typically at 0x08800000")
    print(f"  Stack:      0x0A000000 - (grows down)")
    print(f"  VRAM:       0x04000000 - 0x04200000  (2 MB)")
    print(f"  Scratchpad: 0x00010000 - 0x00014000  (16 KB)")
    print()
    
    print(f"  CHEAT PATCH ADDRESS FORMAT")
    print(f"  _S UCES-01245")
    print(f"  _G Gran Turismo (PSP)")
    print(f"  _C0 Patch Name")
    print(f"  _L 0x08XXXXXX 0xYYYYYYYY")
    print(f"    ↑ modify memory at this address   ↑ with this value")
    print()

    # ─── Tool Invocation ────────────────────────────────────────
    print(f"  QUICK COMMANDS")
    print(f"  " + "-" * 58)
    print(f"  # Ghidra headless analysis (after filling in project name):")
    print(f"  workflow\ghidra_12.0_PUBLIC\support\analyzeHeadless.bat")
    print(f"      <project> -import <EBOOT.BIN> -processor MIPS:little:32:default")
    print(f"")
    print(f"  # PPSSPP with debugger:")
    print(f"  C:\Program Files\PPSSPP\PPSSPPWindows64.exe")
    print(f"  # Enable: Settings → System → Debugger")
    print(f"  # Ctrl+B: Add memory breakpoint")
    print(f"  # Ctrl+F9: Step over / Step into")
    print(f"")

    print("=" * 66)


def main():
    default_path = os.path.join(
        os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__)))),
        "files", "original", "Gran Turismo", "PSP_GAME", "SYSDIR", "EBOOT.BIN"
    )
    eboot_path = sys.argv[1] if len(sys.argv) > 1 else default_path
    
    if not os.path.exists(eboot_path):
        print(f"ERROR: EBOOT not found at {eboot_path}")
        sys.exit(1)
    
    analyze_eboot(eboot_path)


if __name__ == "__main__":
    main()

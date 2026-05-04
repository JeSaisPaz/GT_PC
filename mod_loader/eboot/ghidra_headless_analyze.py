#!/usr/bin/env python3
"""
Ghidra Headless EBOOT Analysis Script
=====================================
Automates Ghidra decompilation of the GT PSP EBOOT.BIN.
Run via Ghidra's analyzeHeadless, then parses results to extract:

  1. String references (bootstrap, .adc, GT.VOL, etc.)
  2. Import/export tables (PSP syscall NIDs → resolved names)
  3. Function locations (VFS loader, Adhoc interpreter, etc.)
  4. Known constants (garage size=200, heap sizes, etc.)

Usage:
  # Step 1: Import and analyze the EBOOT
  workflow\ghidra_12.0_PUBLIC\support\analyzeHeadless.bat ^
      ghidra_projects gt_psp_eboot -import ^
      "files\original\Gran Turismo\PSP_GAME\SYSDIR\EBOOT.BIN" ^
      -processor "MIPS:little:32:default" ^
      -postScript mod_loader\eboot\ghidra_headless_analyze.py

  # Step 2: Parse the Ghidra output
  python mod_loader\eboot\ghidra_headless_analyze.py --parse results.json

  # Or run manually in Ghidra GUI:
  #   File → Import → EBOOT.BIN (PSP PRX)
  #   Analysis → Auto-Analyze
  #   Window → Script Manager → Run this script
"""

import json
import os
import sys
from pathlib import Path

# ─── Ghidra Python API (runs inside Ghidra's Jython) ────────────────────
# The following code runs inside Ghidra's headless analyzer.
# It uses Ghidra's Java API via Jython.

GHIDRA_SCRIPT = r"""
#@category: GT PSP
#@author: Mod Loader
#@description: Extract EBOOT analysis data for GT PSP mod loader

import json
from java.io import File
from java.util import ArrayList, HashSet

# Ghidra imports
from ghidra.app.util.exporter import StringExporter
from ghidra.app.script import GhidraScript
from ghidra.program.model.symbol import SymbolType, SourceType
from ghidra.program.model.address import AddressSet

class EBOOTAnalyzer:
    
    def __init__(self):
        self.results = {
            "module_name": None,
            "strings": [],
            "imports": [],
            "exports": [],
            "functions": [],
            "constants": [],
            "vfs_candidates": [],
        }
    
    def run(self):
        self._get_module_info()
        self._find_strings()
        self._find_functions()
        self._find_imports_exports()
        self._find_vfs_functions()
        self._find_constants()
        self._save_results()
    
    def _get_module_info(self):
        """Get the PRX module name from the program."""
        name = currentProgram.getName()
        self.results["module_name"] = name
        print(f"[EBOOT] Module: {name}")
        
        # Try to get module info from sceModuleInfo
        addr = currentProgram.getAddressFactory().getDefaultAddressSpace().getAddress(0)
        # Module info is typically in .rodata.sceModuleInfo section
        mod_info_sect = currentProgram.getListing().getSection(".rodata.sceModuleInfo")
        if mod_info_sect:
            print(f"[EBOOT] Found .rodata.sceModuleInfo at {mod_info_sect.getStart()}")
    
    def _find_strings(self):
        """Find all strings in the decompiled EBOOT, focusing on game-related ones."""
        print("[EBOOT] Scanning for strings...")
        
        target_strings = [
            "bootstrap", "packed_main_loop", "bootstrap_phase2",
            "shutdown", "Application", "init_sound",
            ".adc", "GT.VOL", "gt5m",
            "projects/", "scripts/", "products/",
            "MainLoop", "SpecDB", "pdiext",
            "ms0:", "flash0:", "disc0:", "umd0:",
            "load", "open", "read", "write",
            "malloc", "free", "calloc", "realloc",
            "PROJECT_ROOT_DIR", "MGOM", "GamePlan",
            "license", "save", "garage", "car",
        ]
        
        # Use Ghidra's string table
        listing = currentProgram.getListing()
        data_iter = listing.getDefinedData(True)
        
        found_strings = set()
        while data_iter.hasNext() and len(found_strings) < 500:
            data = data_iter.next()
            if data.isString():
                s = data.getValue()
                if s and len(str(s)) > 3:
                    str_val = str(s).lower()
                    for target in target_strings:
                        if target.lower() in str_val:
                            addr = data.getAddress()
                            found_strings.add(str(s))
                            self.results["strings"].append({
                                "address": str(addr),
                                "value": str(s),
                                "length": len(str(s)),
                            })
                            if len(found_strings) >= 50:
                                break
        
        print(f"[EBOOT] Found {len(self.results['strings'])} relevant strings")
    
    def _find_functions(self):
        """Enumerate all decompiled functions."""
        print("[EBOOT] Enumerating functions...")
        
        func_mgr = currentProgram.getFunctionManager()
        functions = func_mgr.getFunctions(True)
        
        count = 0
        for func in functions:
            if count >= 2000:
                break
            name = func.getName()
            addr = func.getEntryPoint()
            body = func.getBody()
            
            # Skip auto-generated stubs
            if name.startswith("_") and not name.startswith("__"):
                continue
                
            self.results["functions"].append({
                "name": name,
                "address": str(addr),
                "size": body.getNumAddresses(),
            })
            count += 1
        
        print(f"[EBOOT] Found {len(self.results['functions'])} functions")
    
    def _find_imports_exports(self):
        """Resolve PSP import/export NIDs to named functions."""
        print("[EBOOT] Resolving imports/exports...")
        
        # External references are PSP syscall imports
        for sym in currentProgram.getSymbolTable().getAllSymbols(True):
            source = sym.getSource()
            if source == SourceType.IMPORTED:
                name = sym.getName()
                addr = sym.getAddress()
                self.results["imports"].append({
                    "name": name,
                    "address": str(addr),
                })
    
    def _find_vfs_functions(self):
        """Identify VFS-related functions by cross-referencing known strings."""
        print("[EBOOT] Identifying VFS candidates...")
        
        vfs_keywords = ["load", "open", "read", "file", "vfs", "archive"]
        
        for func_entry in self.results["functions"]:
            name = func_entry["name"].lower()
            for kw in vfs_keywords:
                if kw in name:
                    func_entry["is_vfs_candidate"] = True
                    self.results["vfs_candidates"].append(func_entry)
                    break
    
    def _find_constants(self):
        """Search for known game constants in the binary."""
        print("[EBOOT] Searching for game constants...")
        
        # Constants to look for
        known_constants = [
            (200, "garage_size", "Default garage car limit"),
            (831, "car_count", "Total cars in game"),
            (0xC8, "garage_hex", "Garage limit 200 = 0xC8"),
            (0x4B0, "garage_2000", "Expanded garage 2000 = 0x4B0"),
            (0x800000, "heap_8mb", "8MB heap = 0x800000"),
            (0x2000000, "heap_32mb", "32MB heap = 0x2000000"),
        ]
        
        # Scan the program for these constants as 32-bit values
        # (Approximate — would need proper data flow analysis)
        memory = currentProgram.getMemory()
        for const_val, name, desc in known_constants:
            try:
                # Try to find in the program by scanning
                # This is a simplified approach
                addr_set = AddressSet()
                addr_set.add(memory.getInitializedAddressSet())
                
                locations = []
                # We use Ghidra's find bytes functionality
                # For now, just note what we're looking for
                self.results["constants"].append({
                    "name": name,
                    "value": const_val,
                    "description": desc,
                    "found": False,
                })
            except:
                pass
    
    def _save_results(self):
        """Write analysis results to JSON."""
        output_path = os.path.join(
            str(askFile("Save analysis results", "Save")),
        )
        with open(output_path, "w") as f:
            json.dump(self.results, f, indent=2)
        print(f"[EBOOT] Results saved to {output_path}")


analyzer = EBOOTAnalyzer()
analyzer.run()
"""

# ─── CLI Parser for post-analysis ─────────────────────────────────────────
# This part runs OUTSIDE Ghidra, on the JSON output.

ANALYSIS_RESULTS_TEMPLATE = {
    "meta": {
        "game": "Gran Turismo PSP (UCES01245)",
        "build": "JP2817",
        "eboot_size": 7058320,
        "generated_by": "ghidra_headless_analyze.py",
    },
    "module_name": "PDIAPP",
    "vfs_load_function": {
        "address": None,
        "name": "find_me_in_ghidra",
        "confidence": "pending",
        "notes": [
            "Search for string 'bootstrap' or 'packed_main_loop'",
            "Cross-reference to find caller of the load function",
            "The function likely calls sceIoOpen internally",
        ]
    },
    "adhoc_interpreter": {
        "address": None,
        "name": "find_me_in_ghidra",
        "notes": [
            "Look for a large switch/case dispatch table",
            "The bytecode interpreter processes opcodes like MODULE_DEFINE, LOAD_MODULE",
            "Search for known opcode constants in the binary",
        ]
    },
    "heap_alloc": {
        "address": None,
        "notes": [
            "Look for calls to sceKernelAllocatePartitionMemory",
            "Or search for heap size constants (0x800000 = 8MB)",
            "Patch to 0x2000000 = 32MB",
        ]
    },
    "garage_limit": {
        "address": None,
        "value": 200,
        "notes": [
            "Search for constant 200 (0xC8) in save data logic",
            "Look in SaveDataUtilPSP.ad references",
            "Patch to 2000 (0x7D0)",
        ]
    },
    "known_addresses": {
        "eboot_load_base": "0x08800000",
        "user_ram_start": "0x08800000",
        "user_ram_end": "0x0A000000",
    },
    "cheat_patches": [
        {
            "id": "max_cpu_clock",
            "name": "Max CPU Clock (333MHz)",
            "addresses": [],
            "notes": "Patch scePowerSetCpuClockFrequency or the MSICL register",
        },
        {
            "id": "vfs_redirect",
            "name": "Redirect Script Load to ms0:/PSP/MODS/",
            "address": None,
            "notes": ["Requires finding the VFS load function first"],
        },
        {
            "id": "extended_heap",
            "name": "Extended Memory Heap (32MB)",
            "address": None,
            "notes": ["Find heap allocation call, increase size parameter"],
        },
        {
            "id": "infinite_garage",
            "name": "Unlimited Garage (2000 cars)",
            "address": None,
            "notes": ["Find the 200 constant and change to 2000"],
        },
    ]
}


def parse(output_file: str):
    """Parse a Ghidra analysis output JSON and update our analysis files."""
    print(f"Parsing Ghidra output: {output_file}")
    
    # Read the Ghidra analysis results
    with open(output_file) as f:
        ghidra_results = json.load(f)
    
    # Merge with our template
    results = ANALYSIS_RESULTS_TEMPLATE.copy()
    results["ghidra_results"] = ghidra_results
    
    # Analyze strings found
    strings = ghidra_results.get("strings", [])
    print(f"\nFound {len(strings)} strings")
    
    vfs_candidates = []
    for s in strings:
        val = s["value"]
        addr = s["address"]
        if val in ("bootstrap", "packed_main_loop", "bootstrap_phase2", "shutdown"):
            print(f"  VFS KEY STRING: '{val}' at {addr}")
            vfs_candidates.append(s)
        elif val == "GT.VOL":
            print(f"  ARCHIVE STRING: 'GT.VOL' at {addr}")
        elif ".adc" in val:
            print(f"  SCRIPT EXT: '{val}' at {addr}")
    
    # Save merged results
    output_path = Path(output_file).parent / "ghidra_analysis_results.json"
    with open(output_path, "w") as f:
        json.dump(results, f, indent=2)
    
    print(f"\nMerged results saved to: {output_path}")
    print("\nNext steps:")
    print("  1. Copy addresses from 'bootstrap' string references")
    print("  2. Fill into mod_loader/eboot/vfs_addresses.json")
    print("  3. Create PPSSPP cheat patches in mod_loader/eboot/cheat_patches.ini")


def main():
    if len(sys.argv) > 2 and sys.argv[1] == "--parse":
        parse(sys.argv[2])
    else:
        print("=" * 66)
        print("  Ghidra Headless EBOOT Analyzer — GT PSP Mod Loader")
        print("=" * 66)
        print()
        print("  To run inside Ghidra:")
        print("    1. Import EBOOT.BIN into Ghidra")
        print("    2. Open the Script Manager")
        print("    3. Run this script (ghidra_headless_analyze.py)")
        print()
        print("  To run from command line (headless):")
        print("  analyzeHeadless.bat <project_dir> <project_name> \\")
        print("      -import <EBOOT.BIN> \\")
        print("      -processor MIPS:little:32:default \\")
        print("      -postScript mod_loader\\eboot\\ghidra_headless_analyze.py")
        print()
        print("  To parse existing Ghidra output:")
        print(f"  python {sys.argv[0]} --parse <ghidra_output.json>")
        print()
        print("=" * 66)


if __name__ == "__main__":
    main()

#!/usr/bin/env python3
"""
GT PSP ISO Builder
==================
Full pipeline: original ISO → extract → patch GT.VOL + EBOOT → new modded ISO.

Flow:
  1. Scan ./mods/ for mod manifests
  2. Interactive or CLI-selected mod list
  3. Extract GT.VOL from original ISO
  4. Unpack GT.VOL via GTPSPVolTools
  5. Inject modded core scripts + selected mods
  6. Repack GT.VOL
  7. Optionally patch EBOOT.BIN
  8. Build new ISO with modified GT.VOL
"""

import os
import json
import shutil
import subprocess
import sys
import tempfile
import time
from pathlib import Path
from typing import List, Optional, Dict


class ISOBuilder:
    """Build a modded GT PSP ISO from an original."""

    def __init__(
        self,
        original_iso: Path,
        output_iso: Path,
        mods_dir: Path,
        repo_root: Path,
        adhoc_toolchain: Path,
        gtvol_tool: Path,
        modded_core_adc: Path,
        modded_packed_main_loop: Path,
        eboot_patches: Optional[Dict] = None,
    ):
        self.original_iso = original_iso
        self.output_iso = output_iso
        self.mods_dir = mods_dir
        self.repo_root = repo_root
        self.adhoc_toolchain = adhoc_toolchain
        self.gtvol_tool = gtvol_tool
        self.modded_core_adc = modded_core_adc
        self.modded_packed_main_loop = modded_packed_main_loop
        self.eboot_patches = eboot_patches or {}

        # Temp workspace
        self._tmpdir: Optional[tempfile.TemporaryDirectory] = None
        self._tmp_root: Optional[Path] = None

    # ─── Public API ──────────────────────────────────────────────────────

    def build(self, selected_mod_ids: List[str]) -> bool:
        """Run the full ISO build pipeline."""
        start = time.time()
        self._setup_temp()

        try:
            print("  Step 1/7: Extracting original ISO files...")
            iso_files = self._extract_iso()

            print("  Step 2/7: Extracting GT.VOL from ISO...")
            gtvol_path = iso_files.get("/PSP_GAME/USRDIR/GT.VOL")
            if not gtvol_path or not Path(gtvol_path).exists():
                print("  ERROR: GT.VOL not found in original ISO.")
                return False

            gtvol_extracted = self._tmp_root / "gtvol_extracted"
            self._unpack_gtvol(Path(gtvol_path), gtvol_extracted)

            print("  Step 3/7: Loading selected mods...")
            mods = self._collect_mods(selected_mod_ids)
            if not mods:
                print("  WARNING: No mods selected. Building with modded core only.")

            print("  Step 4/7: Injecting modded core scripts...")
            self._inject_core_scripts(gtvol_extracted)

            print("  Step 5/7: Applying mods...")
            self._apply_mods(gtvol_extracted, mods)

            print("  Step 6/7: Repacking GT.VOL...")
            modified_gtvol = self._tmp_root / "GT.VOL.modified"
            self._repack_gtvol(gtvol_extracted, modified_gtvol)

            print("  Step 7/7: Building new ISO...")
            self._build_iso(iso_files, modified_gtvol)

            elapsed = time.time() - start
            out_size = self.output_iso.stat().st_size
            print(f"  Done! {out_size / 1024 / 1024:.0f} MB ISO written in {elapsed:.0f}s")
            print(f"  Output: {self.output_iso}")
            return True

        except Exception as e:
            print(f"  ERROR: {e}")
            import traceback
            traceback.print_exc()
            return False
        finally:
            self._cleanup()

    def list_available_mods(self) -> List[Dict]:
        """Scan mods_dir for all available mod manifests."""
        return self._scan_mods()

    # ─── Temp Workspace ──────────────────────────────────────────────────

    def _setup_temp(self):
        self._tmpdir = tempfile.TemporaryDirectory(prefix="gtpsp_iso_")
        self._tmp_root = Path(self._tmpdir.name)

    def _cleanup(self):
        if self._tmpdir:
            try:
                self._tmpdir.cleanup()
            except Exception:
                pass
            self._tmpdir = None

    # ─── ISO Extraction ──────────────────────────────────────────────────

    def _extract_iso(self) -> Dict[str, str]:
        """Extract all files from the original ISO to temp paths.
        Returns {iso_path: temp_file_path} mapping."""
        import pycdlib

        iso = pycdlib.PyCdlib()
        iso.open(str(self.original_iso))

        extract_dir = self._tmp_root / "original_files"
        extract_dir.mkdir()

        file_map = {}

        for dirname, dirlist, filelist in iso.walk(iso_path="/"):
            for fname in filelist:
                iso_path = (dirname.rstrip("/") + "/" + fname).replace("//", "/")

                # Build a safe temp path mirroring the ISO structure
                rel = iso_path.lstrip("/")
                target = extract_dir / rel
                target.parent.mkdir(parents=True, exist_ok=True)

                # Extract file
                with open(target, "wb") as f:
                    try:
                        iso.get_file_from_iso_fp(f, iso_path=iso_path)
                    except Exception as e:
                        print(f"    Warning: could not extract {iso_path}: {e}")
                        continue

                file_map[iso_path] = str(target)

        iso.close()
        return file_map

    # ─── GT.VOL Operations ───────────────────────────────────────────────

    def _unpack_gtvol(self, gtvol_path: Path, output_dir: Path):
        """Unpack GT.VOL archive using GTPSPVolTools."""
        output_dir.mkdir(parents=True)
        result = subprocess.run(
            [str(self.gtvol_tool), "unpack", "-i", str(gtvol_path), "-o", str(output_dir)],
            capture_output=True, text=True, timeout=300,
        )
        if result.returncode != 0:
            raise RuntimeError(f"GTPSPVolTools unpack failed: {result.stderr[:500]}")

    def _repack_gtvol(self, input_dir: Path, output_path: Path):
        """Repack a directory into GT.VOL archive."""
        result = subprocess.run(
            [str(self.gtvol_tool), "pack", "-i", str(input_dir), "-o", str(output_path)],
            capture_output=True, text=True, timeout=300,
        )
        if result.returncode != 0:
            raise RuntimeError(f"GTPSPVolTools pack failed: {result.stderr[:500]}")

    def _inject_core_scripts(self, gtvol_extracted: Path):
        """Replace core scripts with modded versions."""
        scripts_dir = gtvol_extracted / "scripts" / "gt5m"
        scripts_dir.mkdir(parents=True, exist_ok=True)

        # Application.adc
        if self.modded_core_adc and self.modded_core_adc.exists():
            shutil.copy2(self.modded_core_adc, scripts_dir / "Application.adc")
            print(f"    Application.adc ({self.modded_core_adc.stat().st_size} bytes)")
        else:
            print("    WARNING: Application_patched.adc not found. Run build_modded_core.ps1 first.")

        # packed_main_loop.adc (contains ModLoader)
        if self.modded_packed_main_loop and self.modded_packed_main_loop.exists():
            shutil.copy2(self.modded_packed_main_loop, scripts_dir / "packed_main_loop.adc")
            print(f"    packed_main_loop.adc ({self.modded_packed_main_loop.stat().st_size} bytes)")
        else:
            print("    WARNING: packed_main_loop.adc not found. Run build_modded_core.ps1 first.")

    # ─── Mod Management ──────────────────────────────────────────────────

    def _scan_mods(self) -> List[Dict]:
        """Scan for mod manifests in the mods directory."""
        mods = []
        if not self.mods_dir.exists():
            return mods

        for manifest_path in sorted(self.mods_dir.rglob("manifest.yaml")):
            try:
                import yaml
                with open(manifest_path) as f:
                    data = yaml.safe_load(f)
                mods.append({
                    "id": data.get("id", manifest_path.parent.name),
                    "name": data.get("name", manifest_path.parent.name),
                    "version": data.get("version", "?"),
                    "author": data.get("author", "?"),
                    "description": data.get("description", ""),
                    "manifest_path": manifest_path,
                    "mod_dir": manifest_path.parent,
                    "data": data,
                })
            except Exception as e:
                print(f"    Warning: could not parse {manifest_path}: {e}")

        return mods

    def _collect_mods(self, selected_ids: List[str]) -> List[Dict]:
        """Resolve selected mod IDs to full mod data."""
        available = self._scan_mods()
        id_map = {m["id"]: m for m in available}
        result = []
        for mid in selected_ids:
            if mid in id_map:
                result.append(id_map[mid])
            else:
                print(f"    Warning: mod '{mid}' not found in {self.mods_dir}")
        return result

    def _apply_mods(self, gtvol_extracted: Path, mods: List[Dict]):
        """Inject mod files into the extracted GT.VOL directory."""
        if not mods:
            print("    No mods to apply.")
            return

        manifest_entries = []

        for mod in mods:
            mid = mod["id"]
            data = mod["data"]
            mod_dir = mod["mod_dir"]
            scripts = data.get("scripts", [])
            assets = data.get("assets", [])
            patches = data.get("eboot_patches", [])

            print(f"    [{mid}] {mod['name']} v{mod['version']}")

            # Compile and inject scripts
            for script_def in scripts:
                src = mod_dir / script_def["source"]
                if not src.exists():
                    print(f"      WARNING: {script_def['source']} not found")
                    continue

                target_rel = script_def.get("target", f"mods/{mid}/{src.stem}.adc")
                target_path = gtvol_extracted / target_rel
                target_path.parent.mkdir(parents=True, exist_ok=True)

                # Compile .ad -> .adc
                print(f"      Compiling: {src.name} -> {target_rel}")
                result = subprocess.run(
                    [str(self.adhoc_toolchain), "build", "-i", str(src),
                     "-o", str(target_path), "-v", "12"],
                    capture_output=True, text=True, timeout=60,
                )
                if result.returncode != 0:
                    print(f"      FAILED: {result.stderr.strip()[:200]}")
                    continue

                manifest_entries.append({
                    "id": mid,
                    "script": target_rel,
                })

            # Copy assets
            for asset_def in assets:
                src = mod_dir / asset_def.get("source", "")
                if not src.exists():
                    print(f"      WARNING: asset {asset_def.get('source')} not found")
                    continue

                target_rel = asset_def["target"]
                target_path = gtvol_extracted / target_rel
                target_path.parent.mkdir(parents=True, exist_ok=True)
                shutil.copy2(src, target_path)
                print(f"      Asset: {target_rel}")

            # Collect EBOOT patches if any
            for p in patches:
                addr = p.get("address", "")
                if addr and addr not in ("TBD", "???"):
                    self.eboot_patches.setdefault("mod_patches", []).append(p)

        # Write mods_manifest.json so ModLoader can discover mods
        if manifest_entries:
            manifest_path = gtvol_extracted / "mods_manifest.json"
            with open(manifest_path, "w") as f:
                json.dump({
                    "mod_loader_version": "1.0.0",
                    "mods": manifest_entries,
                }, f, indent=2)
            print(f"    Manifest: mods_manifest.json ({len(manifest_entries)} entries)")

    # ─── ISO Building ───────────────────────────────────────────────────

    def _build_iso(self, original_files: Dict[str, str], modified_gtvol: Path):
        """Create a new ISO with original files + modified GT.VOL."""
        import pycdlib

        # Keep temp files alive until write completes
        _keep_alive = []

        iso = pycdlib.PyCdlib()
        iso.new(interchange_level=4)

        iso.add_directory("/PSP_GAME")
        iso.add_directory("/PSP_GAME/SYSDIR")
        iso.add_directory("/PSP_GAME/SYSDIR/UPDATE")
        iso.add_directory("/PSP_GAME/USRDIR")
        iso.add_directory("/PSP_GAME/USRDIR/MODULE")

        # Add all files from the original, replacing GT.VOL with modified
        for iso_path, src_path in original_files.items():
            src = Path(src_path)
            if not src.exists():
                continue

            # Skip the original GT.VOL — we'll add the modified one
            if iso_path == "/PSP_GAME/USRDIR/GT.VOL":
                continue

            # For EBOOT, optionally apply patches
            if iso_path == "/PSP_GAME/SYSDIR/EBOOT.BIN":
                patched = self._apply_eboot_patches(src)
                if patched:
                    iso.add_file(str(patched), iso_path)
                    _keep_alive.append(patched)
                    continue

            iso.add_file(str(src), iso_path)

        # Add modified GT.VOL
        if modified_gtvol.exists():
            iso.add_file(str(modified_gtvol), "/PSP_GAME/USRDIR/GT.VOL")

        # Write the ISO
        iso.write(str(self.output_iso))
        iso.close()

    # ─── EBOOT Patching ──────────────────────────────────────────────────

    def _apply_eboot_patches(self, eboot_path: Path) -> Optional[Path]:
        """Apply binary patches to EBOOT.BIN if addresses are filled in."""
        patches = self.eboot_patches.get("mod_patches", [])
        if not patches:
            return None

        # Read the EBOOT
        with open(eboot_path, "rb") as f:
            data = bytearray(f.read())

        patched = False
        for patch in patches:
            addr_str = patch.get("address", "")
            value_str = patch.get("value", "")
            if not addr_str or not value_str or addr_str in ("TBD", "???"):
                continue

            try:
                # PSP memory address → file offset
                # EBOOT loads at 0x08800000, subtract to get file offset
                addr = int(addr_str, 16) - 0x08800000
                values = [int(v, 16) for v in value_str.split()]

                for i, v in enumerate(values):
                    if addr + i < len(data):
                        data[addr + i] = v
                        patched = True
            except (ValueError, IndexError) as e:
                print(f"      Patch error ({patch.get('comment','?')}): {e}")

        if not patched:
            return None

        # Write patched EBOOT to temp
        patched_path = self._tmp_root / "EBOOT.BIN.patched"
        with open(patched_path, "wb") as f:
            f.write(data)

        cnt = sum(1 for p in patches if p.get("address", "") not in ("TBD", "???"))
        print(f"    Patched EBOOT.BIN ({cnt} patch(es) applied)")
        return patched_path

    # ─── Interactive Picker ──────────────────────────────────────────────

    @staticmethod
    def interactive_picker(mods: List[Dict]) -> List[str]:
        """Console-based mod selection. Returns list of selected mod IDs."""
        if not mods:
            print("No mods found in ./mods/ directory.")
            return []

        selected = set()
        print("\nAvailable mods:")
        print("─" * 65)
        for i, m in enumerate(mods, 1):
            print(f"  [{i:2d}]  {m['name']:30s} v{m['version']:6s}  by {m['author']}")
            if m.get("description"):
                print(f"        {m['description']}")
        print("─" * 65)

        while True:
            print("\nEnter numbers (space/comma separated), 'all', or 'done': ", end="")
            try:
                choice = sys.stdin.readline().strip().lower()
            except (EOFError, KeyboardInterrupt):
                print()
                break

            if choice in ("", "done", "q", "quit"):
                break
            if choice == "all":
                selected = set(range(len(mods)))
                break

            for part in choice.replace(",", " ").split():
                part = part.strip()
                if not part:
                    continue
                try:
                    idx = int(part) - 1
                    if 0 <= idx < len(mods):
                        selected.add(idx)
                except ValueError:
                    pass

            # Show current selection
            if selected:
                print(f"  Selected: {', '.join(mods[i]['id'] for i in sorted(selected))}")
            else:
                print("  (none selected)")

        return [mods[i]["id"] for i in sorted(selected)]

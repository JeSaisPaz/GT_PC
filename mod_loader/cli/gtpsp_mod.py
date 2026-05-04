#!/usr/bin/env python3
"""
GT PSP Mod Loader CLI (gtpsp-mod)
==================================
A build tool + mod manager for Gran Turismo PSP modding on PPSSPP.

Usage:
    gtpsp-mod init <name>          Create a new mod project
    gtpsp-mod build <path>         Compile mod .ad source to .adc
    gtpsp-mod deploy <path>        Deploy mod to PPSSPP memstick
    gtpsp-mod patch-eboot          Generate PPSSPP cheat patches from analysis
    gtpsp-mod list                 List available mods
    gtpsp-mod info <name>          Show mod info

Requires:
    - Python 3.8+
    - GTAdhocToolchain (adhoc.exe) in workflow/adhoc-toolchain/
    - PyYAML (pip install pyyaml)
"""

import argparse
import io
import json
import os
import shutil
import subprocess
import sys
import yaml
from pathlib import Path
from typing import Optional, List

# Force UTF-8 on Windows console
if sys.platform == "win32":
    sys.stdout = io.TextIOWrapper(sys.stdout.buffer, encoding="utf-8", errors="replace")
    sys.stderr = io.TextIOWrapper(sys.stderr.buffer, encoding="utf-8", errors="replace")


# ─── Paths ────────────────────────────────────────────────────────────────
REPO_ROOT = Path(__file__).resolve().parent.parent.parent
ADHOC_TOOLCHAIN = REPO_ROOT / "workflow" / "adhoc-toolchain" / "adhoc.exe"
GTPSP_VOL_TOOLS = REPO_ROOT / "workflow" / "gtpspvoltools" / "GTPSPVolTools.exe"
SOURCE_DIR = REPO_ROOT / "source"
MOD_LOADER_DIR = REPO_ROOT / "mod_loader"
PPSSPP_MEMSTICK_DEFAULT = Path(os.environ.get("PPSSPP_MEMSTICK", ""))

# ISO builder (lazy import — needs pycdlib)
_iso_builder = None
def _get_iso_builder():
    global _iso_builder
    if _iso_builder is None:
        import importlib.util
        spec = importlib.util.spec_from_file_location(
            "iso_builder",
            str(MOD_LOADER_DIR / "cli" / "iso_builder.py"),
        )
        mod = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(mod)
        _iso_builder = mod.ISOBuilder
    return _iso_builder

PPSSPP_CANDIDATES = [
    Path(os.environ.get("PPSSPP_MEMSTICK", "")),
    Path.home() / "Documents" / "PPSSPP",
    Path.home() / ".config" / "ppsspp",
    Path("C:") / "Users" / os.environ.get("USERNAME", "") / "Documents" / "PPSSPP",
]


# ─── Helpers ──────────────────────────────────────────────────────────────

def find_ppsspp_memstick() -> Optional[Path]:
    for p in PPSSPP_CANDIDATES:
        if p.exists() and (p / "PSP").exists():
            return p
    return None


def ensure_adhoc() -> Path:
    if not ADHOC_TOOLCHAIN.exists():
        print(f"ERROR: GTAdhocToolchain not found at {ADHOC_TOOLCHAIN}")
        print("Download from: https://github.com/Nenkai/GTAdhocToolchain/releases")
        sys.exit(1)
    return ADHOC_TOOLCHAIN


def compile_ad(ad_path: Path, output_path: Path, version: int = 12) -> bool:
    adhoc = ensure_adhoc()
    cmd = [str(adhoc), "build", "-i", str(ad_path), "-o", str(output_path), "-v", str(version)]
    result = subprocess.run(cmd, capture_output=True, text=True)
    if result.returncode != 0:
        stderr = result.stderr.strip()
        # Extract meaningful error
        for line in stderr.split("\n"):
            if "ERROR" in line or "error" in line:
                print(f"  {line.strip()}")
                break
        else:
            print(f"  FAILED: {stderr[:200]}")
        return False
    return True


def load_yaml(path: Path) -> dict:
    with open(path, "r", encoding="utf-8") as f:
        return yaml.safe_load(f)


def save_yaml(path: Path, data: dict):
    with open(path, "w", encoding="utf-8") as f:
        yaml.dump(data, f, default_flow_style=False)


def find_mods(search_paths: List[Path]) -> List[dict]:
    seen = set()
    found = []
    for base in search_paths:
        if not base.exists():
            continue
        for yaml_path in sorted(base.rglob("manifest.yaml")):
            abs_path = str(yaml_path.resolve())
            if abs_path in seen:
                continue
            seen.add(abs_path)
            try:
                manifest = load_yaml(yaml_path)
                found.append({
                    "id": manifest.get("id", yaml_path.parent.name),
                    "name": manifest.get("name", yaml_path.parent.name),
                    "version": manifest.get("version", "?"),
                    "author": manifest.get("author", "?"),
                    "path": yaml_path.parent,
                })
            except Exception:
                pass
    return found


# ─── Commands ─────────────────────────────────────────────────────────────

def cmd_init(args):
    """Initialize a new mod project."""
    name = args.name
    mod_dir = Path.cwd() / name

    if mod_dir.exists():
        print(f"ERROR: Directory '{mod_dir}' already exists.")
        sys.exit(1)

    mod_dir.mkdir(parents=True)
    (mod_dir / "assets").mkdir()

    manifest = {
        "id": name,
        "version": "1.0.0",
        "name": name.replace("_", " ").title(),
        "description": "A GT PSP mod",
        "author": "Anonymous",
        "scripts": [
            {"source": "main.ad", "target": f"mods/{name}/main.adc"}
        ],
        "assets": [],
    }
    save_yaml(mod_dir / "manifest.yaml", manifest)

    # NOTE: #include path is relative to the mod source file's directory.
    # Since the mod is compiled from its own dir, we need a relative path
    # to the mod_sdk.inc. If the mod_loader is at the repo root alongside
    # the mod dir, this works. Otherwise user must adjust.
    template = f'''//---------------------------------------------------------------------------------------
// {name}.ad
// GT PSP Mod — auto-generated
//---------------------------------------------------------------------------------------

// Adjust this path to match your project layout.
// If your mod dir is at the same level as mod_loader/:
#include "../mod_loader/core/mod_sdk.inc"

function _mod_init()
{{
    // Register event listeners. Example:
    // MOD_ON_EVENT("beforeMenu", function(args) {{
    //     MOD_LOG("Menu opened!");
    // }});
}}
'''
    with open(mod_dir / "main.ad", "w") as f:
        f.write(template)

    print(f"Created mod project: {mod_dir}")
    print(f"Edit manifest.yaml and main.ad, then run:")
    print(f"  gtpsp-mod build {name}")
    print(f"  gtpsp-mod deploy {name}")


def cmd_build(args):
    """Compile mod source files to .adc."""
    mod_path = Path(args.path)
    if not mod_path.exists():
        print(f"ERROR: Mod path '{mod_path}' not found.")
        sys.exit(1)

    manifest_path = mod_path / "manifest.yaml"
    if not manifest_path.exists():
        print(f"ERROR: No manifest.yaml found in '{mod_path}'.")
        sys.exit(1)

    manifest = load_yaml(manifest_path)
    mod_id = manifest.get("id", mod_path.name)
    build_dir = mod_path / "build"
    build_dir.mkdir(exist_ok=True)

    print(f"Building mod: {mod_id} v{manifest.get('version', '?.?.?')}")

    success = True
    compiled = 0
    for script_def in manifest.get("scripts", []):
        src = mod_path / script_def["source"]
        if not src.exists():
            print(f"  WARNING: {script_def['source']} not found, skipping.")
            continue

        rel_target = script_def.get("target", f"mods/{mod_id}/{src.stem}.adc")
        out_path = build_dir / rel_target
        out_path.parent.mkdir(parents=True, exist_ok=True)

        print(f"  Compiling: {src.name} -> {out_path}")
        if compile_ad(src, out_path):
            compiled += 1
        else:
            success = False

    if success and compiled > 0:
        print(f"  Build complete: {compiled} file(s) in {build_dir}")
    elif compiled == 0:
        print(f"  WARNING: No files compiled.")
    else:
        print(f"  Build completed with errors ({compiled} ok).")
    return success


def cmd_deploy(args):
    """Deploy mod to PPSSPP memstick."""
    mod_path = Path(args.path)
    if not mod_path.exists():
        print(f"ERROR: Mod path '{mod_path}' not found.")
        sys.exit(1)

    manifest_path = mod_path / "manifest.yaml"
    if not manifest_path.exists():
        print(f"ERROR: No manifest.yaml in '{mod_path}'.")
        sys.exit(1)

    manifest = load_yaml(manifest_path)
    mod_id = manifest.get("id", mod_path.name)

    ppsspp_dir = find_ppsspp_memstick()
    if args.ppsspp_dir:
        ppsspp_dir = Path(args.ppsspp_dir)
    if not ppsspp_dir or not ppsspp_dir.exists():
        print(f"ERROR: PPSSPP memstick not found.")
        print(f"  Specify with: --ppsspp-dir <path>")
        print(f"  Or set: $env:PPSSPP_MEMSTICK = '<path>'")
        sys.exit(1)

    print(f"Deploying mod: {mod_id}")

    strategy = args.strategy or "gtvol"
    deploy_dir = ppsspp_dir / "PSP" / "UMD0" / "PSP_GAME" / "USRDIR" / "GT.VOL"

    if strategy == "gtvol":
        build_dir = mod_path / "build"
        if not build_dir.exists():
            print(f"  ERROR: Build directory not found. Run 'gtpsp-mod build' first.")
            sys.exit(1)

        deployed = 0
        for file in build_dir.rglob("*.adc"):
            rel_path = file.relative_to(build_dir)
            target = deploy_dir / rel_path
            target.parent.mkdir(parents=True, exist_ok=True)
            shutil.copy2(file, target)
            deployed += 1

        for asset_def in manifest.get("assets", []):
            src = mod_path / asset_def.get("source", "")
            if not src.exists():
                print(f"  WARNING: Asset {asset_def.get('source')} not found, skipping.")
                continue
            target_path = deploy_dir / asset_def["target"]
            target_path.parent.mkdir(parents=True, exist_ok=True)
            shutil.copy2(src, target_path)
            deployed += 1

        print(f"  Deployed {deployed} files to {deploy_dir}")

    elif strategy == "cheat":
        cheat_path = ppsspp_dir / "PSP" / "CHEATS" / f"{args.game_id or 'UCES01245'}.ini"
        cheat_path.parent.mkdir(parents=True, exist_ok=True)

        patches = manifest.get("eboot_patches", [])
        with open(cheat_path, "a") as f:
            f.write(f"\n// Mod: {mod_id}\n")
            f.write(f"// {manifest.get('description', '')}\n")
            f.write(f"_S UCES-01245\n")
            f.write(f"_G Gran Turismo PSP\n")
            for patch in patches:
                addr = patch.get("address", "")
                value = patch.get("value", "")
                comment = patch.get("comment", "")
                if addr and value:
                    f.write(f"_C0 {comment or mod_id}\n")
                    f.write(f"_L 0x{addr} 0x{value}\n")

        print(f"  Appended {len(patches)} cheat patches to {cheat_path}")

    print(f"  Deploy complete. Restart PPSSPP to load the mod.")


def cmd_patch_eboot(args):
    """Generate PPSSPP cheat patches from EBOOT analysis data."""
    analysis_file = MOD_LOADER_DIR / "eboot" / "vfs_addresses.json"
    if not analysis_file.exists():
        print(f"No EBOOT analysis found at {analysis_file}")
        print(f"Run Ghidra analysis first (see mod_loader/eboot/analysis_guide.md)")
        sys.exit(1)

    with open(analysis_file) as f:
        data = json.load(f)

    patches = []
    for entry in data.get("patches", []):
        if entry.get("address") and entry.get("value"):
            patches.append(f"_C0 {entry.get('comment', 'Unnamed patch')}")
            patches.append(f"_L 0x{entry['address']} 0x{entry['value']}")
            patches.append("")

    if not patches:
        print("No completed patches found in analysis file. Fill in addresses first.")
        sys.exit(1)

    out_path = MOD_LOADER_DIR / "eboot" / "generated_patches.ini"
    with open(out_path, "w") as f:
        f.write("// GT PSP Mod Loader — Auto-generated PPSSPP Cheat Patches\n")
        f.write(f"// Source: {analysis_file.name}\n")
        f.write("_S UCES-01245\n")
        f.write("_G Gran Turismo (PSP)\n\n")
        f.write("\n".join(patches))

    print(f"Generated {len(patches)} patch(es) at {out_path}")


def cmd_list(args):
    """List all available mods."""
    search_paths = [
        MOD_LOADER_DIR / "examples",
        Path.cwd(),
    ]
    found = find_mods(search_paths)

    if not found:
        print("No mods found.")
        return

    print(f"Found {len(found)} mod(s):")
    print(f"  {'ID':<20} {'Name':<25} {'Version':<10} {'Author':<20}")
    print(f"  {'-'*75}")
    for m in sorted(found, key=lambda x: x["id"]):
        print(f"  {m['id']:<20} {m['name']:<25} {m['version']:<10} {m['author']:<20}")
        print(f"  {'':>4}{m['path']}")


def cmd_info(args):
    """Show detailed info about a mod."""
    mod_path = Path(args.name)
    if not mod_path.exists():
        candidate = MOD_LOADER_DIR / "examples" / args.name
        if candidate.exists():
            mod_path = candidate
    if not mod_path.exists() or not (mod_path / "manifest.yaml").exists():
        print(f"Mod '{args.name}' not found. Use 'gtpsp-mod list' to see available mods.")
        sys.exit(1)

    manifest = load_yaml(mod_path / "manifest.yaml")
    print(f"  ID:          {manifest.get('id', '?')}")
    print(f"  Name:        {manifest.get('name', '?')}")
    print(f"  Version:     {manifest.get('version', '?')}")
    print(f"  Description: {manifest.get('description', '?')}")
    print(f"  Author:      {manifest.get('author', '?')}")
    print(f"  Scripts:     {len(manifest.get('scripts', []))}")
    print(f"  Assets:      {len(manifest.get('assets', []))}")
    print(f"  EBOOT patches: {len(manifest.get('eboot_patches', []))}")


# ─── ISO Commands ─────────────────────────────────────────────────────────

def cmd_iso(args):
    """Build a modded ISO from original + selected mods."""
    ISOBuilder = _get_iso_builder()

    original_iso = Path(args.original_iso)
    if not original_iso.exists():
        print(f"ERROR: Original ISO not found: {original_iso}")
        sys.exit(1)

    output_iso = Path(args.output) if args.output else REPO_ROOT / "GTPSP_modded.iso"
    mods_dir = Path(args.mods_dir) if args.mods_dir else Path.cwd() / "mods"
    modded_core_adc = MOD_LOADER_DIR / "core" / "Application_patched.adc"
    modded_pml_adc = MOD_LOADER_DIR / "core" / "packed_main_loop.adc"

    # Load EBOOT patch data if available
    eboot_patches = {}
    vfs_json = MOD_LOADER_DIR / "eboot" / "vfs_addresses.json"
    if vfs_json.exists():
        try:
            eboot_patches = json.loads(vfs_json.read_text())
        except Exception:
            pass

    builder = ISOBuilder(
        original_iso=original_iso,
        output_iso=output_iso,
        mods_dir=mods_dir,
        repo_root=REPO_ROOT,
        adhoc_toolchain=ADHOC_TOOLCHAIN,
        gtvol_tool=GTPSP_VOL_TOOLS,
        modded_core_adc=modded_core_adc,
        modded_packed_main_loop=modded_pml_adc,
        eboot_patches=eboot_patches,
    )

    # Interactive picker or direct mod list
    if args.interactive:
        available = builder.list_available_mods()
        selected = ISOBuilder.interactive_picker(available)
    elif args.mods:
        selected = [m.strip() for m in args.mods.split(",") if m.strip()]
    else:
        # No mods specified — build with modded core only
        selected = []
        print("No mods specified. Building with modded core scripts only.")

    print(f"\nBuilding ISO with {len(selected)} mod(s)...")
    success = builder.build(selected)

    if success:
        print(f"\nISO built: {output_iso}")
        print(f"Size: {output_iso.stat().st_size / 1024 / 1024:.0f} MB")
    else:
        print(f"\nISO build FAILED.")
        sys.exit(1)


def cmd_iso_list_mods(args):
    """List available mods in a directory."""
    ISOBuilder = _get_iso_builder()
    mods_dir = Path(args.mods_dir) if args.mods_dir else Path.cwd() / "mods"

    builder = ISOBuilder(
        original_iso=Path("dummy"), output_iso=Path("dummy"),
        mods_dir=mods_dir, repo_root=REPO_ROOT,
        adhoc_toolchain=ADHOC_TOOLCHAIN, gtvol_tool=GTPSP_VOL_TOOLS,
        modded_core_adc=Path("dummy"), modded_packed_main_loop=Path("dummy"),
    )
    mods = builder.list_available_mods()

    if not mods:
        print(f"No mods found in {mods_dir}")
        return

    print(f"Mods in {mods_dir}:")
    for m in mods:
        print(f"  {m['id']:<25s} v{m['version']:<6s} {m['name']}")


# ─── Main ─────────────────────────────────────────────────────────────────

def main():
    parser = argparse.ArgumentParser(
        description="GT PSP Mod Loader — Build, deploy, and manage mods for Gran Turismo PSP on PPSSPP",
        formatter_class=argparse.RawDescriptionHelpFormatter,
        epilog="""
Examples:
  gtpsp-mod init my-awesome-mod
  gtpsp-mod build my-awesome-mod/
  gtpsp-mod deploy my-awesome-mod/
  gtpsp-mod list
  gtpsp-mod info all_cars_garage
        """,
    )

    sub = parser.add_subparsers(dest="command", required=True)

    p_init = sub.add_parser("init", help="Create a new mod project")
    p_init.add_argument("name", help="Mod name (e.g., 'my-awesome-mod')")

    p_build = sub.add_parser("build", help="Compile mod .ad to .adc")
    p_build.add_argument("path", help="Path to mod directory containing manifest.yaml")

    p_deploy = sub.add_parser("deploy", help="Deploy mod to PPSSPP")
    p_deploy.add_argument("path", help="Path to mod directory")
    p_deploy.add_argument("--ppsspp-dir", help="PPSSPP memstick directory override")
    p_deploy.add_argument("--game-id", default="UCES01245", help="Game ID for cheat patches")
    p_deploy.add_argument("--strategy", choices=["gtvol", "cheat"], default="gtvol",
                          help="Deployment strategy")

    p_patch = sub.add_parser("patch-eboot", help="Generate PPSSPP cheat patches")

    sub.add_parser("list", help="List available mods")

    p_info = sub.add_parser("info", help="Show mod details")
    p_info.add_argument("name", help="Mod name or path")

    # ISO subcommand group
    p_iso = sub.add_parser("iso", help="Build or inspect a modded ISO")
    p_iso_sub = p_iso.add_subparsers(dest="iso_command", required=True)

    p_iso_create = p_iso_sub.add_parser("create", help="Build modded ISO from original")
    p_iso_create.add_argument("original_iso", help="Path to original GT PSP ISO")
    p_iso_create.add_argument("-o", "--output", help="Output ISO path (default: GTPSP_modded.iso)")
    p_iso_create.add_argument("--mods-dir", help="Directory with mod folders (default: ./mods/)")
    p_iso_create.add_argument("-m", "--mods", help="Comma-separated mod IDs to include")
    p_iso_create.add_argument("-i", "--interactive", action="store_true",
                              help="Pick mods interactively")

    p_iso_list = p_iso_sub.add_parser("list-mods", help="List mods available in a directory")
    p_iso_list.add_argument("--mods-dir", help="Directory with mod folders (default: ./mods/)")

    args = parser.parse_args()

    commands = {
        "init": cmd_init,
        "build": cmd_build,
        "deploy": cmd_deploy,
        "patch-eboot": cmd_patch_eboot,
        "list": cmd_list,
        "info": cmd_info,
    }

    if args.command == "iso":
        if args.iso_command == "create":
            cmd_iso(args)
        elif args.iso_command == "list-mods":
            cmd_iso_list_mods(args)
        return

    commands[args.command](args)


if __name__ == "__main__":
    main()

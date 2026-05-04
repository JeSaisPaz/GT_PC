# GT PSP Mod Loader

A modular modding framework for **Gran Turismo PSP** on **PPSSPP**, designed to go past the PSP's hardware limits and enable seamless, dynamic modding.

## Architecture

```
┌────────────────────────────────────────────────────────────┐
│                    Mod Loader System                        │
├────────────────────────────────────────────────────────────┤
│  Layer 3: gtpsp-mod CLI tool (Python)                      │
│  - init / build / deploy / patch-eboot / list / info       │
├────────────────────────────────────────────────────────────┤
│  Layer 2: Modified Core Scripts (.ad / .adc)               │
│  - patched Application.ad + packed_main_loop.ad            │
│  - ModLoader module with hook registry                     │
│  - 30+ hookable function points                            │
├────────────────────────────────────────────────────────────┤
│  Layer 1: EBOOT Hooks (PPSSPP cheat patches) [IN PROGRESS] │
│  - VFS redirect → load mods from ms0:/PSP/MODS/            │
│  - Extended memory / garage / performance patches          │
└────────────────────────────────────────────────────────────┘
```

## Quick Start

```powershell
# 1. Build the modded core scripts
.\mod_loader\build_modded_core.ps1

# 2. Create a new mod
python mod_loader/cli/gtpsp_mod.py init my_first_mod
cd my_first_mod

# 3. Edit main.ad to add hooks, then build
python ../mod_loader/cli/gtpsp_mod.py build .

# 4. Deploy to PPSSPP
python ../mod_loader/cli/gtpsp_mod.py deploy . --ppsspp-dir <your_ppsspp_memstick>
```

## Structure

```
mod_loader/
├── cli/                          # Python CLI tool
│   ├── gtpsp_mod.py              # Main CLI entry point
│   └── requirements.txt          # PyYAML dependency
├── core/                         # Modified core scripts
│   ├── mod_loader.ad             # Runtime hook registry
│   ├── mod_hooks.inc             # Hook point index/documentation
│   ├── mod_sdk.inc               # Mod development kit (MOD_* helpers)
│   ├── Application_patched.ad    # Modified entry point
│   ├── main_loop_patched.ad      # Modified game loop with hooks
│   └── packed_main_loop_patched.yaml
├── eboot/                        # EBOOT analysis + PPSSPP patches
│   ├── analysis_guide.md         # Ghidra walkthrough
│   ├── vfs_scanner.py            # Binary scanner tool
│   ├── vfs_addresses.json        | Analysis results (fill as you go)
│   └── cheat_patches.ini         # PPSSPP cheat file template
├── examples/                     # Example mods
│   ├── all_cars_garage/          # Give all cars on boot
│   ├── custom_hud/               # Custom race HUD example
│   ├── infinite_garage/          # Remove garage limit
│   └── skip_intro/               # Skip intro movies
├── mod_sdk/                      # Mod development templates
│   ├── template.ad               # Empty mod template
│   ├── mod_manifest.yaml         # Manifest reference
│   └── mod_api.inc               # (future) extended API
├── tests/                        # Test scripts
└── build_modded_core.ps1         # Build + deploy script
```

## Hook System

Mods can hook into **any of 30+ game functions** via three hook types:

| Type | Behavior | Use Case |
|---|---|---|
| `override` | Replaces the function entirely. Receives original_fn + args. | Complete behavior changes |
| `pre` | Runs before original function. Receives args. | Logging, side effects |
| `post` | Runs after original function. Receives result + args. | Reacting to events |

### Key Hookable Functions

| Function Path | Event |
|---|---|
| `::main::MainLoop::MENU` | When entering the menu sequence |
| `::main::MainLoop::RACE` | When entering a race |
| `::main::GameSequence::setNextSequence` | When changing game mode |
| `::main::GameSequence::setNextProject` | When switching UI projects |
| `::main::SequenceUtil::startPage` | When opening a UI page |
| `::main::GlobalStatus::checkout` | When accessing save data |
| `::main::GlobalStatus::checkin` | When writing save data |
| `::main::DialogUtil::openConfirmDialog` | When showing a dialog |
| See `mod_hooks.inc` for the full list. |

## Mod API (mod_sdk.inc)

```c
#include "mod_loader/core/mod_sdk.inc"

function _mod_init() {
    // Override a function
    MOD_OVERRIDE("::main::SequenceUtil::startPage", function(orig, ctx, page) {
        if (page == SomePage) return;  // block this page
        return orig(ctx, page);
    });

    // Pre-hook
    MOD_PRE_HOOK("::main::GlobalStatus::checkout", function(args) {
        MOD_LOG("Save data accessed");
    });

    // Player data shortcuts
    MOD_GIVE_ALL_CARS();
    MOD_SET_CASH(99999999);
}
```

## EBOOT Analysis Status

- [ ] EBOOT loaded in Ghidra (MIPS R4000 LE)
- [ ] VFS load function identified
- [ ] Memory stick redirect patch written
- [ ] Heap/garage patches written
- [ ] Cheat patches tested

See `eboot/analysis_guide.md` for detailed instructions on helping with this effort.

## PPSSPP Setup

1. Enable cheats: **Settings → System → Enable Cheats → ON**
2. Enable file replacement: **Settings → Tools → Developer Tools → Install Device to insta.. → ON**
   OR manually place files in `PPSSPP_MEMSTICK/PSP/UMD0/PSP_GAME/USRDIR/GT.VOL/`
3. Place cheat patches at: `PPSSPP_MEMSTICK/PSP/CHEATS/UCES01245.ini`
4. Restart the game

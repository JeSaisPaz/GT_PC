# Creating GT PSP Mods

A comprehensive guide to creating mods for Gran Turismo PSP using the Mod Loader framework.

## Table of Contents

1. [Overview](#overview)
2. [Prerequisites](#prerequisites)
3. [Mod Loader Architecture](#mod-loader-architecture)
4. [Quick Start: Your First Mod](#quick-start-your-first-mod)
5. [The Mod Manifest](#the-mod-manifest)
6. [Event System & Hook API](#event-system--hook-api)
7. [Mod Development Kit (mod_sdk.inc)](#mod-development-kit)
8. [Adhoc Language Primer](#adhoc-language-primer)
9. [Mod Types & Patterns](#mod-types--patterns)
   - [Simple Gameplay Tweaks](#simple-gameplay-tweaks)
   - [Car Mods](#car-mods)
   - [UI Mods](#ui-mods)
   - [Track/Course Mods](#trackcourse-mods)
   - [Save Data Mods](#save-data-mods)
   - [Sound/Audio Mods](#soundaudio-mods)
   - [Texture/Asset Replacement Mods](#textureasset-replacement-mods)
10. [Advanced: Custom UI Projects](#advanced-custom-ui-projects)
11. [Advanced: EBOOT Patches (PPSSPP Cheats)](#advanced-eboot-patches)
12. [Building an ISO with Mods](#building-an-iso-with-mods)
13. [Testing & Debugging](#testing--debugging)
14. [Publishing Mods](#publishing-mods)
15. [Reference: Hook Points](#reference-hook-points)
16. [Reference: Game Paths & Namespaces](#reference-game-paths--namespaces)
17. [Reference: SpecDB Tables](#reference-specdb-tables)
18. [Troubleshooting](#troubleshooting)

---

## Overview

The GT PSP Mod Loader lets you modify Gran Turismo PSP without editing the original game files directly. It works by:

1. **Patching the core scripts** (`Application.adc`, `packed_main_loop.adc`) to inject a `ModLoader` runtime module
2. **Event-driven hooks** — mods register callbacks for game events (menu open, race start, etc.)
3. **File replacement** — mods can replace any asset in `GT.VOL` (textures, models, audio, XML configs)
4. **EBOOT patches** — optional memory-level patches via PPSSPP cheat engine

The mod loader layer sits between the game engine and your mod, providing a safe API that won't conflict with other mods.

---

## Prerequisites

### Required Tools

| Tool | Purpose | Location |
|------|---------|----------|
| **GTAdhocToolchain** (`adhoc.exe`) | Compiles `.ad` source to `.adc` bytecode | `workflow/adhoc-toolchain/adhoc.exe` |
| **Python 3.8+** | CLI tool (`gtpsp-mod`) | System install |
| **PyYAML** | Manifest parsing | `pip install pyyaml` |
| **PPSSPP** (optional) | Emulator for testing | Any recent version |
| **GTPSPVolTools** (optional) | Repack `GT.VOL` for ISO building | `workflow/gtpspvoltools/GTPSPVolTools.exe` |
| **pycdlib** (optional) | ISO building (`gtpsp-mod iso create`) | `pip install pycdlib` |

### Project Structure

```
GTPSP-decompile/
├── source/                         # Original game source code (.ad files)
│   ├── scripts/gt5m/              # Core game logic
│   │   ├── util/                  # Utility scripts
│   │   └── global_status/         # Save data structures
│   ├── projects/gt5m/             # UI project scripts
│   │   ├── arcade/                # Arcade mode (27 source files)
│   │   ├── race/                  # Race modules (24 source files)
│   │   ├── detail/                # Popup implementations
│   │   ├── dialog/                # Dialog system
│   │   ├── option/                # Options menu
│   │   └── ... (15 projects total)
│   └── products/gt5m/script/      # Menu class definitions
├── mod_loader/                     # The mod loader framework (YOU ARE HERE)
│   ├── cli/                       # gtpsp-mod CLI tool
│   ├── core/                      # Mod loader core (mod_loader.ad, mod_sdk.inc, etc.)
│   ├── examples/                  # Example mods
│   ├── mod_sdk/                   # Templates for new mods
│   ├── eboot/                     # EBOOT analysis + PPSSPP cheat patches
│   │   ├── analysis_guide.md      # Ghidra walkthrough for EBOOT hacking
│   │   ├── cheat_patches.ini      # PPSSPP cheat file template
│   │   └── vfs_addresses.json     # Analysis results (fill as you reverse-engineer)
│   ├── tests/                     # (empty — for future automated tests)
│   └── README.md                  # Framework overview
└── files/decompiled/              # Extracted game files
    └── Gran Turismo/PSP_GAME/USRDIR/GT.VOL/
        ├── scripts/gt5m/          # Original compiled scripts (.adc)
        ├── car/                   # Car models & data
        ├── crs/                   # Track data
        ├── piece_gt5m/            # UI textures
        ├── textdata/gt5m/         # XML configs
        └── ...
```

---

## Quick Start: Your First Mod

### Step 1: Build the Modded Core

This patches the game's entry point and main loop with ModLoader hooks:

```powershell
.\mod_loader\build_modded_core.ps1
```

This produces:
- `test_output/modded_core/scripts/gt5m/Application.adc` — patched entry point
- `test_output/modded_core/scripts/gt5m/packed_main_loop.adc` — patched main loop with ModLoader

### Step 2: Create a Mod Project

```powershell
python mod_loader/cli/gtpsp_mod.py init my_first_mod
```

This creates:
```
my_first_mod/
├── manifest.yaml          # Mod metadata & file list
├── main.ad                # Your mod source code
└── assets/                # (empty) place assets here
```

### Step 3: Write Your Mod

Edit `my_first_mod/main.ad`:

```c
#include "../mod_loader/core/mod_sdk.inc"

function _mod_init()
{
    MOD_ON_EVENT("beforeMenu", function(args) {
        MOD_SET_CASH(99999999);
        MOD_LOG("Cash maxed out!");
    });
}
```

### Step 4: Build

```powershell
python mod_loader/cli/gtpsp_mod.py build my_first_mod/
```

Output: `my_first_mod/build/mods/my_first_mod/main.adc`

### Step 5: Deploy to PPSSPP

```powershell
python mod_loader/cli/gtpsp_mod.py deploy my_first_mod/ --ppsspp-dir <path_to_memstick>
```

This copies your `.adc` to:
`<memstick>/PSP/UMD0/PSP_GAME/USRDIR/GT.VOL/mods/my_first_mod/main.adc`

### Step 6: Deploy Modded Core to PPSSPP

Copy the modded core files:
```
test_output/modded_core/scripts/gt5m/Application.adc
test_output/modded_core/scripts/gt5m/packed_main_loop.adc
```

To:
```
<ppsspp_memstick>/PSP/UMD0/PSP_GAME/USRDIR/GT.VOL/scripts/gt5m/
```

### Step 7: Run in PPSSPP

1. Enable file replacement: **Settings → Tools → Developer Tools → Install Device to insta.. → ON**
2. Launch the game. Your mod should activate on first menu entry.

---

## The Mod Manifest

Every mod needs a `manifest.yaml`. This is the complete reference:

```yaml
# === REQUIRED ===
id: my_mod                      # Unique ID (lowercase, no spaces)
version: "1.0.0"                # Semantic version
name: "My Mod"                  # Display name
description: "What my mod does" # Short description
author: "Your Name"             # Your name/handle

# === SCRIPTS ===
# List of .ad files to compile and where to place the .adc output
scripts:
  - source: main.ad             # Relative to manifest.yaml
    target: mods/my_mod/main.adc   # Relative to GT.VOL root

  # Multi-file mods: list all source files
  - source: car_data.ad
    target: mods/my_mod/car_data.adc

# === ASSETS ===
# Files to copy into the game's virtual filesystem
assets:
  # Replace a car model
  - source: assets/my_car.hq
    target: car/hq/my_car.hq

  # Replace a texture
  - source: assets/my_texture.img
    target: piece_gt5m/some_texture.img

  # Replace an XML config (e.g., add a new track)
  - source: assets/courselist.xml
    target: textdata/gt5m/courselist.xml

# === EBOOT PATCHES ===
# Memory patches applied to EBOOT.BIN (PPSSPP cheat format)
# Addresses are in the PSP memory space (0x08800000+)
eboot_patches:
  - address: "088XXXXX"          # Hex memory address
    value: "XXXXXXXX"            # Hex value(s) to write
    comment: "What this patch does"
```

### Script Target Path Convention

Mod scripts should be placed under `mods/<mod_id>/` in the game's VFS:

| Target Path | Convention |
|---|---|
| `mods/all_cars_garage/main.adc` | ✅ Correct |
| `scripts/gt5m/modified_core.adc` | ❌ Don't overwrite original scripts |

The ModLoader loads mods from `mods/<mod_id>/main.adc` by convention.

---

## Event System & Hook API

The ModLoader provides an event-based hook system. There are **no function overrides** — instead, the patched main loop fires events at key moments, and your mod listens.

### Registering for Events

```c
#include "mod_loader/core/mod_sdk.inc"

function _mod_init()
{
    // Fire-and-forget: callback receives an args array
    MOD_ON_EVENT("beforeMenu", function(args) {
        MOD_LOG("Menu is opening!");
    });

    // One-shot: fires once, then auto-removes
    MOD_ONCE("onGameStart", function(args) {
        MOD_LOG("This runs only once at boot");
    });

    // Query event: return a string to modify game behavior
    MOD_ON_EVENT("getMenuProject", function(args) {
        return "arcade";  // Override which menu opens
    });
}
```

### Event Types

| Type | Function | Behavior |
|---|---|---|
| **Fire-and-forget** | `MOD_ON_EVENT(name, cb)` | All callbacks run. No return value expected. |
| **One-shot** | `MOD_ONCE(name, cb)` | Fires once, then auto-removes. |
| **Query** | Return a string from callback | First non-nil return value wins. Used to override game decisions. |

### Available Events

| Event | Type | When it Fires | Callback Args |
|---|---|---|---|
| `onGameStart` | fire | First MainLoop iteration | `[]` |
| `beforeMenu` | fire | Before a menu project starts | `[]` |
| `afterMenu` | fire | After menu project exits | `[]` |
| `beforeRace` | fire | Before a race executes | `[]` |
| `afterRace` | fire | After a race ends | `[]` |
| `getMenuProject` | query | To determine which menu to open | `[]` — return `"arcade"`, `"gtmode"`, etc. |
| `onShutdown` | fire | Before game shutdown | `[]` |

> **Note**: These are the currently implemented events. The system can be extended — see `mod_hooks.inc` for documentation on adding new hook points. To add more events, edit `main_loop_patched.ad` and insert `::main::ModLoader::fireEvent("yourEvent", [args])` at the desired location.

### Advanced: Mod-to-Mod Communication

You can fire custom events for other mods to listen to:

```c
// Mod A fires a custom event
MOD_FIRE("player_leveled_up", [new_level]);

// Mod B listens
MOD_ON_EVENT("player_leveled_up", function(args) {
    var level = args[0];
    MOD_LOG("Player reached level " + level);
});
```

### Removing Event Listeners

```c
var callback = function(args) { MOD_LOG("fired!"); };

// Register
MOD_ON_EVENT("myEvent", callback);

// Later: unregister
MOD_OFF("myEvent", callback);
```

---

## Mod Development Kit

The `mod_sdk.inc` file (included in your mod via `#include`) provides these helpers:

### Event Helpers

```c
MOD_ON_EVENT(name, callback)     // Listen for an event
MOD_ONCE(name, callback)         // Listen once
MOD_OFF(name, callback)          // Stop listening
MOD_FIRE(name, args_array)       // Fire a custom event
```

### Player Data Helpers

```c
MOD_GET_CASH()                   // Returns current cash (int)
MOD_SET_CASH(amount)             // Sets cash (int)
MOD_GET_GARAGE_SIZE()            // Returns number of owned cars (int)
MOD_GIVE_ALL_CARS()              // Adds all 831+ cars to garage
```

### Debug Helpers

```c
MOD_LOG(message)                 // Print to game's exception log
MOD_SHOW_MESSAGE(ctx, title, msg) // Show a dialog box in-game
```

---

## Adhoc Language Primer

Mods are written in **Adhoc**, the scripting language used by Polyphony Digital for Gran Turismo games (versions 4 through Sport). Here's what you need to know:

### Syntax Basics

```c
// Comments use double-slash

// Variables (dynamic typing)
var x = 42;
var name = "hello";
var flag = true;
var nothing = nil;

// Static variables (persist across function calls, per-module)
static counter = 0;

// Functions
function add(a, b)
{
    return a + b;
}

// Anonymous functions (closures)
var callback = function(arg) {
    return arg * 2;
};

// Control flow
if (x > 10) { /* ... */ }
else if (x > 5) { /* ... */ }
else { /* ... */ }

while (condition) { /* ... */ }

for (var i = 0; i < 10; i++) { /* ... */ }
```

### Arrays & Maps

```c
// Arrays (dynamic)
var list = [1, 2, 3];
list.push(4);
var first = list[0];
var size = list.size;

// Maps (dictionaries)
var map = Map();
map["key"] = "value";
var val = map["key"];

// Mixed literals
var data = [
    "name" : "GT-R",
    "power" : 480,
    "parts" : ["engine", "suspension"]
];
```

### Modules & Namespaces

```c
// Module definition
module ::main::MyModNamespace
{
    static my_var = 0;

    function doSomething()
    {
        // ...
    }
}

// Accessing other modules
::main::ModLoader::fireEvent("eventName", []);
::main::gtengine::MSpecDB::getCarCode("NISSAN_GT-R_07");

// Dynamic module loading
module temp {}
temp.load("mods/my_mod/main");
temp::someFunction();
temp.clearStatic();
```

### Key Differences from JavaScript/C

| Concept | Adhoc | Notes |
|---|---|---|
| `===` | Not available | Use `==` for equality |
| `null` | `nil` | Represents undefined/null |
| `Map()` | Dictionary | Created with `Map()` literal |
| `function` keyword | Same | Functions are first-class |
| `var` | Same | Dynamic typing |
| `static` | Persists in module | Module-level state |
| String interpolation | Not available | Use `+` concatenation |

### Important Gotchas

1. **String length limits**: `STString(N)` creates fixed-length strings. `STString(16)` can hold 16 characters. Writing more truncates silently.
2. **No inheritance**: Adhoc doesn't support class inheritance. Use composition and module patterns.
3. **No exceptions**: `try/catch` exists but is limited. Errors often crash silently.
4. **Module cleanup**: After dynamically loading a module, call `mod.clearStatic()` to free memory.
5. **Case sensitivity**: Everything is case-sensitive.

---

## Mod Types & Patterns

### Simple Gameplay Tweaks

These are the easiest mods — just hook an event and call an API function.

#### Give All Cars + Cash

```c
#include "../mod_loader/core/mod_sdk.inc"

static _initialized = false;

function _mod_init()
{
    MOD_ONCE("beforeMenu", function(args) {
        MOD_GIVE_ALL_CARS();
        MOD_SET_CASH(99999999);
    });
}
```

#### Skip Intro Movies

```c
#include "../mod_loader/core/mod_sdk.inc"

function _mod_init()
{
    MOD_ON_EVENT("getMenuProject", function(args) {
        return "arcade";
    });
}
```

#### Disable BGM (Music)

```c
#include "../mod_loader/core/mod_sdk.inc"

function _mod_init()
{
    MOD_ON_EVENT("beforeMenu", function(args) {
        ::main::SoundUtil::BGMFadeout(0);  // Fade out instantly
    });

    MOD_ON_EVENT("beforeRace", function(args) {
        ::main::SoundUtil::BGMFadeout(0);
    });
}
```

### Car Mods

Car data in GT PSP is stored across several locations:

1. **SpecDB** (`specdb/GT_PSP_JP2817/GENERIC_CAR.dbt`) — Car specifications (power, weight, drivetrain, etc.)
2. **Car models** (`car/hq/`, `car/race/`) — 3D model data
3. **Car textures** — Applied to 3D models
4. **Car sounds** (`carsound/`) — Engine audio
5. **Car list** (`textdata/gt5m/carlist.xml`) — Which cars exist in the game
6. **Dealership configs** (`textdata/gt5m/buy_car/carsetlist*.xml`) — How cars are sold
7. **Localized names** (`specdb/GT_PSP_JP2817/CAR_NAME_*.dbt`) — Names per language

#### Approach A: Script-Level Car Mods (No New Models)

If you want to modify existing car properties at runtime:

```c
#include "../mod_loader/core/mod_sdk.inc"

function _mod_init()
{
    // Modify car stats when the game starts
    MOD_ONCE("onGameStart", function(args) {
        // Get a car code
        var car_code = ::main::gtengine::MSpecDB::getCarCode("NISSAN_GT-R_07");

        // SpecDB operations — modify power, weight, etc.
        // (Requires understanding of SpecDB field names)
        MOD_LOG("Car code: " + car_code);
    });
}
```

#### Approach B: Add Cars via XML + Assets

To add a new car to the game:

1. **Add to car list**: Edit `textdata/gt5m/carlist.xml` to add your car entry
2. **Create model files**: Place in `car/hq/` (high quality) and `car/race/` (in-game)
3. **Set up SpecDB**: Add entries to relevant `.dbt` files (or modify existing entries)
4. **Dealership config**: Add to `carsetlist*.xml` to make it buyable

In your manifest.yaml:
```yaml
scripts:
  - source: main.ad
    target: mods/my_car_mod/main.adc

assets:
  # Car list (add your car reference)
  - source: assets/carlist.xml
    target: textdata/gt5m/carlist.xml

  # Car model (high quality - menus)
  - source: assets/car_data/NISSAN_MY_CAR.hq
    target: car/hq/NISSAN_MY_CAR.hq
  - source: assets/car_data/NISSAN_MY_CAR.00
    target: car/hq/NISSAN_MY_CAR.00
  - source: assets/car_data/NISSAN_MY_CAR.01
    target: car/hq/NISSAN_MY_CAR.01

  # Car model (race - in-game LODs)
  - source: assets/car_data/NISSAN_MY_CAR.race
    target: car/race/NISSAN_MY_CAR.race

  # Car textures
  - source: assets/textures/car_body.img
    target: car/hq/NISSAN_MY_CAR_body.img

  # Dealership config
  - source: assets/carsetlist_mycar.xml
    target: textdata/gt5m/buy_car/carsetlist_mycar.xml
```

#### Car File Format Reference

In `car/` directory:

| File Pattern | Purpose | Notes |
|---|---|---|
| `hq/<maker>_<car>_<year>` | High-quality menu model | LOD level 0 |
| `hq/<maker>_<car>_<year>.00` | Menu LOD 1 | |
| `hq/<maker>_<car>_<year>.01` | Menu LOD 2 | |
| `hq/<maker>_<car>_<year>.02` | Menu LOD 3 | |
| `race/<maker>_<car>_<year>.race` | Race model (optimized) | |
| `race/<maker>_<car>_<year>.race.00` | Race LOD 1 | |
| `info/<maker>_<car>_<year>` | Car metadata | |
| `thumbnail/<maker>_<car>_<year>.img` | Car thumbnail | |
| `thumbnail_L/<maker>_<car>_<year>.img` | Large thumbnail | |
| `interior/<maker>_<car>_<year>` | Interior model (if exists) | |

Car naming convention: `MAKER_CARNAME_YEAR` (e.g., `NISSAN_GT-R_07`, `FERRARI_F40_92`).

### UI Mods

UI in GT PSP is built from:
1. **MWidget projects** (`.mproject`) — Widget tree definitions for each screen
2. **Adhoc scripts** — UI logic (event handlers, transitions)
3. **GPB files** (`piece_gt5m/`) — Texture atlases and UI pieces

#### Approach A: Modify Existing UI Behavior

Hook into UI events and modify behavior:

```c
#include "../mod_loader/core/mod_sdk.inc"

function _mod_init()
{
    // Intercept page navigation
    MOD_ON_EVENT("beforeMenu", function(args) {
        var project = ::main::GameSequence::context.current_project;
        MOD_LOG("Opening project: " + project);

        // Force a different project
        if (project == "boot") {
            ::main::GameSequence::context.current_project = "arcade";
        }
    });
}
```

#### Approach B: Replace UI Textures

Replace textures in `piece_gt5m/`:

```yaml
assets:
  - source: assets/my_custom_button.img
    target: piece_gt5m/button_start.img
    format: RGBA4444
```

UI texture formats:
| Texture Type | Common Format | Size Range |
|---|---|---|
| Icons | RGBA4444 | 32x32, 64x64 |
| Buttons | RGBA4444 | 80x32, 128x64 |
| Background elements | RGB565 | 80x64, 256x64 |
| Text overlays | L8 (luminance) | Various |
| Logos | RGBA5551 | Various |

Common `piece_gt5m/` files:
- `bg_*.img` — Background textures
- `button_*.img` — Button textures
- `icon_*.img` — Icons
- `mark_*.img` — Marks and indicators
- `num_*.img` — Number textures
- `tire_*.img` — Tire icons
- `flag_*.img` — Flag icons
- `result_*.img` — Result screen elements

#### Approach C: Modify UI Scripts

The UI logic lives in `projects/gt5m/` source files. Each project handles a specific screen. To modify UI behavior deeply, you'd need to:

1. Find the relevant source in `source/projects/gt5m/<project>/`
2. Understand the widget tree from the `.mproject` file
3. Copy the relevant source into your mod
4. Modify the logic
5. Use your mod's hooks to replace the UI module

**However**, because UI projects are compiled as standalone `.adc` files loaded by the game, modifying them requires replacing the entire project's `.adc`. You cannot partially override UI scripts via the event system alone. For significant UI changes, you need to:

1. Recompile the entire project (see `source/projects/gt5m/<name>/`)
2. Include the modified `.adc` as an asset in your mod
3. Deploy to the `projects/gt5m/` VFS path

Example manifest for a full UI replacement:
```yaml
assets:
  # Replace entire arcade project (compiled from source/projects/gt5m/arcade/)
  - source: build/arcade.adc
    target: projects/gt5m/arcade/arcade.adc
```

### Track/Course Mods

Track data lives in `crs/`:

```
crs/
├── c001/          # Course ID (maps to track name via courselist.xml)
│   ├── c001.ad    # Binary track asset data
│   ├── c001.cam   # Camera positions/paths
│   ├── c001.cinf  # Course information
│   ├── c001.envptr # Environment pointers
│   ├── c001.layout # Track layout
│   ├── race.mdl   # Race model
│   └── race.txs   # Race textures
├── c001x/         # Course variant (alternate layout)
├── c002/
└── ...
```

Track IDs are mapped to readable names in `textdata/gt5m/courselist.xml`:
```xml
<courselist>
  <course id="c001" name="London" />
  <course id="c002" name="Fuji Speedway" />
  <!-- etc -->
</courselist>
```

#### Course Data Analysis

The course file formats are proprietary and not fully documented. Key files:

| File | Content | Status |
|---|---|---|
| `.ad` | Binary track geometry & collision data | Not documented |
| `.cam` | Camera spline paths | Partially documented |
| `.cinf` | Metadata (length, name, etc.) | Partially documented |
| `.envptr` | Environment/lighting references | Known structure |
| `.layout` | Track layout | Not documented |
| `race.mdl` | Race 3D model | Binary, needs Noesis |
| `race.txs` | Race textures | TXS3 format (convertible) |

To create a custom track, you'd currently need to:
1. Clone an existing course's `crs/XXXX/` directory
2. Modify the binary data (requires custom tools not yet developed)
3. Update `courselist.xml` to register the new course
4. Update `COURSE.dbt` in SpecDB for the new course entry

### Save Data Mods

Player save data is managed through `GlobalStatus`:

```c
#include "../mod_loader/core/mod_sdk.inc"

function _mod_init()
{
    MOD_ON_EVENT("beforeMenu", function(args) {
        // Checkout provides read/write access to save data
        var status = ::main::GlobalStatus::checkout();
        if (status == nil) return;

        // Modify player profile
        status.user_profile.cash = 50000000;

        // Access completed races / licenses
        // status.user_profile.race_complete_flags...
        // status.user_profile.license_flags...

        // Access garage
        var garage = status.user_profile.garage;
        // garage.cars — array of owned car codes
        // garage.addCar(code, 0) — add a car

        // Checkin to save changes
        ::main::GlobalStatus::checkin();
    });
}
```

Important: Always call `checkout()` before reading/writing and `checkin()` after to persist changes. Leaving a checkout open without checkin can cause save corruption.

### Sound/Audio Mods

Audio files in GT PSP:

| Location | Format | Content |
|---|---|---|
| `sound_gt/se/` | `.sgd` | Sound effect banks |
| `sound_gt/library/bgm.lib` | Text index | Music library |
| `sound_gt/library/*.at3` | ATRAC3 | Music tracks |
| `carsound/` | Binary | Per-car engine sounds |

#### Replace Music

1. Convert your audio to ATRAC3 `.at3` format (requires ATRAC3 encoder)
2. Place in `sound_gt/library/`
3. Update `bgm.lib` if adding new tracks

```yaml
assets:
  - source: assets/my_custom_music.at3
    target: sound_gt/library/bgm_mytrack.at3
  - source: assets/bgm.lib
    target: sound_gt/library/bgm.lib
```

#### Audio Conversion Workflow

```bash
# Extract original AT3 files from GT.VOL (already done)
# Convert AT3 to WAV for editing
python scripts/convert_audio.py convert \
    --input "files/decompiled/Gran Turismo/PSP_GAME/USRDIR/GT.VOL/sound_gt" \
    --output "converted/audio"

# Edit WAV in your audio editor (Audacity, etc.)
# Convert back to AT3 (requires Sony ATRAC3 encoder tool)
```

### Texture/Asset Replacement Mods

The mod loader supports replacing any file in `GT.VOL` via the `assets` field in your manifest.

#### Texture Conversion Pipeline

```bash
# Convert game textures to PNG for editing
python scripts/convert_textures.py txs3_to_png \
    -i "files/decompiled/Gran Turismo/PSP_GAME/USRDIR/GT.VOL/piece_gt5m" \
    -o "converted/textures"

# Edit PNGs in your image editor
# Convert back to game format
python scripts/convert_textures.py png_to_txs3 \
    -i "converted/textures/my_edit.png" \
    -o "my_mod/assets/my_edit.img" \
    --format RGBA4444
```

#### Supported Texture Formats

| Format Byte | Name | Bits/Pixel | Use Case |
|---|---|---|---|
| 0x01 | RGBA8888 | 32 | High-quality UI |
| 0x03 | RGBA5551 | 16 | UI with 1-bit alpha |
| 0x04 | RGB565 | 16 | No-alpha textures |
| 0x05 | RGBA4444 | 16 | Standard UI (4-bit alpha) |
| 0x07 | L8 | 8 | Grayscale (text masks) |
| 0x08 | L4 | 4 | Grayscale, low-res |
| 0x0A | DXT1 | 4 | Compressed (1 file only) |

Common UI texture sizes (PSP native: 480x272):
- 80x64 — Standard button
- 80x32 — Narrow button
- 256x64 — Wide element
- 128x128 — Icon/logo
- 16x364 — Tall thin element (scrollbars, etc.)

---

## Advanced: Custom UI Projects

For mods that add entirely new screens or modify existing ones extensively, you need to work with the MWidget system.

### MWidget Overview

UI screens are defined in `.mproject` files. Example (`boot.mproject`):

```
Project{
  name string{"BootProject"}
  has_script digit{1}
  children[1]{
    RootWindow{
      name string{"BootRoot"}
      has_script digit{1}
      children[1]{
        ColorFace{
          name string{"bg"}
          color[1]{ RGBA{0 0 0 255} }
        }
      }
    }
  }
}
```

### Widget Types

From `Prototypes.mwidget` (31+ types):

| Widget | Purpose |
|---|---|
| `RootWindow` | Root of a screen |
| `MColorFace` | Solid color rectangle |
| `MImageFace` | Image/texture display |
| `MTextFace` | Text label |
| `MComposite` | Container for children |
| `MHBox` | Horizontal layout box |
| `MVBox` | Vertical layout box |
| `MFBox` | Flow layout box |
| `MScrollbar` | Scrollbar control |
| `MListBox` | List display |
| `MButton` | Clickable button |
| `MEditBox` | Text input field |
| `MCheckBox` | Toggle/checkbox |

### Creating a New UI Screen

1. Create a new `.mproject` file defining the widget tree
2. Write an `.ad` script with event handlers for your widgets
3. Create a `.yaml` project file listing all source files
4. Compile to `.adc` with `adhoc.exe build -p project.yaml`
5. Include the compiled `.adc` and `.mproject` as assets in your mod

The ModLoader's `getMenuProject` event can redirect to your custom project ID.

---

## Advanced: EBOOT Patches

The EBOOT patching system targets the game's executable (`EBOOT.BIN`) to:

1. **VFS Redirect**: Make the game load files from `ms0:/PSP/MODS/` instead of `GT.VOL`
2. **Extended Garage**: Remove hardcoded garage size limits
3. **Extended Memory**: Alter heap allocation sizes
4. **Infinite Nitrous**: Various gameplay hacks

### How It Works

PPSSPP supports a cheat engine that applies memory patches at load time. Patches are defined in:

`<PPSSPP_MEMSTICK>/PSP/CHEATS/<GAME_ID>.ini`

Format:
```ini
_S UCES-01245
_G Gran Turismo (PSP)

_C0 <Patch Name>
_L 0x<address> 0x<value>
```

### Current EBOOT Analysis Status

See `mod_loader/eboot/analysis_guide.md` for the full Ghidra walkthrough. The VFS redirect patch requires:

1. Load `EBOOT.BIN` in Ghidra (MIPS R4000, Little Endian)
2. Find the VFS file loading function (reads from `GT.VOL`)
3. Identify strings and branch points for the file path
4. Calculate the PPSSPP memory address
5. Write the cheat patch

Key addresses are tracked in `mod_loader/eboot/vfs_addresses.json`.

### Applying Cheat Patches

```powershell
# Generate cheat patches from analysis data
python mod_loader/cli/gtpsp_mod.py patch-eboot

# Output: mod_loader/eboot/generated_patches.ini
```

Or deploy directly:
```powershell
# Deploy a mod that includes EBOOT patches
python mod_loader/cli/gtpsp_mod.py deploy my_mod/ --strategy cheat
```

### Defining EBOOT Patches in a Mod

In your `manifest.yaml`:
```yaml
eboot_patches:
  - address: "088XXXXX"
    value: "XXXXXXXX"
    comment: "My patch description"
```

---

## Building an ISO with Mods

For a standalone modded ISO (no PPSSPP file replacement needed):

### Prerequisites

```powershell
pip install pycdlib
```

### Build ISO

```powershell
# Interactive mod selection
python mod_loader/cli/gtpsp_mod.py iso create "D:\path\to\original.iso" -i

# Or specify mods directly
python mod_loader/cli/gtpsp_mod.py iso create "D:\path\to\original.iso" -m "all_cars_garage,skip_intro"

# Custom output path
python mod_loader/cli/gtpsp_mod.py iso create "original.iso" -o "GTPSP_modded.iso"
```

The ISO builder:
1. Extracts original ISO to temp
2. Unpacks `GT.VOL`
3. Injects modded core scripts
4. Compiles and injects selected mods
5. Repacks `GT.VOL`
6. Applies EBOOT patches if configured
7. Writes new ISO

---

## Testing & Debugging

### The MOD_LOG Function

Log messages are written to the game's exception file:

```c
MOD_LOG("Something happened");
```

On PPSSPP, check the console output or enable logging in:
**Settings → Tools → Logging → Console Log Level → INFO**

### The MOD_SHOW_MESSAGE Dialog

Display a visible in-game dialog:

```c
function _mod_init()
{
    MOD_ONCE("onGameStart", function(args) {
        MOD_SHOW_MESSAGE(
            ::main::GameSequence::context,
            "Mod Loaded",
            "Your mod is working!"
        );
    });
}
```

### Common Testing Flow

1. Make a small change to your mod
2. `python mod_loader/cli/gtpsp_mod.py build <mod>`
3. `python mod_loader/cli/gtpsp_mod.py deploy <mod>`
4. Restart PPSSPP
5. Observe behavior
6. Repeat

### Testing Mod Conflicts

If running multiple mods:

1. The ModLoader combines all callback lists per event
2. All `fireEvent` callbacks run in registration order
3. For query events, the first non-nil return wins
4. Test mods individually first, then together

### Common Issues

| Symptom | Likely Cause |
|---|---|
| Black screen on boot | Core scripts not deployed or corrupt |
| Mod doesn't activate | `manifest.yaml` target path wrong |
| `_mod_init` not called | ModLoader core not built/deployed |
| Game crashes on menu | Script error — check log |
| Save data reset | Missing `checkin()` after `checkout()` |

---

## Publishing Mods

### Mod Distribution Structure

```
my_awesome_mod/
├── manifest.yaml          # Required
├── main.ad                # Source code
├── assets/                # Asset files
│   ├── my_texture.img
│   └── carlist.xml
├── build/                 # Compiled output (gitignore this)
│   └── mods/my_awesome_mod/
│       └── main.adc
└── README.md              # Optional: description + installation
```

### What to Share

Share the entire mod directory (including `manifest.yaml` and `main.ad` source). Users will:

1. Place the mod directory in their workspace
2. Run the CLI build + deploy commands
3. Or use the ISO builder to include it

### Best Practices

1. **Unique mod IDs** — Prefix with your name: `bob_my_mod`
2. **Don't overwrite original scripts** — Place scripts under `mods/<your_id>/`
3. **Check in after checkout** — Always balance `checkout()` with `checkin()`
4. **Use one-shot** for setup code — `MOD_ONCE` for initialization
5. **Version your mods** — Use semver in `manifest.yaml`
6. **Document dependencies** — If your mod depends on another mod's events, document it

---

## Reference: Hook Points

### Currently Implemented

These events fire from `main_loop_patched.ad`'s `MainLoop()`:

| Event | Type | Location in Code | Purpose |
|---|---|---|---|
| `onGameStart` | fire | Start of `MainLoop()` | One-time init |
| `beforeMenu` | fire | Before `MGOM.start()` in MENU case | Pre-menu logic |
| `afterMenu` | fire | After `MGOM.start()` returns | Post-menu logic |
| `beforeRace` | fire | Before `executeRace()` in RACE case | Pre-race setup |
| `afterRace` | fire | After `executeNext()` in RACE case | Post-race cleanup |
| `getMenuProject` | query | Before `MGOM.start()` | Override menu |
| `onShutdown` | fire | Before `execShutdown()` | Cleanup |

### Planned (not yet implemented)

These events can be added to the core scripts by editing `main_loop_patched.ad`:

| Proposed Event | Type | Per-game Function | Suggestion |
|---|---|---|---|
| `onRaceStart` | fire | `doStartRace()` in race project | When race begins |
| `onRaceEnd` | fire | Result screen | When race finishes |
| `onCarSelect` | fire | `CarRoot` | When player picks a car |
| `onCourseSelect` | fire | `CourseRoot` | When player picks a track |
| `onSaveLoad` | fire | After save data loaded | Modify save on load |
| `onSaveWrite` | fire | Before save data written | Intercept save |

### Adding a New Hook Point

To add a new event, edit the appropriate source file (e.g., `main_loop_patched.ad`):

```c
// Before the target operation:
::main::ModLoader::fireEvent("myNewEvent", [arg1, arg2]);

// For a query (mod can override behavior):
var override = ::main::ModLoader::getEventResultString("myQueryEvent");
if (override != nil) {
    // Use override value instead of default
}
```

Then rebuild the core with `build_modded_core.ps1`.

---

## Reference: Game Paths & Namespaces

### Important Namespace Paths

| Path | Module | Purpose |
|---|---|---|
| `::main` | Root | Global state, AppOpt |
| `::main::ModLoader` | Mod Loader | Hook registry |
| `::main::GameSequence` | Game Sequence | State machine (MENU/RACE) |
| `::main::GameSequence::context` | Context | `current_project`, `finished`, etc. |
| `::main::GlobalStatus` | Global Status | Save data (checkout/checkin) |
| `::main::gtengine::MSpecDB` | SpecDB | Database access |
| `::main::pdiext` | Engine API | Fonts, utilities, system |
| `::main::SoundUtil` | Sound | BGM, SFX control |
| `::main::DialogUtil` | Dialog | Confirm dialogs |
| `::main::SequenceUtil` | Sequence | Page navigation |
| `::main::DebugTool` | Debug | Heap status, logging |

### VFS Path Mapping

When deploying files, the target path is relative to `GT.VOL` root. Common paths:

| VFS Path | Contents |
|---|---|
| `scripts/gt5m/` | Core game scripts (.adc) |
| `projects/gt5m/<name>/` | UI project scripts |
| `products/gt5m/script/` | Menu class modules |
| `mods/<mod_id>/` | Mod scripts (your mods go here) |
| `car/hq/` | High-quality car models |
| `car/race/` | Race car models |
| `car/thumbnail/` | Car thumbnails |
| `crs/cXXX/` | Course data |
| `piece_gt5m/` | UI textures |
| `textdata/gt5m/` | XML configurations |
| `sound_gt/` | Audio files |
| `font/` | Font files |
| `specdb/GT_PSP_JP2817/` | SpecDB tables |

---

## Reference: SpecDB Tables

The Specification Database (`specdb/`) contains game data. Key tables:

| Table File | Contents | Key Fields |
|---|---|---|
| `GENERIC_CAR.dbt` | Master car list | code, name, power, weight, drivetrain, displacement |
| `VARIATION.dbt` | Color variations | car_code, color_name, RGB values |
| `MAKER.dbt` | Manufacturers | code, name, country |
| `TUNER.dbt` | Tuning companies | code, name |
| `ENGINE.dbt` | Engine specs | code, power, torque, rpm |
| `SUSPENSION.dbt` | Suspension specs | code, spring rates, dampers |
| `CHASSIS.dbt` | Chassis data | code, weight, rigidity |
| `DRIVETRAIN.dbt` | Drivetrain | type (FF/FR/MR/4WD), ratios |
| `GEAR.dbt` | Gearbox ratios | gear_1 through gear_6, final |
| `FRONTTIRE.dbt` / `REARTIRE.dbt` | Tire data | width, aspect_ratio, diameter |
| `BRAKE.dbt` | Brake specs | power, balance |
| `LSD.dbt` | Limited slip diff | initial, accel, decel |
| `TURBINEKIT.dbt` | Turbo/supercharger | boost_pressure, power_gain |
| `COURSE.dbt` | Track data | code, length, country, layout_type |
| `RACE.dbt` | Race events | course, laps, opponents, restrictions |
| `DEFAULT_PARTS.dbt` | Default car setups | car_code, parts list |
| `ENEMY_CARS.dbt` | AI opponent cars | car_code, skill_level |

To access SpecDB from a mod:
```c
// Get car code from name
var code = ::main::gtengine::MSpecDB::getCarCode("NISSAN_GT-R_07");

// Get car label from code
var label = ::main::gtengine::MSpecDB::getCarLabel(code);

// Get course code
var course = ::main::gtengine::MSpecDB::getCourseCode("FUJI");

// Get all car labels
var all_cars = ::main::gtengine::MSpecDB::getCarLabelList();
```

---

## Troubleshooting

### Build Fails

| Error | Fix |
|---|---|
| `adhoc.exe not found` | Download GTAdhocToolchain v1.3.5+ to `workflow/adhoc-toolchain/` |
| `pyyaml not installed` | `pip install pyyaml` |
| `#include file not found` | Check the include path in your `.ad` file. It's relative to the `.ad` file's location. |
| `Syntax error in .ad` | Check for missing semicolons, unmatched braces, or unsupported constructs |

### Runtime Issues

| Problem | Check |
|---|---|
| Game loads but mod doesn't work | 1. Is `Application_patched.adc` and `packed_main_loop.adc` deployed? 2. Is your mod's `.adc` at the target path? 3. Does your mod compile without errors? |
| `_mod_init` not called | Verify your mod's `.adc` is at the path specified in `manifest.yaml` target |
| Black screen | Core scripts not deployed, or corrupt. Rebuild and redeploy. |
| Crash when entering a menu | Event callback has an error. Use `MOD_LOG` to narrow down. |
| Mod works once then never again | Use `MOD_ONCE` for one-time setup, or check if your `static` flag logic is correct |
| Game doesn't see new cars | Check `carlist.xml` format and SpecDB entries |
| Textures show incorrectly | Wrong format in conversion (use RGBA4444 for most UI textures) |

### PPSSPP Setup

```powershell
# Run the setup script to verify your environment
.\mod_loader\setup.ps1
```

PPSSPP requirements:
1. **Enable file replacement**: `Settings → Tools → Developer Tools → Install Device to insta.. → ON`
2. Or use cheat patches: `Settings → System → Enable Cheats → ON`
3. Deploy to: `<PPSSPP_MEMSTICK>/PSP/UMD0/PSP_GAME/USRDIR/GT.VOL/`
4. Restart the game after deploying

---

## Appendix: Full Example — Custom Car with Data

This example shows a complete mod that adds a custom car with modified SpecDB data, custom textures, and a script that grants it to the player.

### Directory Structure

```
super_car_mod/
├── manifest.yaml
├── main.ad
├── assets/
│   ├── carlist.xml
│   ├── car_body.img
│   └── car_specs.txt
```

### manifest.yaml

```yaml
id: super_car_mod
version: "1.0.0"
name: "Super Car Pack"
description: "Adds a custom super car to the game with unique specs."
author: "Your Name"
scripts:
  - source: main.ad
    target: mods/super_car_mod/main.adc
assets:
  - source: assets/carlist.xml
    target: textdata/gt5m/carlist.xml
  - source: assets/car_body.img
    target: piece_gt5m/car_icon_super.img
eboot_patches: []
```

### main.ad

```c
#include "../mod_loader/core/mod_sdk.inc"

static _initialized = false;

function _mod_init()
{
    if (_initialized) return;
    _initialized = true;

    // Give the custom car on first menu entry
    MOD_ONCE("beforeMenu", function(args) {
        var status = ::main::GlobalStatus::checkout();
        if (status == nil) return;

        // Try to add a car by code (if it exists in SpecDB)
        var code = ::main::gtengine::MSpecDB::getCarCode("SUPER_CAR_XX");
        if (code != nil && code > 0) {
            status.user_profile.garage.addCar(code, 0);
            MOD_LOG("Super Car added to garage!");
        } else {
            MOD_LOG("Car not found in SpecDB — check carlist.xml");
        }

        // Set some starting cash
        if (status.user_profile.cash < 100000) {
            status.user_profile.cash = 5000000;
        }

        ::main::GlobalStatus::checkin();
    });

    MOD_LOG("Super Car Mod loaded!");
}
```

This comprehensive guide should give you everything you need to create mods ranging from simple gameplay tweaks to complex car and UI modifications. For further assistance, refer to the source code in `source/` for game logic patterns, and join the GT Modding community for format-specific questions.

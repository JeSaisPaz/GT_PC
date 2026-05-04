# Gran Turismo PSP Decompilation Project

## Overview
This project aims to decompile and analyze Gran Turismo for PSP (Gran Turismo PSP, also known as Gran Turismo 5 Mobile / gt5m). The project involves reverse engineering the game's binary files, understanding its architecture, and creating comprehensive documentation of the game's internal structure.

**Internal Codename**: `gt5m` (Gran Turismo 5 Mobile)
**Adhoc Version**: 12 (same as GT5/GT6/GT Sport)
**Region**: EU (UCES01245) - Build JP2817

## Decompilation Status

### Completed Tasks
- **GT.VOL Extraction**: 21,211 files extracted using GTPSPVolTools
- **ADC Script Disassembly**: 71 ADC files disassembled to `.ad.diss` format (~7.35 MB total)
- **Full Source Code Recovery**: 163 `.ad` source files (15 projects + standalone scripts) from OpenAdhoc (by pez2k), 100% of game logic
- **Build Pipeline**: All projects compile with GTAdhocToolchain v1.3.5; verified against original `.adc` via GTAdhocCompare.py
- **Project Structure Documentation**: Complete mapping of game architecture

### Generated Files
| File Type | Count | Description |
|-----------|-------|-------------|
| `.ad` | 163 | Reconstructed Adhoc source code (from OpenAdhoc) |
| `.ad.diss` | 71 | Disassembled Adhoc bytecode |
| `.strings` | 71 | Extracted string tables |
| `.yaml` | 15 | Project build configuration files |
| `.h` | 2 | Adhoc header files |

## Project Structure

### Directory Layout
```
GTPSP-decompile/
├── files/                    # Game files (original and decompiled)
│   ├── original/            # Original game files from ISO
│   │   └── Gran Turismo/
│   │       ├── PSP_GAME/
│   │       │   ├── SYSDIR/     # System files (EBOOT.BIN, BOOT.BIN)
│   │       │   ├── USRDIR/     # User files (GT.VOL, MODULE)
│   │       │   └── [PSP metadata files]
│   │       └── UMD_DATA.BIN
│   └── decompiled/          # Decompiled/extracted game files
│       └── Gran Turismo/
│           └── PSP_GAME/
│               └── USRDIR/
│                   └── GT.VOL/  # Extracted game archive (21,211 files)
│                       ├── advertise/     # Advertising/marketing assets
│                       ├── car/           # Car models and data
│                       ├── carsound/      # Car audio files
│                       ├── character/     # Character models
│                       ├── crs/           # Course/track data (109 tracks)
│                       ├── description/   # Text descriptions
│                       ├── font/          # Font files
│                       ├── icon/          # Icon graphics
│                       ├── movie/         # Video files (.pmf)
│                       ├── piece_gt5m/    # UI pieces/elements
│                       ├── products/      # Product definitions
│                       ├── projects/      # Game project files (UI screens)
│                       ├── replay/        # Replay data
│                       ├── scripts/       # Game scripts (ADC files)
│                       ├── sound_gt/      # Sound effects and music
│                       ├── specdb/        # Specification database
│                       ├── textdata/      # Localization and XML configs
│                       └── wheel/         # Wheel models
├── source/                  # Reconstructed Adhoc source code (.ad files)
│   ├── scripts/gt5m/        # Core game logic (bootstrap, main loop, sound)
│   │   ├── util/            # Utility scripts (SpecDB, SaveData, etc.)
│   │   └── global_status/   # Global game state management
│   ├── projects/gt5m/       # UI project scripts (arcade, race, gtmode, etc.)
│   │   ├── arcade/          # Arcade mode (largest project, 27 source files)
│   │   ├── race/            # Race modules (24 source files)
│   │   ├── detail/          # Detail popup implementations
│   │   ├── dialog/          # Dialog system
│   │   ├── option/          # Options menu
│   │   └── ...              # 15 project modules total
│   └── products/gt5m/script/ # Menu class definitions
├── scripts/                 # Build and utility scripts
│   ├── build_all.ps1        # Full build pipeline
│   └── compare_all.ps1      # Verify recompiled vs original .adc
├── workflow/                # Tools for decompilation
│   ├── adhoc-toolchain/    # GT Adhoc decompiler/compiler (v1.3.5)
│   ├── ghidra_12.0_PUBLIC/ # Reverse engineering framework
│   ├── GT2TextureEditor/   # GT1/2 texture editor
│   ├── GT3PMBDumper/       # GT3 menu extractor
│   ├── GTMusicInfEditor/   # Music metadata editor
│   ├── gtpspvoltools/      # GT.VOL packer/unpacker
│   ├── img-buster/         # TXS3/IMG texture converter
│   ├── noesis/             # 3D model viewer/converter
│   ├── prxtool-master/     # PSP PRX file tool
│   ├── quickbms/           # Universal archive extractor
│   ├── seq2midi/           # Sequence to MIDI converter
│   ├── TSX3Converter/      # Texture set converter
│   └── xxd/                # Hex dump utility
├── test_output/             # Comparison reports and test artifacts
├── PROJECT.md               # This documentation
└── openadhoc_repo/          # Cloned OpenAdhoc repository (reference)
```

## Technical Architecture

### PSP Platform Specifications
- **CPU**: MIPS R4000 32-bit (333 MHz)
- **GPU**: Custom Sony GPU with 2MB VRAM
- **RAM**: 32MB main memory
- **Storage**: UMD (Universal Media Disc) or Memory Stick
- **Audio**: Sony VME (Virtual Mobile Engine) for audio processing

### Game File Structure

#### 1. **EBOOT.BIN** (Main Executable)
- **Location**: `SYSDIR/EBOOT.BIN`
- **Size**: 7,058,320 bytes
- **Purpose**: Main game executable
- **Format**: PSP executable (PRX format variant)
- **Analysis**: Can be analyzed with Ghidra/PRXTool

#### 2. **GT.VOL** (Main Game Archive)
- **Location**: `USRDIR/GT.VOL`
- **Size**: 1,053,487,104 bytes (~1GB)
- **Purpose**: Contains all game assets (models, textures, audio, scripts)
- **Format**: Proprietary Polyphony Digital archive format
- **Compression**: Mixed (zstd compression for some files)
- **Extraction**: Using GTPSPVolTools
- **Total Files**: 21,211 files extracted

#### 3. **PRX Modules** (System Libraries)
- **Location**: `USRDIR/MODULE/`
- **Files**:
  - `LIBFONT.PRX` (20,720 bytes): Font rendering library
  - `LIBSUPPREACC.PRX` (19,408 bytes): Supplementary accessibility library
  - `PSMF.PRX` (6,800 bytes): PlayStation Media Framework
- **Purpose**: PSP system libraries loaded at runtime

### File Formats Analysis

#### File Type Distribution (21,211 total files)
| Extension | Count | Description |
|-----------|-------|-------------|
| (no ext) | 9,539 | Binary data files |
| .00-.30 | ~8,000 | Indexed asset files (LOD levels, variants) |
| .img | 733 | Image/texture files |
| .gpb | 566 | UI asset containers |
| .xml | 289 | Configuration and data files |
| .bin | 184 | Binary data files |
| .ad | 109 | Course/track binary data |
| .rt2 | 111 | Unknown format |
| .mpnt | 105 | Map point data |
| .dat | 102 | Generic data files |
| .cinf | 82 | Course information |
| .cam | 79 | Camera data |
| .envptr | 79 | Environment pointers |
| .adc | 71 | Adhoc compiled scripts |
| .at3 | 56 | Sony ATRAC3 audio |
| .dbt | 54 | Database tables |
| .idi | 54 | Database indices |
| .sdb | 10 | String databases |
| .mproject | 13 | UI project definitions |
| .mwidget | 1 | Widget prototypes |

#### 1. **ADC Files** (Adhoc Compiled Scripts)
- **Location**: `scripts/gt5m/`, `projects/gt5m/`, `products/gt5m/`
- **Purpose**: Game logic, UI handling, event processing
- **Format**: Adhoc bytecode Version 12
- **Total Count**: 71 ADC files
- **Disassembly**: Generated `.ad.diss` and `.strings` files

**Core Scripts** (`scripts/gt5m/`):
| Script | Size | Purpose |
|--------|------|---------|
| `Application.adc` | 1KB | Main application entry point |
| `bootstrap.adc` | 5KB | Game initialization |
| `bootstrap_phase2.adc` | 3KB | Secondary initialization |
| `packed_main_loop.adc` | 14KB | Main game loop |
| `init_sound.adc` | 19KB | Sound system initialization |
| `shutdown.adc` | 1KB | Game shutdown handler |

**Project Scripts** (`projects/gt5m/`):
| Module | Script Count | Purpose |
|--------|-------------|---------|
| `arcade/` | 2 | Arcade mode (2.3MB disassembly!) |
| `boot/` | 1 | Boot sequence |
| `config/` | 1 | Configuration (gt5m.adc) |
| `cursor/` | 1 | Cursor handling |
| `detail/` | 9 | Detail popup implementations |
| `dialog/` | 1 | Dialog system |
| `gtmode/` | 1 | GT Mode |
| `install/` | 1 | Installation handler |
| `manual/` | 12 | Manual configs (per language) |
| `option/` | 1 | Options menu |
| `play_movie/` | 1 | Movie player |
| `race/` | 18 | Race modules and implementations |
| `ranking/` | 1 | Rankings system |
| `ui_kit/` | 1 | UI toolkit |

**Utility Scripts** (`scripts/gt5m/util/`):
- `ArcadeDifficultyUtil.adc` - AI difficulty handling
- `EventFlagsUtil.adc` - Event flag management
- `GamePlanImpl.adc` - Game plan implementation
- `LicenseUtil.adc` - License system utilities
- `MakerUtil.adc` - Car manufacturer utilities
- `OrdinalUtil.adc` - Ordinal number formatting
- `RewardUtil.adc` - Reward system
- `SaveDataUtilPSP.adc` - PSP save data handling
- `SpecDatabaseUtil.adc` - Spec database access
- `USBPSPCommPSP.adc` - USB communication (GT5 link)
- `VoucherUtil.adc` - Voucher/DLC handling

#### 2. **UI Project Files** (.mproject / .mwidget)
- **Location**: `projects/gt5m/*/` and `products/gt5m/script/`
- **Format**: Text-based UI layout definitions
- **Purpose**: Define UI widget trees and properties

**Example** (`boot.mproject`):
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

#### 3. **Database Files** (SpecDB)
- **Location**: `specdb/GT_PSP_JP2817/`
- **Total Files**: 123 database files
- **Build ID**: JP2817

**Database Tables (.dbt)**:
| Table | Purpose |
|-------|---------|
| `GENERIC_CAR.dbt` | Master car specifications |
| `ENGINE.dbt` | Engine data |
| `SUSPENSION.dbt` | Suspension specs |
| `CHASSIS.dbt` | Chassis data |
| `DRIVETRAIN.dbt` | Drivetrain specs |
| `GEAR.dbt` | Gearbox ratios |
| `FRONTTIRE.dbt` / `REARTIRE.dbt` | Tire data |
| `BRAKE.dbt` | Brake specs |
| `LSD.dbt` | Limited slip differential |
| `TURBINEKIT.dbt` | Turbo/supercharger kits |
| `COURSE.dbt` | Track specifications |
| `RACE.dbt` | Race event data |
| `VARIATION.dbt` | Car color variations |
| `MAKER.dbt` / `TUNER.dbt` | Manufacturers |
| `CAR_NAME_*.dbt` | Localized car names (9 languages) |

**String Databases (.sdb)**:
- `UnistrDB.sdb` - Universal strings
- `*_StrDB.sdb` - Localized strings (japanese, american, british, french, german, italian, spanish, big5, korean)

#### 4. **XML Configuration Files**
- **Location**: `textdata/gt5m/`
- **Total**: 289 XML files
- **Purpose**: Game configuration and data definitions

**Key XML Files**:
- `courselist.xml` - Track definitions (45 courses)
- `carlist.xml` - Car roster
- `makerlist.xml` - Manufacturer list
- `enemylist.xml` - AI opponent definitions
- `shufflelist.xml` - Random selection pools
- `license/license_User*.xml` - License test configs (83 tests)
- `buy_car/carsetlist*.xml` - Dealership configurations

#### 5. **Course Data** (`crs/`)
- **Total**: 444 files for 109 track configurations
- **File Types**:
  - `.ad` - Binary track asset data
  - `.cam` - Camera positions and paths
  - `.cinf` - Course information
  - `.envptr` - Environment pointers
  - `.layout` - Track layout data
  - `*x/` - Track variant folders
  - `race.mdl` / `race.txs` - Race assets

#### 6. **Texture Files**
- **Formats**: `.img` (733), `.gpb` (566), `.txs` (2), `.tsx3` (3)
- **Purpose**: Game textures (cars, tracks, UI)
- **Conversion**: Using `scripts/convert_textures.py` (custom tool)
- **PSP-specific**: RGB565, RGBA4444, RGBA5551, L8, L4 formats
- **Status**: 707/708 textures successfully converted to PNG (DXT1 unsupported)

#### 7. **Audio Files**
- **Formats**: `.sgd` (4), `.at3` (56), `.lib` (1)
- **Purpose**: Sound effects, music, engine sounds
- **Types**:
  - `.sgd`: Sound effect groups
  - `.at3`: Sony ATRAC3 audio (music)
  - `.lib`: Music library index
  - `carsound/`: Per-car engine audio

#### 8. **Video Files**
- **Format**: `.pmf` (2)
- **Purpose**: Intro/logo videos
- **Playback**: PSMF (PlayStation Media Framework)

## Decompilation Workflow

### Step 1: Archive Extraction (COMPLETED)
```powershell
# Extract GT.VOL using GTPSPVolTools
GTPSPVolTools.exe unpack -i "GT.VOL" -o "extracted"

# Result: 21,211 files extracted
# Generated files.txt with file list and metadata
```

### Step 2: Script Disassembly (COMPLETED)
```powershell
# Disassemble all ADC scripts using adhoc-toolchain
$adcFiles = Get-ChildItem -Path "files\decompiled" -Filter "*.adc" -Recurse
foreach ($file in $adcFiles) {
    & "workflow\adhoc-toolchain\adhoc.exe" $file.FullName
}

# Result: 71 .ad.diss files + 71 .strings files generated
# Total disassembly size: ~7.35 MB
```

**Disassembly Output Format** (`.ad.diss`):
```
==== Disassembly generated by GTAdhocToolchain ====
Original File Name: scripts/gt5m/bootstrap.ad
Version: 12
(112 strings)
Root Instructions: 45
  > Stack Size: 2 - Variable Storage Size: 1 - Variable Storage Size Static: 24
   5E6|  19|  0| MODULE_DEFINE: main,main
   5F1|  21|  1| STATIC_DEFINE: PROJECT_ROOT_DIR
   ...
```

### Step 3: Binary Analysis (PPSSPP Runtime)

The EBOOT.BIN uses PSP KIRK retail encryption — raw binary scan finds NO readable strings or code.
**Ghidra CLI bridge cannot decrypt PSP PRX files** (compatibility issue with Ghidra 12 API changes).

**Solution**: Use PPSSPP's runtime + WebSocket API to access the decrypted EBOOT in PSP memory.

#### 3a. PPSSPP WebSocket Debugger (Preferred — No Ghidra Required)

PPSSPP has a built-in WebSocket API on the same port as disc sharing. It provides programmatic access
to breakpoints, memory, registers, backtraces, disassembly, and HLE function listings.

**Architecture:**
```
┌─────────────┐     ws://host:port/debugger      ┌──────────────┐
│ Node.js CLI │──── debugger.ppsspp.org ────────▶│   PPSSPP     │
│ (our script)│    JSON event protocol            │  Emulator    │
└─────────────┘                                   └──────────────┘
Events: cpu.breakpoint.add, memory.read, hle.func.list, cpu.getReg, ...
```

**Setup:**
1. Enable in PPSSPP: **Settings → Tools → Developer Tools → Allow remote debugger → ON**
2. Or set in `ppsspp.ini`:
   ```ini
   [General]
   RemoteDebuggerOnStartup = True
   RemoteISOPort = 8833       ; WebSocket + disc share port
   ```
3. Launch PPSSPP with log console: `PPSSPPWindows64.exe -l -d --windowed "<game.iso>"`
4. Run analysis script: `node mod_loader/eboot/vfs_analyzer.js`

**Available API Events** (from `Core/Debugger/WebSocket/`):

| Category | Events | Purpose |
|----------|--------|---------|
| **Breakpoints** | `cpu.breakpoint.add/remove` | Set/watch execution breakpoints |
| **CPU** | `cpu.getReg`, `cpu.setReg` | Read/write MIPS registers (a0-a3, ra, sp, pc) |
| **Memory** | `memory.read`, `memory.write`, `memory.readString` | Read/write decrypted PSP RAM |
| **Disassembly** | `disasm.dump` | Get MIPS disassembly of any address range |
| **Functions** | `hle.func.list`, `hle.func.rename` | List/rename detected HLE stubs |
| **Backtrace** | `hle.backtrace` | Walk stack frames (call chain) |
| **Game** | `game.status`, `game.reset` | Game state |
| **Logging** | log events (broadcast) | Real-time emulator log stream |
| **Stepping** | cpu.stepping (broadcast) | Fires on breakpoint hits |

**Workflow for VFS Address Discovery:**
```javascript
// Step 1: Connect
ppsspp.send({ event: 'version', name: 'tool', version: '1.0' });

// Step 2: List HLE stubs — find zz_sceIoOpen
ppsspp.send({ event: 'hle.func.list' })
  // → find f.name === 'zz_sceIoOpen', get f.address

// Step 3: Set breakpoint on sceIoOpen
ppsspp.send({ event: 'cpu.breakpoint.add', address: sceIoOpenAddr });

// Step 4: On breakpoint hit (cpu.stepping event):
ppsspp.send({ event: 'cpu.getReg', name: 'a0' })   // → filename
ppsspp.send({ event: 'cpu.getReg', name: 'ra' })   // → caller address
ppsspp.send({ event: 'memory.readString', address: a0.uintValue });

// Step 5: Resume
ppsspp.send({ event: 'cpu.resume' });
```

**See:** `mod_loader/eboot/vfs_analyzer.js` — complete automated analysis script.

#### 3b. Decrypting EBOOT.BIN via PPSSPP Memory Dump

Since PPSSPP decrypts the EBOOT at runtime (handles KIRK decryption internally),
you can dump the decrypted code from PSP RAM after the game loads:

```powershell
# Launch PPSSPP with game, then:
node mod_loader\eboot\eboot_dump.js
# → Connects via WebSocket, waits for boot, dumps 0x08800000-0x0A000000
# → Writes decrypted.bin + decrypted.asm (disassembly) + decrypted.json (symbols)
```

The dumped binary can then be analyzed with any MIPS tool:
- **ghidra-cli** (for Ghidra-backed queries on the decrypted dump)
- **Ghidra GUI** (import as Raw Binary, MIPS:LE:32:default)
- **PRXTool** (`prxtool -i decrypted.bin -o output.idc`)

#### 3c. PRX Module Analysis
```powershell
# Analyze PRX files with PRXTool
prxtool -i LIBFONT.PRX -o libfont.idc

# The PRX modules in USRDIR/MODULE/ are NOT encrypted
# (they are standard PSP PRX files for font, PSMF, etc.)
```

### Step 4: Asset Conversion (COMPLETED)
```powershell
# Convert textures to PNG
python scripts/convert_textures.py txs3_to_png -i "files/decompiled" -o "converted/textures"

# Analyze texture files
python scripts/convert_textures.py analyze -i "files/decompiled"

# Convert audio to WAV (requires ffmpeg)
python scripts/convert_audio.py convert --input "files/decompiled" --output "converted/audio"

# View 3D models with Noesis
Noesis.exe -view model.bin

# Convert audio sequences
GTSeq2Midi.exe sequence.seq
```

## Key Technical Details

### 1. **Memory Architecture**
- **User Memory**: 0x08000000-0x0A000000 (32MB)
- **Kernel Memory**: 0x88000000-0x8A000000 (mirrored)
- **VRAM**: 0x04000000-0x05000000 (2MB)
- **Scratchpad**: 0x00010000-0x00014000 (16KB fast RAM)

### 2. **Graphics Pipeline**
- **Resolution**: 480×272 (PSP native)
- **Texture Memory**: 2MB VRAM
- **Texture Formats**: RGB565, RGBA5551 (uncompressed)
- **Polygon Budget**: Estimated 10-20k polygons per frame
- **Rendering**: Forward rendering with PSP GU

### 3. **Audio System**
- **Format**: AT3 (ATRAC3) for music, SGD for sound effects
- **Channels**: Stereo
- **Sampling Rate**: 44.1kHz or 22.05kHz
- **Music**: Streamed from UMD via bgm.lib
- **Sound Effects**: Loaded into memory (gtpspsys.sgd, gtpsp_race.sgd)
- **Engine Sounds**: Per-car audio in carsound/

### 4. **Physics Engine**
- **Based on**: Gran Turismo 4/5 physics
- **Simplified**: For PSP hardware limitations
- **Car Models**: Simplified collision meshes
- **Track Data**: Reduced detail compared to console versions

### 5. **Scripting System (Adhoc)**
- **Version**: Adhoc 12 (same as GT5/GT6/GT Sport)
- **Language**: JavaScript/Python-like syntax compiled to bytecode
- **Purpose**: All game logic, UI, event handling, menus
- **Integration**: Tightly coupled with game engine via `pdiext` module
- **Decompilation**: Full disassembly possible with GTAdhocToolchain

**Adhoc Runtime Architecture**:
```
Application.adc (Entry Point)
    └── bootstrap.adc (Load bootstrap module)
        └── execBoot() → Initialize systems
    └── packed_main_loop.adc (Main game loop)
        └── bootstrap_phase2.adc (Secondary init)
            └── execBootPhase2()
    └── shutdown.adc
        └── execShutdown() → Cleanup
```

**Key Adhoc Modules**:
- `main` - Root module with global state
- `main::menu` - Menu event loop handling
- `pdiext` - Engine API (MProductInformation, etc.)
- `System` - Base system classes

### 6. **UI System**
- **Framework**: MWidget-based hierarchical UI
- **Definition Files**: `.mproject` (UI screens), `.mwidget` (prototypes)
- **Assets**: `.gpb` (graphics), `.img` (images)
- **Event Model**: Script-driven with `has_script` flag

**UI Widget Types** (from Prototypes.mwidget):
- `MComposite` - Container widget
- `MColorFace` - Solid color background
- `MImageFace` - Image display
- `MTextFace` - Text rendering
- `MHBox`/`MVBox`/`MFBox` - Layout containers
- `MScrollbar` - Scrollbar controls
- `MListBox` - List displays
- And 31 more widget types...

### 7. **Game Content**
- **Cars**: 831+ vehicles (from GENERIC_CAR.dbt)
- **Tracks**: 45 unique circuits with variants
- **Game Modes**: Arcade, GT Mode, Time Attack, Drift Trial, License Tests
- **Languages**: 9 (Japanese, English US/UK, French, German, Italian, Spanish, Chinese, Korean)

## Project Quirks and Challenges

### 1. **Proprietary Formats**
- Most file formats are custom Polyphony Digital formats
- Limited documentation available outside GT modding community
- Requires reverse engineering each format
- Some formats shared with GT5/GT6 (Adhoc, SpecDB, MWidget)

### 2. **PSP Hardware Limitations**
- **Memory**: 32MB total RAM limits asset size
- **CPU**: 333 MHz MIPS limits physics complexity
- **GPU**: Fixed pipeline with limited capabilities
- **Storage**: UMD read speed limitations (~3.6 MB/s)

### 3. **Compression Variations**
- Some files in GT.VOL are zstd compressed
- Compression indicated in files.txt metadata
- File alignment requirements (64-byte boundaries)
- `Compressed: True/False` and `ZSize` vs `Size` fields

### 4. **Localization Complexity**
- 9 language versions in same archive
- Shared assets with language-specific text
- Character encoding: UTF-8 for most, Shift-JIS for Japanese
- Separate string databases per language (*_StrDB.sdb)

### 5. **Asset Dependencies**
- Scripts reference SpecDB tables
- UI projects reference GPB texture banks
- Course data references multiple companion files (.cam, .cinf, .envptr)
- Car models have LOD variants (.00, .01, .02, etc.)

### 6. **Course ID Mapping**
- Internal course codes (c001-c114) map to readable names via courselist.xml
- Some IDs are skipped (c017, c074-c078, etc.)
- Variants stored in companion folders (c004x, c005x, etc.)

## File Purpose Reference

### Core Game Files
- `EBOOT.BIN`: Main executable (game entry point, MIPS R4000)
- `GT.VOL`: Asset archive (21,211 files, ~1GB)
- `PARAM.SFO`: PSP system parameters
- `ICON0.PNG`, `PIC1.PNG`: Game icons

### Script Files (ADC) - Complete List
**Entry Point**:
- `scripts/gt5m/Application.adc` - Main application entry

**Bootstrap Sequence**:
- `scripts/gt5m/bootstrap.adc` - Initial bootstrap
- `scripts/gt5m/bootstrap_phase2.adc` - Secondary initialization
- `scripts/gt5m/init_sound.adc` - Audio system setup
- `scripts/gt5m/packed_main_loop.adc` - Main game loop
- `scripts/gt5m/shutdown.adc` - Game shutdown

**Global State**:
- `scripts/gt5m/global_status/packed_global_status.adc` - Global game state

**Project Modules** (UI Screens):
- `projects/gt5m/arcade/arcade.adc` - Arcade mode (largest: 2.3MB diss)
- `projects/gt5m/boot/boot.adc` - Boot sequence
- `projects/gt5m/config/gt5m.adc` - Configuration
- `projects/gt5m/cursor/cursor.adc` - Cursor handling
- `projects/gt5m/dialog/dialog.adc` - Dialog system
- `projects/gt5m/gtmode/gtmode.adc` - GT Mode
- `projects/gt5m/install/install.adc` - Installation
- `projects/gt5m/manual/manual.adc` - Manual viewer
- `projects/gt5m/option/option.adc` - Options
- `projects/gt5m/play_movie/play_movie.adc` - Movie player
- `projects/gt5m/race/*.adc` - 18 race-related scripts
- `projects/gt5m/ranking/ranking.adc` - Rankings
- `projects/gt5m/ui_kit/ui_kit.adc` - UI toolkit

**Utilities**:
- `scripts/gt5m/util/*.adc` - 11 utility scripts

### Database Files (SpecDB)
**Car Data**:
- `GENERIC_CAR.dbt` - Master car table
- `VARIATION.dbt` - Color variations
- `MODEL_INFO.dbt` - Model metadata
- `MAKER.dbt` / `TUNER.dbt` - Manufacturers

**Parts Data**:
- `ENGINE.dbt`, `SUSPENSION.dbt`, `CHASSIS.dbt`
- `BRAKE.dbt`, `DRIVETRAIN.dbt`, `GEAR.dbt`
- `FRONTTIRE.dbt`, `REARTIRE.dbt`, `LSD.dbt`
- `TURBINEKIT.dbt`, `MUFFLER.dbt`, `CLUTCH.dbt`

**Game Data**:
- `COURSE.dbt` - Track database
- `RACE.dbt` - Race events
- `DEFAULT_PARTS.dbt` - Default car setups
- `ENEMY_CARS.dbt` - AI opponent cars

**Localization**:
- `CAR_NAME_*.dbt` - Car names (9 languages)
- `*_StrDB.sdb` - UI strings (9 languages)
- `UnistrDB.sdb` - Universal strings

### Asset Directories
- `car/hq/` - High-quality car models
- `car/race/` - Race-optimized car models
- `car/thumbnail/` / `thumbnail_L/` - Car preview images
- `car/info/` - Car metadata files
- `car/interior/` - Interior models
- `crs/` - Track/course data (444 files)
- `carsound/` - Per-car engine audio
- `sound_gt/se/` - Sound effects
- `sound_gt/library/` - Music library
- `textdata/gt5m/` - XML configurations
- `font/` - Font files
- `piece_gt5m/` - UI texture pieces

## Tools Integration

### Extraction Pipeline
1. **GTPSPVolTools** (v1.0+): Extract/pack GT.VOL archives
   - `unpack -i GT.VOL -o output/` - Extract all files
   - `pack -i folder/ -o GT.VOL` - Repack modified files
2. **QuickBMS**: For any additional archive formats
3. **xxd/HxD**: Binary analysis and format identification

### Analysis Pipeline
1. **Ghidra 12.0**: Binary reverse engineering (MIPS R4000)
2. **PRXTool**: PRX module analysis and IDA script generation
3. **GTAdhocToolchain 1.3.5**: ADC script disassembly
   - `adhoc.exe <file.adc>` - Disassemble to .ad.diss
   - Generates .strings file with string table
   - Supports Adhoc versions 7-12

### Asset Pipeline
1. **img-buster**: TXS3/IMG texture conversion (Python)
   - Supports RGB565, RGBA5551, RGBA8888, DXT formats
2. **TSX3Converter**: PS3/PS4 texture conversion
3. **Noesis**: Universal 3D model viewer/converter
4. **GTSeq2Midi**: Music sequence to MIDI conversion
5. **GT2TextureEditor**: GT1/2 texture editing (limited PSP use)
6. **GT3PMBDumper**: GT3 menu extraction (reference)

### Workflow Commands
```powershell
# Extract GT.VOL
& "workflow\gtpspvoltools\GTPSPVolTools.exe" unpack -i "GT.VOL" -o "extracted"

# Disassemble single ADC file
& "workflow\adhoc-toolchain\adhoc.exe" "path\to\script.adc"

# Batch disassemble all ADC files
Get-ChildItem -Path "files\decompiled" -Filter "*.adc" -Recurse | ForEach-Object {
    & "workflow\adhoc-toolchain\adhoc.exe" $_.FullName
}

# Convert texture to PNG (img-buster)
python "workflow\img-buster\img-buster.py" -i texture.img -o texture.png
```

## Asset Conversion Tools

### New Conversion Scripts
The project now includes Python scripts for converting game assets to editable formats:

#### 1. **Audio Conversion** (`scripts/convert_audio.py`)
- Converts `.at3` (ATRAC3) audio files to `.wav` format
- Analyzes audio file structure and metadata
- Supports batch conversion of all audio files

**Usage:**
```bash
# Analyze audio files
python scripts/convert_audio.py analyze --input "files/decompiled"

# Convert AT3 files to WAV
python scripts/convert_audio.py convert --input "files/decompiled" --output "converted/audio"
```

**Supported Audio Formats:**
- `.at3`: ATRAC3 music files (56 tracks, ~2-5 minutes each)
- `.sgd`: Sony sound effect banks (4 files: system, race, drift, jackpot)
- `carsound/`: Car engine audio (binary format, needs further analysis)

#### 2. **Texture Conversion** (`scripts/convert_textures.py`)
- Converts `.img` (TXS3) texture files to `.png` format
- Handles GT PSP texture formats correctly: L4, L8, RGB565, RGBA4444, RGBA5551, RGBA8888
- **707/708 textures successfully converted** (DXT1 not yet supported)
- **Interactive verification mode** (`--interactive` flag) lets user confirm each conversion
- Includes `analyze` mode for inspecting texture headers

**Usage:**
```bash
# Analyze texture files
python scripts/convert_textures.py analyze --input "files/decompiled"

# Convert all IMG files to PNG (batch)
python scripts/convert_textures.py txs3_to_png -i "files/decompiled" -o "converted/textures"

# Convert with interactive verification
python scripts/convert_textures.py txs3_to_png -i "path/to/img" -o "converted" --interactive

# Convert PNG back to TXS3
python scripts/convert_textures.py png_to_txs3 -i "converted" -o "repacked" --format RGB565
```

**TXS3 Header Structure:**
```
Offset 0x00: Magic '3SXT' (4 bytes) - little-endian on PSP, 'TXS3' big-endian
Offset 0x04: File Size (4 bytes)
Offset 0x08-0x13: Various header fields
Offset 0x14: PGLUTextureInfo Count (2 bytes)
Offset 0x16: Image Info Count (2 bytes)
Offset 0x18: PGLUTextureInfo Pointer (4 bytes)
Offset 0x1C: Image Info Pointer (4 bytes) - points to ImageInfo struct
```

**ImageInfo Structure (at img_ptr):**
```
Offset 0x00: Data Pointer (4 bytes) - absolute offset to pixel data
Offset 0x04: Data Size (4 bytes) - size of pixel data in bytes
Offset 0x08: Unknown (1 byte)
Offset 0x09: Format (1 byte) - 0x01=RGBA8888, 0x04=RGB565, 0x05=RGBA4444, 0x07=L8, 0x08=L4, 0x03=RGBA5551, 0x0A=DXT1
Offset 0x0A: Mipmap Count (1 byte)
Offset 0x0B: Unknown (1 byte)
Offset 0x0C: Width (2 bytes, uint16) - WARNING: often incorrect!
Offset 0x0E: Height (2 bytes, uint16) - WARNING: often incorrect!
```

**Key Findings & Corrections:**
- `img_ptr` = offset at bytes 28-31 of file header (absolute)
- `data_ptr` = `img_ptr + 0x24` always (i.e. right after the 32-byte ImageInfo + 4 dummy bytes)
- **Header width/height are frequently wrong** (e.g. header says 100x76 for what's actually 80x64 or 80x32). Real dimensions must be computed from `data_size / bpp`.
- **4-byte padding** (all zeros or all 0xFF) at `data_ptr` in ~60% of files. These 4 bytes are NOT pixel data.
- Common UI texture sizes: 80x64 (RGBA4444), 80x32 (RGB565), various powers of 2
- Format byte at offset 0x09 (not 0x08 as in some references)
- L4 format: nibbles are interleaved (high nibble = even pixels, low nibble = odd pixels), scaled 0-15 to 0-255

#### 3. **Batch Conversion Script** (`scripts/convert_all.bat`)
- Windows batch script for converting all assets
- Creates organized output directory structure
- Provides progress reporting

**Usage:**
```bash
scripts\convert_all.bat
```

### Conversion Workflow

#### Audio Pipeline:
1. Extract `.at3` files from GT.VOL using GTPSPVolTools
2. Convert to `.wav` using `convert_audio.py` (requires ffmpeg)
3. Edit audio in standard audio editors
4. Convert back to `.at3` (requires ATRAC3 encoder - proprietary)

#### Texture Pipeline:
1. Extract `.img` files from GT.VOL using GTPSPVolTools
2. Convert to `.png` using `convert_textures.py` (requires Pillow)
3. Edit textures in image editors
4. Convert back to `.img` using `convert_textures.py`
5. Repack into GT.VOL using GTPSPVolTools

### Technical Details

#### Audio File Analysis:
- `.at3` files are RIFF/WAVE format with ATRAC3 codec (Audio Format 0xFFFE)
- Sample rate: 44.1kHz, Channels: 2 (stereo)
- File sizes: 2-5MB per music track
- Sound effects in `.sgd` format need further reverse engineering

#### Texture File Analysis:
- Files use TXS3 format with `3SXT` magic (little-endian for PSP)
- Header parsing corrected: Format byte at ImageInfo+0x09, not 0x08
- Many UI textures use L4 format (4-bit luminance, 16 gray levels)
- L4 decodes correctly: nibbles interleaved, scaled 0-15 to 0-255
- Common PSP texture sizes: 32x32, 64x64, 128x128, 256x256
- Some non-standard sizes exist (e.g., 16x364 for tall UI elements)

#### Dependencies:
- **Python 3.x** with **Pillow** library for texture conversion
- **ffmpeg** for audio conversion (ATRAC3 decoding)
- **numpy** for efficient pixel data manipulation

### Future Work

#### Audio:
- Reverse engineer `.sgd` sound bank format
- Analyze car engine sound files in `carsound/` directory
- Create tools for modifying and repacking audio

#### Textures:
- Support DXT1/BC1 compressed textures (only 1 file: `manufacturer.img` in `tunner_logo_S`)
- Support DXT3/5 block compression formats
- Verify texture correctness by spot-checking PNG outputs
- Create GUI tool for texture browsing and conversion

#### General:
- Integrate with existing GT modding tools
- Create comprehensive documentation of all file formats
- Develop mod management and packaging tools

## References

### External Resources
- [GT Modding Hub](https://nenkai.github.io/gt-modding-hub/) - Comprehensive GT modding documentation
- [GTAdhocToolchain](https://github.com/Nenkai/GTAdhocToolchain) - Official Adhoc decompiler
- [OpenAdhoc](https://github.com/Nenkai/OpenAdhoc) - GT script recreation project (GT PSP 100% complete)
- [GTPSPVolTools](https://github.com/Nenkai/GTPSPVolTools) - GT.VOL extraction tool
- [GT Modding Discord](https://nenkai.github.io/gt-modding-hub/discord/) - Community support

### Game Identifiers
- **Title**: Gran Turismo (PSP)
- **Internal Name**: gt5m (Gran Turismo 5 Mobile)
- **Region**: EU (UCES01245)
- **Build**: JP2817
- **Adhoc Version**: 12

## Conclusion

The Gran Turismo PSP decompilation project has successfully completed multiple phases of reverse engineering:

**Completed**:
- GT.VOL extraction (21,211 files)
- ADC script disassembly (71 scripts → 7.35 MB of disassembly)
- Project structure documentation
- Tool workflow establishment
- **Asset conversion pipeline development**
- **Adhoc script analysis and documentation**

## Adhoc Script Structure and Conventions

### Overview
Gran Turismo PSP uses Adhoc Version 12 for its game logic, UI, and event handling. The scripts are compiled to `.adc` bytecode and disassembled to `.ad.diss` format for analysis.

### File Organization
- **Core Logic**: `scripts/gt5m/` (Application.adc, bootstrap.adc, main game loop)
- **UI Screens**: `projects/gt5m/` (race, detail, menu, options modules)
- **Utilities**: `scripts/gt5m/util/` (specialized helpers like SpecDatabaseUtil, SaveDataUtil)

### Code Conventions Observed in Disassembly

#### Module System
- `MODULE_DEFINE: namespace,module` - Defines modules/namespaces (similar to classes)
- Modules can contain static variables, functions, and methods
- Example: `MODULE_DEFINE: SpecDatabaseUtil`

#### Variable and Constant Types
- `VARIABLE_EVAL` - Access variable (local, static, or heap)
- `ATTRIBUTE_EVAL` - Access object property/attribute
- `STRING_CONST` - String literals
- `INT_CONST` - Integer literals
- `FLOAT_CONST` - Floating-point literals  
- `BOOL_CONST` - Boolean literals (True/False)
- `NIL_CONST` - Null/undefined value

#### Memory Management
- Stack-based operations with visible stack sizes in disassembly headers
- Local variables pushed/popped via `VARIABLE_PUSH`/`ASSIGN_POP`
- Static variables allocated per-module
- Heap allocation for object instances

#### Control Flow
- `JUMP` - Unconditional jump
- `JUMP_IF_FALSE` - Conditional branch
- `LEAVE` - Scope cleanup (function/method exit)
- `SET_STATE` - State machine transitions (EXIT, RETURN, YIELD)

#### Function Calls
- `CALL: ArgCount=N` - Invoke function/method with N arguments
- Object methods accessed via `ATTRIBUTE_EVAL` followed by `CALL`
- Static functions accessed through module paths

#### Exception Handling
- `TRY_CATCH: handler_address` - Exception handling block
- Exceptions written to `/APP_DATA_RAW/exceptions.txt` when built with `--write-exceptions-to-file`

#### UI Patterns Observed
- Widget hierarchies defined through `ELEMENT_EVAL` and `ATTRIBUTE_PUSH`
- Event callbacks: `onFocus`, `onActivate`, `set_value`, `reset_value`
- Resource cleanup patterns using watchers (`CreateWatcher`, `clear` methods)
- Localization via `translate` calls with `ATTRIBUTE_EVAL: translate`

#### Data Access Patterns
- SpecDB access through `main,gtengine,MSpecDB,*` paths
- Engine integration via `main,pdiext,*` paths
- Localization through formatted strings and unit conversions

### Key Architectural Findings
1. **Modular Design**: Game logic separated into focused modules (racing, UI, utilities)
2. **Event-Driven UI**: Heavy use of callback methods for user interactions
3. **Resource Management**: Systematic cleanup via watchers and finalize methods
4. **Localization Built-in**: String translation integrated throughout UI components
5. **State Management**: Explicit state transitions for async operations (loading, yielding)

These patterns align with the Polyphony Digital Adhoc framework used across GT4-GT Sport titles, confirming GT PSP shares the same architectural foundation as its console counterparts.

**New Asset Conversion Tools**:
- Audio conversion: `.at3` → `.wav` with metadata analysis
- Texture conversion: `.img` (TXS3) → `.png` with support for non-standard dimensions
- Batch processing scripts for automated conversion
- Round-trip testing for texture modification workflow

**Key Findings**:
- Uses Adhoc Version 12 (same as GT5/GT6)
- MWidget-based UI system with text-based definitions
- SpecDB database system with 123 tables
- 45 tracks, 831+ cars, 9 languages
- Full game logic in script layer (not hardcoded)
- **Audio**: ATRAC3 format with standard RIFF/WAVE headers
- **Textures**: Custom TXS3 format with non-standard dimensions (e.g., 16x364)

**Technical Discoveries**:
- Texture files use `3SXT` magic (little-endian TXS3)
- Many UI elements use unusual aspect ratios (tall, thin textures)
- Audio files are standard ATRAC3 but with custom GUIDs
- Asset files contain embedded metadata (e.g., filename references)

The project reveals a sophisticated game architecture with extensive scripting, database-driven design, and modular asset organization. The newly developed conversion tools enable modders to extract, edit, and repackage game assets for the first time.

## Mod Loader System

The **GT PSP Mod Loader** (`mod_loader/`) is a modular modding framework that enables dynamic modding of Gran Turismo PSP on PPSSPP without editing original game files directly.

### Architecture

```
┌────────────────────────────────────────────────────────────┐
│                    Mod Loader System                        │
├────────────────────────────────────────────────────────────┤
│  Layer 3: gtpsp-mod CLI tool (Python)                      │
│  - init / build / deploy / patch-eboot / list / info       │
│  - ISO builder (requires pycdlib)                          │
├────────────────────────────────────────────────────────────┤
│  Layer 2: Modified Core Scripts (.ad / .adc)               │
│  - patched Application.ad + packed_main_loop.ad            │
│  - ModLoader module with event-based hook registry         │
│  - 7+ event hook points (extensible)                       │
├────────────────────────────────────────────────────────────┤
│  Layer 1: EBOOT Hooks (PPSSPP cheat patches) [IN PROGRESS] │
│  - VFS redirect → load mods from ms0:/PSP/MODS/            │
│  - Extended memory / garage / performance patches          │
└────────────────────────────────────────────────────────────┘
```

The three-layer design separates concerns:
- **Layer 1** (EBOOT) handles memory-level patches — not yet complete (VFS addresses not identified)
- **Layer 2** (Core Scripts) patches the game's entry point and main loop to inject the ModLoader runtime — **complete and building**
- **Layer 3** (CLI) provides a user-friendly Python toolchain — **complete and functional**

### Directory Structure

```
mod_loader/
├── cli/                          # Python CLI tool
│   ├── gtpsp_mod.py              # Main CLI entry point (init/build/deploy/list/info/iso)
│   ├── iso_builder.py            # Full ISO build pipeline (requires pycdlib)
│   └── requirements.txt          # PyYAML, pycdlib dependencies
├── core/                         # Modified core scripts (Layer 2)
│   ├── mod_loader.ad             # Runtime event hook registry
│   ├── mod_hooks.inc             # Hook point index/documentation
│   ├── mod_sdk.inc               # Mod Development Kit (MOD_* helpers)
│   ├── Application_patched.ad    # Modified entry point
│   ├── Application_patched.adc   # Compiled output (1.2KB)
│   ├── main_loop_patched.ad      # Modified game loop with event hooks
│   ├── packed_main_loop.adc      # Compiled output (98KB)
│   └── packed_main_loop_patched.yaml  # YAML project config
├── eboot/                        # EBOOT analysis + PPSSPP patches (Layer 1)
│   ├── analysis_guide.md         # Ghidra walkthrough
│   ├── vfs_scanner.py            # Binary scanner tool
│   ├── vfs_addresses.json        # Analysis results (to be filled)
│   └── cheat_patches.ini         # PPSSPP cheat file template
├── examples/                     # Example mods (all compile successfully)
│   ├── all_cars_garage/          # Give all cars on boot
│   ├── custom_hud/               # Custom race HUD example
│   ├── infinite_garage/          # Remove garage limit
│   └── skip_intro/               # Skip intro movies
├── mod_sdk/                      # Mod development templates
│   ├── template.ad               # Empty mod template
│   └── mod_manifest.yaml         # Manifest reference
├── tests/                        # (empty — for future automated tests)
├── build_modded_core.ps1         # Build + deploy script for core scripts
├── setup.ps1                     # Environment verification script
├── README.md                     # Framework overview
└── CREATING_MODS.md              # Comprehensive mod creation guide
```

### How It Works

The mod loader patches two core game scripts at the source level:

1. **`Application_patched.ad`** (replaces original `Application.adc`):
   - Loads bootstrap (unchanged)
   - Loads `packed_main_loop` (which now includes the ModLoader module)
   - Calls `ModLoader::initialize()` and `ModLoader::scanAndLoadMods()`
   - Loads bootstrap_phase2 and shutdown normally
   - Fires `onShutdown` event before exit

2. **`main_loop_patched.ad`** (replaces original `packed_main_loop.adc`):
   - `#include`s all original game modules + `mod_loader.ad` + `mod_sdk.inc`
   - Injects event hooks at dispatch points in the MainLoop
   - No original functions are redefined — zero script conflicts

### Event System

Mods register callbacks for game events. Three hook types:

| Type | Function | Behavior |
|---|---|---|
| **Fire-and-forget** | `MOD_ON_EVENT(name, cb)` | All callbacks run, no return expected |
| **One-shot** | `MOD_ONCE(name, cb)` | Fires once, then auto-removes |
| **Query** | Return a string from callback | First non-nil return value wins |

#### Available Hook Points

| Event | Type | When it Fires |
|---|---|---|
| `onGameStart` | fire | First MainLoop iteration (boot complete) |
| `beforeMenu` | fire | Before a menu project starts |
| `afterMenu` | fire | After menu project exits |
| `beforeRace` | fire | Before a race executes |
| `afterRace` | fire | After a race ends |
| `getMenuProject` | query | To determine which menu opens |
| `onShutdown` | fire | Before game shutdown |

New hook points can be added by editing `main_loop_patched.ad` and inserting:
```c
::main::ModLoader::fireEvent("yourEvent", [arg1, arg2]);
```

### Mod Development Kit (mod_sdk.inc)

```c
#include "mod_loader/core/mod_sdk.inc"

function _mod_init()
{
    // Event helpers
    MOD_ON_EVENT("beforeMenu", function(args) { ... });
    MOD_ONCE("onGameStart", function(args) { ... });
    MOD_OFF("eventName", callback);
    MOD_FIRE("customEvent", [arg1]);

    // Player data shortcuts
    MOD_GET_CASH();
    MOD_SET_CASH(99999999);
    MOD_GET_GARAGE_SIZE();
    MOD_GIVE_ALL_CARS();

    // Debug
    MOD_LOG("message");
    MOD_SHOW_MESSAGE(context, "title", "message");
}
```

### Mod Manifest

Every mod requires a `manifest.yaml`:

```yaml
id: my_mod
version: "1.0.0"
name: "My Mod"
description: "What it does"
author: "Your Name"
scripts:
  - source: main.ad                    # .ad source relative to manifest
    target: mods/my_mod/main.adc       # .adc target in GT.VOL
assets:
  - source: assets/my_texture.img
    target: piece_gt5m/some_piece.img
eboot_patches:
  - address: "088XXXXX"
    value: "XXXXXXXX"
    comment: "What this patch does"
```

### Build Pipeline

```powershell
# Step 1: Build the modded core scripts
.\mod_loader\build_modded_core.ps1
# Produces: test_output/modded_core/scripts/gt5m/Application.adc
# Produces: test_output/modded_core/scripts/gt5m/packed_main_loop.adc

# Step 2: Create a mod project
python mod_loader/cli/gtpsp_mod.py init my_first_mod

# Step 3: Build the mod
python mod_loader/cli/gtpsp_mod.py build my_first_mod/

# Step 4: Deploy to PPSSPP
python mod_loader/cli/gtpsp_mod.py deploy my_first_mod/ --ppsspp-dir <path>

# Step 5: Build a modded ISO (requires pycdlib)
python mod_loader/cli/gtpsp_mod.py iso create "original.iso" -i
```

### Current Build Status

| Component | Status | Verified |
|---|---|---|
| `Application_patched.ad` → `.adc` | ✅ Building | 1,284 bytes |
| `main_loop_patched.ad` → `packed_main_loop.adc` | ✅ Building | 98,075 bytes |
| `all_cars_garage` example | ✅ Building | 3,178 bytes |
| `custom_hud` example | ✅ Building | |
| `infinite_garage` example | ✅ Building | |
| `skip_intro` example | ✅ Building | |
| `gtpsp_mod.py` CLI | ✅ All commands functional | init/build/deploy/list/info |
| `build_modded_core.ps1` | ✅ Build + deploy | |
| `setup.ps1` | ✅ Environment checker | |
| ISO Builder (`iso_builder.py`) | ✅ Code present | Requires `pycdlib` |
| EBOOT VFS analysis (`vfs_addresses.json`) | 🔄 In progress | Address fields not yet filled |
| `tests/` directory | ❌ Empty | No automated tests yet |

### PPSSPP Setup

1. Enable file replacement: **Settings → Tools → Developer Tools → Install Device to insta.. → ON**
2. Deploy modded core + mods to: `<memstick>/PSP/UMD0/PSP_GAME/USRDIR/GT.VOL/`
3. For cheat patches: **Settings → System → Enable Cheats → ON**
   Place patches at: `<memstick>/PSP/CHEATS/UCES01245.ini`
4. Restart the game

### Mod Types Supported

| Mod Type | Approach | Example |
|---|---|---|
| **Gameplay tweaks** | Event hooks + API calls | Give all cars, max cash |
| **Save data mods** | `GlobalStatus::checkout()` / `checkin()` | Modify garage, licenses |
| **Texture replacement** | Asset replacement in manifest | UI textures, car skins |
| **Audio replacement** | Asset replacement + conversion | Music (ATRAC3), sound effects |
| **XML config mods** | Asset replacement | carlist.xml, courselist.xml |
| **Car mods** | Asset replacement + SpecDB data | New cars, modified stats |
| **UI mods** | Script hooks + project replacement | Custom HUD, new screens |
| **Track/course mods** | Asset replacement | Custom tracks (format-dependent) |
| **EBOOT patches** | PPSSPP cheat patches (future) | VFS redirect, garage limits |

### EBOOT Analysis Status (Layer 1)

- [x] EBOOT decrypted via PPSSPP runtime memory dump (WebSocket API)
- [x] Decrypted binary imported into Ghidra (project `gt_psp_decrypted`, MIPS:LE:32:default)
- [x] Key strings found: PDIAPP module, GT.VOL, .adc, scripts/, projects/, gt5m, SpecDB, pdiext
- [x] sceIoOpen stub identified at `0x08C7A870` (zz_sceIoOpen)
- [x] All 36 I/O syscall stubs mapped (see `vfs_addresses.json`)
- [ ] VFS load function address — requires runtime analysis (see below)
- [ ] Memory stick redirect patch — requires VFS function address
- [ ] Heap/garage EBOOT patches — handled via script layer (mod_loader)
- [ ] Cheat patches tested in PPSSPP

**Runtime VFS Analysis**: Run `node mod_loader/eboot/vfs_analyzer.js` with PPSSPP running.
Navigating the game menus triggers `.adc` script loads; the caller address captured at
the `zz_sceIoOpen` breakpoint is the VFS load function. The `ra` register gives the
calling function address directly.

See `mod_loader/eboot/vfs_addresses.json` for complete findings.

### Mod Creation Guide

A comprehensive guide (`mod_loader/CREATING_MODS.md`) covers:

- **Adhoc language primer** — syntax, modules, maps, gotchas
- **Complete hook API reference** — all events with examples
- **Mod types in detail**: car mods (file formats, SpecDB access), UI mods (MWidget system), save data mods, sound/audio mods, texture replacement, track/course mods
- **Custom UI projects** — working with `.mproject` files and widget types
- **EBOOT patches** — PPSSPP cheat format, Ghidra workflow
- **ISO building** — full pipeline from original ISO to modded output
- **Testing & debugging** — MOD_LOG, MOD_SHOW_MESSAGE, common issues
- **SpecDB table reference** — all 25+ database tables with key fields
- **Game namespace reference** — important module paths and VFS paths
- **Troubleshooting** — build failures, runtime issues, PPSSPP setup

## Native PC Port (adhoc-vm)

A Rust-based native PC port at `pc_port/`. Custom Adhoc VM + SDL2 + SpecDB. **Main game loop runs natively in Rust**, bypassing `packed_main_loop.adc` bytecode interpretation. Project scripts (arcade, race, etc.) still execute through the VM.

### Architecture (Updated 2026-04-27)

```
bootstrap.adc [VM] → execBoot() → init modules, SpecDB, menu system
packed_main_loop.adc [VM] → registers modules only (GameSequence, GamePlan, etc.)
bootstrap_phase2.adc [VM] → execBootPhase2() → init organizer, race operator, sound
native MainLoopState::tick() [NATIVE] → 12-phase state machine
  ├── MENU: MGOM.start("arcade") → loads arcade.mproject + arcade.adc [VM]
  │      MGOM.sync() → waits for user input → page transitions
  └── RACE: executeRace() stub
```

### Build & Usage

```bash
cargo build
cargo run -- --boot           # Native main loop + SDL window + UI
cargo run -- --headless-boot  # Old VM-driven Application.adc path
cargo run -- --test-all       # 18/18 .adc files parse correctly
```

### Completed (2026-04-29)

| Feature | Status | Details |
|---------|--------|---------|
| Native main loop | ✅ | 12-phase tick replaces VM MainLoop bytecode |
| Input wiring | ✅ | KEY_STATE atomics, edge detection, 12 PSP buttons |
| GameSequence state | ✅ | Shared module-level OnceLock with correct enums |
| call_value perf | ✅ | Arc<CodeFrame> + child_frame_index O(1) lookup |
| **IteratorNext opcode** | ✅ **NEW** | Foreach loop support — fixes `execBoot()` infinite loop |
| **Bootstrap Execution** | ✅ **NEW** | `execBoot()` and `execBootPhase2()` now execute via VM |
| **MOrganizer natives** | ✅ **NEW** | Game mode organizer: init/start/stop/isRunning |
| **MRaceOperator natives** | ✅ **NEW** | Race operator: init/start/stop/isRunning |
| **MSound natives** | ✅ **NEW** | Sound system: init/playBGM/stopBGM/playSE |
| **Menu UI Focus** | ✅ **NEW** | Widget focus navigation (UP/DOWN/LEFT/RIGHT) + activation |
| Bootstrap | ✅ | MenuClassDefine, config, init_sound loaded |
| 9-slice FrameImageFace | ✅ | draw_texture_region() sub-rect blit |
| Font caching | ✅ | OnceLock<FontArc> |
| Page navigation | ✅ | go_to_page() with widget→page routing map |
| Page-based rendering | ✅ | Active page shown, utilities always visible |
| Page init callbacks | ✅ | onLoad/onInitialize called after project start |
| ListBox/OptionMenu/SceneFace/ProgressFace/Scrollbar | ✅ | Visible placeholders |
| Actor animations wired | ✅ | get_actor_float() → widget properties |
| VaCall / StringPush / ArrayPush | ✅ | Properly decoded |
| Text alignment | ✅ | draw_text_align (left/center) |
| Script callback wiring | ✅ | Widget events → VM function lookup |
| Race gameplay engine | ✅ | new `race.rs`: physics, 3D track, car, chase camera |
| Course loading | ✅ | .ad course parser → TrackState |
| Car model loading | ✅ | 3LDM model parser (proper ModelSet3 format) → CarModel |
| 3D projection | ✅ | Mat3 perspective + look_at for chase cam |
| 3LDM parser fix | ⚠️ Partial | Header parsing implemented; GT PSP files have null mesh pointers (0x38=0), requires format variant handling |

### Remaining

| Priority | Task | Notes |
|----------|------|-------|
| P2 | Race: car 3D model rendering | Placeholder triangle, needs CarModel mesh render |
| P2 | Race: filled track triangles | Currently wireframe only |
| P2 | Race: lap detection | Distance-based, no actual start/finish line |
| P2 | Race: track from race.mdl | Uses fallback procedural, need proper mdl loading |
| P2 | Textures from race.txs | TXS3 loading not wired |
| P3 | 3D rendering pipeline | Depth buffering, texture mapping |
| P3 | Vehicle physics | MRaceOperator per-frame ticks |
| P3 | Engine audio | MEngineSound.loadPreset stubs |
| P3 | Save/load | MSaveDataUtilPSP, GlobalStatus stubs |

### Key Source Files

```
pc_port/src/
├── main.rs              # CLI, --boot, --headless-boot, --game, --test-all
├── vm/
│   ├── engine.rs        # VM: execute_frame, exec_insn (60+ ops), call_value
│   ├── loader.rs        # .adc parser: V12 headers, varint, symbol table
│   ├── decoder.rs       # Opcode + Instruction enums
│   ├── native.rs        # NativeRegistry with fallback prefixes
│   ├── value.rs         # Value enum, FunctionValue (with static_base)
│   ├── frame.rs         # Frame struct (stack, locals, static_base, def_counter)
│   ├── module.rs        # ModuleRegistry, CodeFrame storage
│   └── storage.rs       # LocalStorage, StaticStorage, StorageKind
├── engine/
│   ├── ui.rs            # MProject parser, widget rendering, animation, UiManager
│   ├── main_loop.rs     # Native main loop: 12-phase state machine bypasses VM
│   ├── race.rs          # Race gameplay: physics, 3D rendering, chase camera
│   ├── menu.rs          # MGOM, GameSequence state, input, widget stubs
│   ├── gtengine.rs      # SpecDB queries, MOrganizer, MRaceOperator
│   ├── pdiapp.rs        # SDL2 rendering, MTexture, MRender, MInput, MDEBUG
│   ├── pdistd.rs        # Module::load, MRandom, MTime, MFile
│   ├── pdiext.rs        # Save data, STStructure, ByteData, fonts
│   ├── graphics.rs      # SDL2 renderer: window, text, wireframe, textures
│   ├── audio.rs         # SDL2_mixer: .at3 → ffmpeg → PCM → play
│   ├── model.rs         # 3LDM car, .ad course, .cam camera parsers
│   ├── sprite.rs        # TXS3/IMG texture decoder + LRU cache
│   └── specdb.rs        # .dbt/.idi parser, bit-packed fields, UTF-16LE strings
```

### Reference Repos

```
workflow/
├── GTAdhocToolchain-master/  # C# toolchain — V12 format reference
│   └── GTAdhocToolchain.Core/
│       ├── Instructions/      # InsVariablePush.cs, InsVariableEvaluation.cs (IsStatic = symbols.Count > 1)
│       ├── AdhocCodeFrame.cs  # Split-stack serialization (V11+)
│       ├── AdhocVersion.cs    # Feature flags (UsesNewSplitStack)
│       └── AdhocStream.cs     # DecodeBitsAndAdvance varint
└── GTSpecDB/                  # SpecDB binary format reference
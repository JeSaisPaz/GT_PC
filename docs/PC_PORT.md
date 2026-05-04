# Gran Turismo PSP — Native PC Port

## Goal

Run Gran Turismo PSP natively on Windows without PPSSPP emulation by reimplementing the
Adhoc bytecode interpreter and the game's native engine APIs, using the 100% recovered
script sources from OpenAdhoc and the decrypted EBOOT as reference.

## Architecture

```
┌──────────────────────────────────────────────────────────┐
│                   Game Scripts (.ad/.adc)                  │
│  163 source files: bootstrap, arcade, race, gtmode, etc  │
└────────────────────────┬─────────────────────────────────┘
                         │ loads + executes
┌────────────────────────▼─────────────────────────────────┐
│                  Adhoc Bytecode VM (Rust)                  │
│  ┌──────────┐  ┌──────────┐  ┌──────────┐  ┌──────────┐ │
│  │  Loader  │  │  Engine  │  │  Types   │  │  Native  │ │
│  │  /Parser │──▶  /Eval   │──▶  /GC     │──▶  Bridge  │ │
│  └──────────┘  └──────────┘  └──────────┘  └────┬─────┘ │
└──────────────────────────────────────────────────┼───────┘
                                                   │ calls
┌──────────────────────────────────────────────────▼───────┐
│               Native Engine API Modules                    │
│  ┌────────┐ ┌────────┐ ┌──────────┐ ┌──────┐ ┌────────┐ │
│  │ pdistd │ │ pdiext │ │ gtengine │ │menu  │ │ pdiapp │ │
│  │ (std)  │ │ (ext)  │ │ (SpecDB) │ │(UI)  │ │(app)   │ │
│  └────┬───┘ └────┬───┘ └────┬─────┘ └──┬───┘ └───┬────┘ │
└───────┼──────────┼──────────┼───────────┼──────────┼──────┘
        │          │          │           │          │
┌───────▼──────────▼──────────▼───────────▼──────────▼──────┐
│              Platform Backend (SDL2/OpenGL/Audio)          │
│  ┌──────────┐  ┌──────────┐  ┌──────────┐  ┌──────────┐  │
│  │ Renderer │  │  Audio   │  │  Input   │  │  Window  │  │
│  │ (OpenGL) │  │ (SDL)    │  │ (SDL)    │  │ (SDL)    │  │
│  └──────────┘  └──────────┘  └──────────┘  └──────────┘  │
└──────────────────────────────────────────────────────────┘
```

## Development Plan

### Phase 1: Adhoc Bytecode VM (Foundation)

The core interpreter that loads .adc files and executes Adhoc bytecode. Everything else
builds on this.

**Estimated effort:** ~2-3 weeks (full-time)

| Task | Description | Dependencies |
|------|-------------|--------------|
| **1.1** | Rust project scaffolding | None |
| **1.2** | `.adc` file loader — read header, symbol table, code frames | None |
| **1.3** | Instruction decoder — parse all 70+ opcodes from byte stream | 1.2 |
| **1.4** | Value type system — Int, Float, Double, Bool, String, Nil, Void, Array, Map, Object, FunctionRef | 1.2 |
| **1.5** | Stack machine — operand stack with push/pop/eval | 1.3 |
| **1.6** | Variable storage — local frame + module-static storage | 1.4 |
| **1.7** | Control flow — jumps, conditional jumps, try/catch, leave | 1.5 |
| **1.8** | Module/class/function system — MODULE_DEFINE, CLASS_DEFINE, FUNCTION_DEFINE, METHOD_DEFINE | 1.5 |
| **1.9** | Calling convention — CALL, VA_CALL, method dispatch, `self` binding | 1.6 |
| **1.10** | Debug CLI — load and step through .adc files, dump state | 1.2 |
| **1.11** | Test with single script — load `Application.adc`, verify module/function structure | 1.8 |

### Phase 2: Native Engine Bridge

Expose the game's native module APIs to the Adhoc VM via a Foreign Function Interface (FFI).

**Estimated effort:** ~4-6 weeks (full-time)

#### Subphase 2A: pdistd (Standard Library) ~200 functions
| Task | Functions | Notes |
|------|-----------|-------|
| **2.1** | RNG (`MRandom`) | Mersenne Twister or similar |
| **2.2** | String utils (`AsciiStringHash`, format) | |
| **2.3** | File I/O (`MFile`, `ReadFile`, `WriteFile`) | Read from extracted GT.VOL |
| **2.4** | XML parsing (`MXml`, `MDomNode`) | Game configs in textdata/ |
| **2.5** | Compression (`Deflate`, `Inflate`) | zlib binding |
| **2.6** | Base64, Crypto (`MEncryption`, `MCipher`) | Save data encryption |
| **2.7** | Time (`MTime`) | |
| **2.8** | Math (`MFloat`, `MVector`) | |
| **2.9** | Dynamic resources (`MDynRes`) | |
| **2.10** | Formatting (`MMisc::GetMoneyString`, units) | |

#### Subphase 2B: pdiext (Extended Library) ~223 functions
| Task | Functions | Notes |
|------|-----------|-------|
| **2.11** | Product info (`MProductInformation`) | App metadata strings |
| **2.12** | Font rendering (`LoadLatinFont`) | Load from font/ directory |
| **2.13** | BGM system (`MSystemBGM`) | Audio playback |
| **2.14** | Sound effects (`MEngineSound`, `MSoundContext`) | |
| **2.15** | Save data (`MSaveDataUtilPSP`) | Player progress persistence |
| **2.16** | Unit conversion (`MUnit`) | mph/kph, hp/kW, etc |
| **2.17** | Input (`SuperPortButtonBit`, `SuperPortAnalogChannel`) | |
| **2.18** | Voucher/DLC (`MVoucher`) | |
| **2.19** | USB comm (`MUsbPspCommPSP`) | Stub (GT5 link feature) |

#### Subphase 2C: gtengine (Game Engine) ~694 functions — largest module
| Task | Functions | Notes |
|------|-----------|-------|
| **2.20** | SpecDB reader | Load and query .dbt/.idi files |
| **2.21** | Car data API (`getCarCode`, `getCarName`, etc) | |
| **2.22** | Course/track API (`getCourseCode`, `getCourseCondition`) | |
| **2.23** | Race parameter API (`MRaceParameter`, `MCarParameter`) | |
| **2.24** | AI enemy system (`EnemySetUtil`) | |
| **2.25** | Replay system (`MReplayInfo`) | |
| **2.26** | All enum definitions | RaceType, StartType, etc |

#### Subphase 2D: pdiapp + menu (Application + UI)
| Task | Functions | Notes |
|------|-----------|-------|
| **2.27** | Game records (`CreateGameRecordStructure`) | |
| **2.28** | XML utilities (`XmlUtil`) | |
| **2.29** | MWidget base (`MMenuGameObjectManager`, `MRootTransition`) | UI widget tree |
| **2.30** | Widget event system (`MFunctionEvent`, `MScriptEvent`, `MActivateEvent`) | |
| **2.31** | Animation actors (`MMoveActor`, `MFadeActor`, `MInterpolator`) | |
| **2.32** | Watcher system (`MScriptWatcher`) | Async timer/ticker |
| **2.33** | Scrollbar/listbox (`MAdjustment`, `MListBox`) | |

### Phase 3: Graphics, Audio & Input

Replace PSP hardware layer with PC equivalents.

**Estimated effort:** ~3-4 weeks (full-time)

| Task | Description |
|------|-------------|
| **3.1** | Window + event loop (SDL2) |
| **3.2** | OpenGL renderer — framebuffer, basic 2D |
| **3.3** | Texture loader — read PSP .img (TXS3) format, upload to GL |
| **3.4** | GPB texture bank loader — UI texture atlases |
| **3.5** | Font renderer — render Latin/Japanese text |
| **3.6** | 3D model loader — car/track .mdl format |
| **3.7** | Scene graph + camera system |
| **3.8** | Audio playback — WAV/ATRAC3 via SDL_mixer or miniaudio |
| **3.9** | BGM streaming — playlist management |
| **3.10** | Controller/keyboard input mapping |

### Phase 4: Game Integration

Wire everything together to produce a playable game.

**Estimated effort:** ~4-6 weeks (full-time)

| Task | Description |
|------|-------------|
| **4.1** | Boot sequence — load Application.adc → bootstrap → init_sound |
| **4.2** | Main loop — event dispatch, frame timing |
| **4.3** | Menu screens — Boot → Title → Arcade Mode selection |
| **4.4** | Car selection screen — Arcade mode car picker |
| **4.5** | Track selection — load course data |
| **4.6** | Race mode — basic race loop (time attack) |
| **4.7** | AI opponents — enemy car selection and behavior |
| **4.8** | HUD rendering — speed, position, lap time |
| **4.9** | Save data — player progress persistence |
| **4.10** | Full game modes — Arcade, Time Attack, Drift Trial, GT Mode |

## Codebase Structure

```
pc_port/
├── Cargo.toml                    # Rust project
├── src/
│   ├── main.rs                   # CLI entry point
│   ├── vm/
│   │   ├── mod.rs
│   │   ├── loader.rs             # .adc bytecode reader
│   │   ├── decoder.rs            # Instruction decoder
│   │   ├── value.rs              # Adhoc value type system
│   │   ├── engine.rs             # Stack machine executor
│   │   ├── storage.rs            # Variable storage (local + static)
│   │   ├── frame.rs              # Call frames
│   │   ├── module.rs             # Module/class/function definitions
│   │   └── native.rs             # Native function bridge
│   ├── engine/
│   │   ├── mod.rs
│   │   ├── pdistd.rs             # Standard library stubs
│   │   ├── pdiext.rs             # Extended library stubs
│   │   ├── gtengine.rs           # Game engine stubs
│   │   ├── menu.rs               # Menu/UI stubs
│   │   ├── pdiapp.rs             # Application stubs
│   │   └── specdb.rs             # SpecDB reader
│   └── platform/
│       ├── mod.rs
│       ├── window.rs             # SDL2 window
│       ├── render.rs             # OpenGL renderer
│       ├── audio.rs              # Audio playback
│       ├── input.rs              # Input handling
│       └── assets.rs             # Asset loading
└── tests/
    ├── loader_test.rs
    ├── engine_test.rs
    └── scripts/                   # Test .adc scripts
```

## Key Design Decisions

1. **Language: Rust** — Safe, fast, pattern-matching for opcodes, cargo available
2. **VM type: Stack-based** (matches Adhoc bytecode exactly — no translation)
3. **Value type: tagged union** (`enum AdhocValue { Int(i32), Float(f32), ... }`)
4. **Variable storage: indexable arrays** (not hash maps — bytecode uses numeric indices)
5. **Native bridge: registered function table** — engine modules register function pointers by module path
6. **SpecDB: memory-mapped** — load .dbt/.idi files into lookup tables at startup
7. **Assets: on-demand loading** — load from extracted GT.VOL directory by path
8. **Renderer: OpenGL 3.3+** — wide compatibility, sufficient for PSP-era graphics

## Known Issues

### Rendering: Red screen with overlapping text (2025-04-29)

**Symptom:** Race mode shows completely red screen with white/yellow text overlapping
**Root causes (suspected):**
1. Course texture loading incorrectly — log shows "8x512" pixels (should be full-track texture)
2. Track triangles may be rendering solid red instead of proper track geometry fill
3. OpenGL not initialized — `init_opengl()` is commented out in main.rs line 493
4. Screen clear may not be working in SDL2 rendering path

**Files involved:**
- `pc_port/src/engine/race.rs` — `render()`, `render_sdl2()`, `fill_triangle()`
- `pc_port/src/engine/graphics.rs` — `clear()`, `draw_text_align()`, OpenGL functions
- `pc_port/src/main.rs` — init_renderer() at line 490, init_opengl at line 493 (commented out)
- `pc_port/src/engine/model.rs` — `load_course_texture()` at line 615 (loads race.txs)

**Potential fixes:**
1. Enable OpenGL: uncomment line 493 in main.rs to call `init_opengl(960, 544)`
2. Add debug logging in `render_sdl2()` to output triangle count and coordinates
3. Fix course texture loading — verify race.txs dimensions or use fallback
4. Check fill_triangle algorithm for inverted rendering (could be filling wrong direction)

## References

- **GTAdhocToolchain**: `workflow/adhoc-toolchain/` — compiler, disassembler, opcode definitions
- **OpenAdhoc source**: `source/` — 163 recovered .ad scripts
- **OpenAdhoc repo**: `openadhoc_repo/` — upstream reference
- **Decrypted EBOOT**: `test_output/decrypted/decrypted_eboot.bin` — native engine reference
- **PPSSPP WebSocket API**: `mod_loader/eboot/` — runtime debugging tools
- **Game assets**: `files/decompiled/Gran Turismo/PSP_GAME/USRDIR/GT.VOL/` — 21,211 files
- **SpecDB**: `files/decompiled/Gran Turismo/PSP_GAME/USRDIR/GT.VOL/specdb/` — 123 tables
- **Adhoc language spec**: `workflow/adhoc-toolchain/LANGUAGE_SPECIFICATION.md`
- **GTAdhocToolchain source**: https://github.com/Nenkai/GTAdhocToolchain
- **OpenAdhoc source**: https://github.com/Nenkai/OpenAdhoc

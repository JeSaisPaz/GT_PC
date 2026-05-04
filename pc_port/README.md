# adhoc-vm — Gran Turismo PSP Native PC Port

**3,139 lines of Rust** — Adhoc bytecode VM + engine API stubs + SDL2 graphics backend.

## Build & Run

```powershell
cargo run --release -- --boot          # Graphical boot sequence
cargo run --release -- --dump file.adc  # Dump .adc structure
cargo run --release -- --specdb         # Test SpecDB loading
cargo run --release -- --list-native    # List 200+ registered native stubs
cargo run --release -- --trace file.adc # Execute with instruction trace
```

## Architecture

```
pc_port/src/
├── main.rs              # CLI entry point, 380+ native stub registrations
├── vm/                  # Adhoc bytecode VM (1,624 lines)
│   ├── value.rs         # Tagged-union value type (19 variants)
│   ├── decoder.rs       # Opcode enum (72 opcodes v5–v12)
│   ├── loader.rs        # Binary .adc parser (LEB128 symbol table, nested frames)
│   ├── engine.rs        # Stack-based execution engine with trace/log
│   ├── module.rs        # Module/class/function registries
│   ├── frame.rs         # Call frame (stack, locals, try-catch, IP)
│   ├── storage.rs       # Local + static variable storage
│   └── native.rs        # Native function FFI bridge
├── engine/              # Native engine API modules (441 lines)
│   ├── specdb.rs        # SpecDB .dbt/.idi binary parser
│   ├── gtengine.rs      # gtengine stubs (MSpecDB, MRaceParameter, etc.)
│   └── menu.rs          # MWidget UI stubs (20+ classes, 30+ button inputs)
└── platform/            # Graphics backend (534 lines)
    ├── window.rs        # SDL2 window with event loop
    ├── render.rs        # 2D canvas renderer (fill_rect, draw_rgba, draw_texture)
    ├── texture.rs       # PSP TXS3/IMG texture decoder (RGB565, RGBA4444, L4, L8)
    ├── font.rs          # ab_glyph font renderer (Windows Segoe UI)
    └── boot.rs          # Boot screen + menu sequence
```

## Phase 1: Adhoc Bytecode VM ✓

| Component | Status | Lines |
|-----------|--------|-------|
| `.adc` loader (header, symbol table, code frames) | ✅ | 431 |
| Opcode decoder (72 opcodes v5–v12) | ✅ | 255 |
| Stack-based execution engine | ✅ | 689 |
| Value type system (19 types) | ✅ | 109 |
| Local/static variable storage | ✅ | 49 |
| Call frame management | ✅ | 43 |
| Module/class/function definitions | ✅ | 60 |
| Native function FFI bridge | ✅ | 29 |

**Key engine features:**
- Instruction tracing (`--trace`) — prints every instruction with source line
- Native call logging (`--log-native`) — logs all FFI calls
- Stack dump (`dump_stack()`) — prints call frames on crash
- Safety limit (10M instruction cap, overflow-safe buffers)
- Try-catch with error string propagation
- String method dispatch (12 methods: length, split, trim, indexOf, etc.)
- Array method dispatch (5 methods: push, pop, clear, length)

## Phase 2: Native Engine API Stubs ✓

| Module | Calls | Status |
|--------|-------|--------|
| `pdistd` (standard library) | ~200 | ✅ MRandom, MTime, MLocale, MFile, MXml, etc. |
| `pdiext` (extended library) | ~223 | ✅ MProductInformation, MSystemBGM (8 methods), MEngineSound, MSaveDataUtil, MMisc |
| `gtengine` (game engine) | ~694 | ⚠️ SpecDB parser + 25 stub APIs |
| `menu` (UI framework) | ~151 | ✅ 20+ MWidget classes (MMenuGameObjectManager, MRootTransition, MMoveActor, etc.) |
| `pdiapp` (application) | ~8 | ✅ Game records, XmlUtil |
| `GlobalStatus`, `GameSequence`, `BranchStatus`, `DebugTool` | Various | ✅ Full boot path stubs |

**SpecDB reader** (`specdb.rs`):
- Parses `GTDB`/`GTID` binary format (based on Nenkai's 010 Editor templates)
- Loads all 45+ game tables: GENERIC_CAR (837 rows), ENGINE, VARIATION (6042 rows), COURSE (85 rows), RACE (5 rows)
- IDI index file support for row ID lookup
- UTF-16LE string decoding
- Auto-clamps column count to fit file size (fixes malformed RACE table)

## Phase 3: Graphics Backend ✓

| Component | Status | Lines |
|-----------|--------|-------|
| SDL2 window + event loop | ✅ | 58 |
| 2D canvas renderer | ✅ | 61 |
| PSP TXS3/IMG texture decoder | ✅ | 192 |
| ab_glyph font rendering | ✅ | 67 |
| Boot screen + menu sequence | ✅ | 150 |

**Texture format support:** RGB565, RGBA4444, RGBA5551, RGBA8888, L8, L4
**Auto-dimension correction:** PSP headers often report wrong w/h; decoder computes from data_size/bpp
**Font:** Loads Segoe UI / Arial from Windows system fonts, rasterizes via ab_glyph

## Running

```powershell
cargo run --release -- --boot
```

Shows:
1. **Boot screen** (0–3s): Black → white fade-in bars (simulating PDIPROG logo)
2. **Menu screen** (3s+): Dark GT-style background, "GRAN TURISMO" title, 5 menu items (ARCADE MODE, TIME ATTACK, etc.)
3. **Texture**: Loads `tunner_logo_S/polyphony.img` — manufacturer logo
4. **HUD**: Frame counter bottom-left

Press **Escape** or close window to exit.

## Debug API

The PC port includes a debug API accessible via the `handle_debug_command` function:

| Command | Description |
|---------|-------------|
| `trace` | Toggle instruction tracing |
| `log-native` | Toggle native call logging |
| `call <path>` | Call a native function |
| `list-native` | List all native functions |
| `stack` | Dump call stack |
| `help` | Show help |
| `quit` | Exit |

## Dependencies

- `sdl2` (bundled, statically linked) — window, input, canvas
- `gl` — OpenGL bindings (reserved for future)
- `ab_glyph` — Pure Rust font rasterization
- `rand` — Random number generation

## Remaining Work

- [ ] Wire full VM boot: load `Application.adc` → `bootstrap.adc` → `packed_main_loop.adc` with script execution
- [ ] Load and render actual GPB texture atlases from `projects/`
- [ ] Implement `MImageFace`/`MTextFace`/`MColorFace` native stubs that call renderer
- [ ] MWidget scene graph: widget tree → render passes
- [ ] Audio: SDL2 audio for BGM + SFX playback
- [ ] Input: map keyboard/controller to PSP button events
- [ ] 3D rendering: car models, track data
- [ ] Full race mode: physics, AI, HUD

---

*Built as part of the [GTPSP-decompile](https://github.com/anomalyco/GTPSP-decompile) project.*

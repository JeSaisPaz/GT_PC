# GT PSP PC Port — Project Status (2026-04-28)

## Goal
Native PC port of GT PSP using custom Adhoc VM + SDL2 + SpecDB. Main game loop runs natively in Rust. Project scripts still execute through the VM.

## Build & Usage
```bash
cargo build --release
cargo run --release -- --boot   # Native main loop + SDL window + race
cargo run --release -- --test-3ldm  # 3LDM parser test harness
```

## Architecture

```
bootstrap.adc [VM] → execBoot()        (skipped — infinite VM loop)
packed_main_loop.adc [VM]             → registers modules (163 globals)
bootstrap_phase2.adc [VM] → execBootPhase2()  (skipped)
MenuClassDefine.adc [VM]
config/gt5m.adc [VM]
init_sound.adc [VM]
MainLoopState::tick() [NATIVE] — 12-phase state machine
  ├── CheckConditions → SEQUENCE_RACE=4 → RaceBGM → … → RaceExecute → RaceRunning
  └── Native race physics + 3D rendering (no VM bytecode for gameplay)
```

## Current State — What Works

### Core Loop
| Component | Status | Notes |
|-----------|--------|-------|
| Bootstrap | ✅ | All 6 .adc scripts load (frame idx 0-6) |
| execBoot / execBootPhase2 | ⚠️ Skipped | VM infinite loop in initArgs, but game works anyway |
| Main loop state machine | ✅ | 12-phase tick, CheckConditions → Race* → ClearFontCache |
| Shared sequence state | ✅ | OnceLock<Mutex<SeqState>>, SEQUENCE_RACE=4 |
| FPS | ✅ | 44-52 FPS with wireframe rendering |

### Race Gameplay (src/engine/race.rs)
| Feature | Status | Notes |
|---------|--------|-------|
| Car physics | ✅ | Acceleration (35), brake (60), speed-dependent steering, drag |
| **Car model loading** | ✅ | GT PSP 3LDM variant: 311v/592tri via all-runs heuristic scanner |
| Car spawn position | ✅ | Extracted from c001.ad metadata records (type-01 entries) |
| **Course loading** | ✅ | race.mdl → 199v/28tri real mesh (near-LOD), procedural fallback |
| Procedural track | ✅ | Rolling hills with sine/cosine heightfield (fallback only) |
| Track surface following | ✅ | Grid-accelerated nearest-triangle height lookup |
| Off-track detection | ✅ | Closest triangle distance test with heavy speed penalty |
| Lap detection | ✅ | Enter/Exit/Re-enter state machine, spawn-safe (starts in_zone=true) |
| Auto-throttle | ✅ | Temporary — car drives at max speed for testing |
| Race finish & restart | ✅ | After 3 laps → RaceEndReplay → re-initialize |

### 3D Rendering (src/engine/race.rs + src/engine/graphics.rs)
| Feature | Status | Notes |
|---------|--------|-------|
| Mat4::perspective | ✅ | Fixed row-major layout (swapped -1 and 2fn*nf) |
| Mat4::look_at | ✅ | View matrix follows car from behind |
| NDC → screen mapping | ✅ | (x+1)*half_w, height-(y+1)*half_h |
| Triangle wireframe | ✅ | 3-pixel-thick SDL2 draw_line edges |
| Triangle scanline fill | ❌ Broken | Fill code exists (`fill_triangle`) but returns 0 drawn |
| Camera update | ✅ | Per-frame look_at from behind car |
| HUD | ✅ | Speed km/h, lap counter, timer, on/off-track status |
| Dark blue background | ✅ | fill_rect(0,0,960,544,10,10,40) |
| Start/finish line | ✅ | Yellow vertical line at spawn position |
| **Car model rendering** | ✅ | **311v/592tri** wireframe in red (was 28v/13tri) |
| Track rendering | ✅ | race.mdl 199v/28tri wireframe in green/orange |

### Texture Loading (src/engine/sprite.rs + src/engine/model.rs)
| Feature | Status | Notes |
|---------|--------|-------|
| **Course texture (race.txs)** | ✅ | 16×256 RGBA, standalone 3SXT format |
| **Car texture (embedded TXS3)** | ✅ | 16×28 RGBA, extracted from 3LDM texture_set_ptr (0x48) |
| **UV coordinate extraction** | ✅ | FVF semantic 3 (UV/map) parsed from 3LDM |
| **OpenGL texture upload** | ✅ | GPU texture creation + caching |
| **Textured rendering** | ✅ | Shaders with UV attributes, texture sampling |
| Sprite loading (.img) | ⚠️ Stub | TXS3 parser exists for piece_gt5m/ |

### SDL2 / Graphics Backend (src/engine/graphics.rs)
| Feature | Status | Notes |
|---------|--------|-------|
| SDL2 window (960×544) | ✅ | Accelerated + vsync canvas |
| Thread-local renderer | ✅ | Rc<RefCell<>> with lazy headless auto-init |
| Input mapping | ✅ | 12 PSP buttons → Scancode (WASD, arrows, shift, return) |
| ab_glyph font rendering | ✅ | arial.ttf, draw_text_align with left/center/right |
| Texture cache | ✅ | LoadedTexture storage, draw_texture_region |
| ProjectorRef | ✅ | Project/project_and_fill_triangles (wireframe) |
| canvas_set_color / canvas_draw_line | ✅ | Public helpers for direct SDL2 access |

### 3LDM Model Parser (src/engine/model.rs) — 2026-04-28 overhaul
| Feature | Status | Notes |
|---------|--------|-------|
| Standard 3LDM path (FVF+Mesh) | ✅ | Header → FVF → Mesh → vertices + strip indices |
| **GT PSP variant (null mesh/FVF ptr)** | ✅ | Falls through to all-runs heuristic scanner |
| **All-runs vertex scanner** | ✅ | Collects every ≥8-vertex run, merges nearby, dedup overlaps |
| **Strip-based index detection** | ✅ | Finds i16 runs with validation ratio, detects restart markers |
| Vertex validation | ✅ | Finite + abs<100k + magnitude>0.001 filter prevents NaN/inf |
| **Embedded TXS3 texture** | ✅ | `parse_txs3_texture_at()` with base-offset correction |
| Model entry parsing | ✅ | `ModelEntry` struct, `read_models()`, setup_cmds pointers |
| Params: texture_set_ptr (0x48) | ✅ | Header field for embedded texture set |

## What's Broken / Blocking

| Priority | Issue | Details |
|----------|-------|----------|
| **P0** | ~~Car model only 28v/13tri~~ | **FIXED**: All-runs scanner finds **311v/592tri** across 23 vertex runs. GT PSP files use null mesh/FVF pointers with geometry stored in setup-cmds (VM opcodes) + scattered vertex buffers. Scanner heuristically extracts valid f32 triplets and i16 strip indices. |
| **P0** | ~~Textures not wired to rendering~~ | **FIXED**: OpenGL texture pipeline complete - UV extraction from FVF, GPU texture upload, textured shaders with UV attributes. Car and course textures render with proper texture mapping when UVs are available. |
| **P0** | Race track is procedural (low-detail) | **PARTIAL**: race.mdl loads 199v/28tri (near-LOD). Full-detail tracks are in c###x files (PACL format, not yet parsed). |
| **P1** | Triangle fill broken | The `fill_triangle()` in race.rs uses scanline fill but produces no visible output. Wireframe-only currently. |
| **P1** | Camera left/right rotates car | Arrow keys mapped to steering → moves camera. Right key = steer right = camera rotates. Feature or bug? |
| **P1** | FPS drops with more triangles | 800tri = 44 FPS, 3200tri = unplayable. Wireframe ×9 lines per tri. Need filled triangles + fewer draw calls. |
| **P2** | Menu/UI completely bypassed | Arcade.mproject loads (15 root windows) but no interaction. Game goes straight to RACE. |
| **P2** | No sound | Audio registered but no music or engine sounds play. |
| **P2** | execBoot/execBootPhase2 skipped | VM infinite loops in these, so car setup, organizer init, etc. are missing. |
| **P3** | No AI opponents | Only player car, no ghost/CPU cars. |

## What to Do Next

### Immediate
1. **~~Wire textures to rendering~~** | ✅ **COMPLETE** — OpenGL texture pipeline with UV extraction, GPU upload, and textured shaders implemented.
2. **Fix fill_triangle** — The scanline fill probably has an off-by-one or clamping issue. Debug why no scanlines produce.
3. **Remove auto-throttle** — Replace `|| true` with actual key input so player controls car.
4. **Reduce false-positive triangles** — Some scanner triangles reference wrong vertices; check index-to-vertex mapping after destripping.

### Medium-term
5. **Parse PACL track geometry** — c###x files (PACL format) contain the full 3-7MB track mesh. Research format from GT PSP track data specs.
6. **Menu UI** — Fix arcade.mproject focus widgets so menu navigation works before entering race.
7. **Improve vertex extraction** — Try stride-20 (position + normal) and stride-12 (position only) to capture interleaved vertex data beyond 12-byte runs.
8. **Implement audio system** — Engine sounds, BGM, and tire screech based on car physics.

### Later
9. AI opponents, save/load, SpecDB-driven car selection, full GT Mode.

---
tags: [pc-port, overview, status]
type: documentation
project: GT PSP PC Port
status: active
---

# GT PSP PC Port — Documentation

> Native PC port of Gran Turismo PSP using custom Rust Adhoc VM + SDL2.

## Goal

Run Gran Turismo PSP natively on Windows without PPSSPP emulation. Main game loop runs natively in Rust. Project scripts execute through the custom VM.

## Build & Run

```bash
# Build
cargo build --release

# Standard boot (via VM, includes race)
cargo run --release -- --boot

# Test 3LDM parser
cargo run --release -- --test-3ldm

# Test SpecDB
cargo run --release -- --specdb

# Debug: dump script
cargo run --release -- --dump file.adc

# Debug: trace execution
cargo run --release -- --trace file.adc
```

## Current Status (2026-04-28)

### Working ✓

| Component | Status | Lines | Notes |
|-----------|--------|-------|-------|
| VM Loader | ✅ | 431 | .adc binary parser |
| Opcode Decoder | ✅ | 333 | 77 opcodes v5-v12 |
| Execution Engine | ✅ | 689 | Stack-based |
| Value Types | ✅ | 109 | 19 variants |
| Bootstrap Scripts | ✅ | - | 6 .adc files load |
| Main Loop State | ✅ | - | 12-phase tick |
| Car Physics | ✅ | - | Accel/brake/steering |
| Car Model | ✅ | - | 311v/592tri via scanner |
| Course Loading | ✅ | - | race.mdl 199v/28tri |
| 3D Rendering | ⚠️ | - | Wireframe only |
| TXS3 Decoder | ✅ | 192 | RGB565, RGBA4444, L4, L8 |
| SDL2 Window | ✅ | 58 | 960×544 |
| Input | ✅ | - | WASD, arrows, shift |
| HUD | ✅ | - | Speed, lap, timer |
| SpecDB Reader | ✅ | - | 45+ tables |

### Broken

| Issue | Priority | Notes |
|-------|----------|-------|
| Triangle fill | P1 | Fill code returns 0 |
| Textures in 3D | P1 | Loading works, not wired |
| Menu/UI | P2 | Loads but no interaction |
| No sound | P2 | Registered but no playback |
| AI opponents | P3 | Player car only |

## Architecture

```
bootstrap.adc [VM] → execBoot()
packed_main_loop.adc [VM] → registers modules
bootstrap_phase2.adc [VM] → execBootPhase2()
native MainLoopState::tick() [NATIVE] → 12-phase state machine
  ├── CheckConditions → SEQUENCE_RACE=4 → RaceExecute → RaceRunning
  └── 3D rendering (SDL2)
```

## Race Gameplay

| Feature | Status | Notes |
|---------|--------|-------|
| Car physics | ✅ | 35 accel, 60 brake |
| **Car model loading** | ✅ | 311v/592tri via heuristic |
| Car spawn | ✅ | From c001.ad metadata |
| **Course loading** | ✅ | race.mdl 199v/28tri |
| Procedural track | ✅ | Rolling hills fallback |
| Surface following | ✅ | Grid height lookup |
| Off-track detection | ✅ | Triangle distance test |
| Lap detection | ✅ | Enter/Exit state machine |
| Auto-throttle | ✅ | Temporary (for testing) |
| Race finish | ✅ | 3 laps → restart |

## 3D Rendering

| Feature | Status | Notes |
|---------|--------|-------|
| Perspective matrix | ✅ | Fixed projection |
| LookAt camera | ✅ | Follows car |
| Wireframe | ✅ | SDL2 draw_line |
| Triangle fill | ❌ | Returns 0 |
| Camera update | ✅ | Per-frame |
| HUD | ✅ | Speed, lap, timer |
| Car render | ✅ | Wireframe red |
| Track render | ✅ | Wireframe green |

## 3LDM Parser (2026-04-28)

GT PSP uses null mesh/FVF pointers with geometry in setup commands (VM opcodes) + scattered vertex buffers.

| Feature | Status | Notes |
|---------|--------|-------|
| Standard path | ✅ | FVF → Mesh |
| **GT PSP variant** | ✅ | Falls through to scanner |
| All-runs scanner | ✅ | ≥8-vertex runs |
| Vertex dedup | ✅ | Overlap removal |
| Strip detection | ✅ | i16 runs w/ validation |
| TXS3 extraction | ✅ | Embedded textures |

## 3LDM Header Fields

| Offset | Field | Type |
|--------|-------|------|
| 0x00 | Magic (3LDM) |
| 0x04 | File Size |
| 0x10 | Model Count |
| 0x14 | Shape Count |
| 0x18 | FVF Count |
| 0x30 | Models Pointer |
| 0x38 | Meshes Pointer |
| 0x40 | FVF Pointer |
| 0x48 | Texture Set Pointer |

## TXS3 Texture Formats

| Format | Bytes/Pixel |
|--------|-----------|
| RGBA8888 | 4 |
| RGB565 | 2 |
| RGBA4444 | 2 |
| RGBA5551 | 2 |
| L8 | 1 |
| L4 | 0.5 |

## See Also

- [[10_PC_Port/00_Index|Index]]
- [[20_ADHOC_VM/00_Index|Adhoc VM]]
- [[30_Technical/00_Index|Technical]]
- [[40_Reference/00_Index|Reference]]

---

*Updated: 2026-04-29*
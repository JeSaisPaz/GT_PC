---
tags: [index, pc-port, rust, vm, sdl2]
type: index
project: GT PSP PC Port
status: active
---

# GT PSP PC Port — Hub

> Native PC port of Gran Turismo PSP using custom Rust Adhoc VM + SDL2.

## Quick Links

| Section | Description |
|---------|-------------|
| **[[10_PC_Port/00_Index|Index]]** | Main PC Port documentation |
| **[[10_PC_Port/07_Playable_Game_Guide|Playable Game Guide]]** | Step-by-step to gameplay |
| **[[10_PC_Port/08_VM_Initialization|VM Initialization]]** | VM + native setup |
| **[[10_PC_Port/09_SpecDB_Loading|SpecDB Loading]]** | Database + optimization |
| **[[10_PC_Port/11_Menu_UI|Menu UI]]** | Focus navigation + activation |
| **[[10_PC_Port/12_Render_Issue|Render Issue]]** | Red screen debugging (open) |
| **[[10_PC_Port/01_Documentation|Documentation]]** | Full PC Port docs |
| **[[10_PC_Port/02_Race_Engine|Race Engine]]** | Physics implementation |
| **[[10_PC_Port/03_Model_Parser|3D Model Parser]]** | 3LDM parsing |
| **[[10_PC_Port/04_SpecDB_Reader|SpecDB Reader]]** | Database parser |
| **[[10_PC_Port/05_Graphics|Graphics]]** | SDL2 renderer |
| **[[10_PC_Port/06_Native_API|Native API]]** | 380+ native functions |
| **[[20_ADHOC_VM/00_Index|Adhoc VM]]** | Rust VM |
| **[[40_Reference/00_Index|Reference]]** | Game reference |

## Architecture

```
.adc Scripts (~18 .adc + ~36 .ad source in codebase)
         ↓ VM (Rust)
Native Engine APIs
         ↓ platform
SDL2 + OpenGL
```

## Build & Run

```bash
cargo build --release
cargo run --release -- --boot  # Standard boot (includes race mode)
```

## Current Status

| Component | Status |
|-----------|--------|
| VM Loader | ✅ |
| Race Engine | ✅ |
| 3D Model Parser | ✅ |
| SpecDB Reader | ✅ |
| Graphics (SDL2) | ✅ |
| HUD | ✅ |
| Track Timing | ✅ |
| Native API | 350 functions |
| **Texture Rendering** | ✅ **Wired to OpenGL** |
| **VM Bootstrap** | ✅ **execBoot/execBootPhase2 execute correctly (IteratorNext fixed)** |
| **Menu UI Focus** | ✅ **Navigation and activation implemented** |

## Quick Reference

| Command | Description |
|---------|-------------|
| `--boot` | Boot via VM (includes race) |
| `--dump <file>` | Dump script |
| `--trace <file>` | Trace execution |
| `--list-native` | Native functions |

## See Also

- [[10_PC_Port/12_Render_Issue|Render Issue]]
- [[10_PC_Port/02_Race_Engine|Race Engine]]
- [[90_Legacy_Modding/00_Index|Legacy Modding]]

---

*Updated: 2026-04-29 (IteratorNext, Bootstrap Execution, Menu UI Focus, Render Issue)*
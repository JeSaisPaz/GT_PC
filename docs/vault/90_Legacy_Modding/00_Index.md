---
tags: [index, legacy, modding, ppsspp]
type: index
section: Legacy
status: historical
---

# Legacy Modding — Index

> Historical modding information for GT PSP (PPSSPP emulator).

> **Note**: This content is for reference. The PC Port is the active project.

## Mod Loader Framework

### Quick Start

```powershell
# Build modded core
.\mod_loader\build_modded_core.ps1

# Create mod
python mod_loader/cli/gtpsp_mod.py init my_mod

# Deploy
python mod_loader/cli/gtpsp_mod.py deploy my_mod/ --ppsspp-dir <path>
```

## Hook System

| Event | Fires |
|-------|-------|
| `onGameStart` | First MainLoop iteration |
| `beforeMenu` | Before menu project |
| `afterMenu` | After menu project exits |
| `beforeRace` | Before race |
| `afterRace` | After race |

## Mod Types

- **Gameplay** — Script hooks
- **Car** — SpecDB + model files
- **Track** — COURSE.dbt + courselist.xml
- **Texture** — Asset replacement
- **Audio** — AT3 replacement

## See Also

- [[10_PC_Port/00_Index|PC Port]]
- [[40_Reference/00_Index|Game Reference]]
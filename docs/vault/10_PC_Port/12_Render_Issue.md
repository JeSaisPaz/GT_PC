---
tags: [pc-port, rendering, issue, graphics, opengl, sdl2]
type: debugging
project: GT PSP PC Port
status: resolved
created: 2026-04-29
updated: 2026-04-29
affected: race.rs, graphics.rs, sprite.rs, model.rs
---

# Race Rendering Issue — Red Screen

> Race mode shows red screen with overlapping text instead of proper 3D track rendering.

## Symptom (Original)

- **Display:** Entire screen is red with white/yellow text overlapping
- **Log output showed:** `"[Race] Course texture loaded (8x512)"` — suspicious 8px wide texture
- **FPS:** ~23 fps (software rendering struggling)

## Root Causes Identified & Fixed

### Fix 1: Course Texture Size (`compute_real_dim`)

**File:** `pc_port/src/engine/sprite.rs:159`

`compute_real_dim()` was using a brute-force divisor search that returned wrong dimensions (8x512 instead of 128x64).

**Fix:** Trust texture header dimensions first; only recalculate if header is clearly wrong.

**Result:** `Course texture loaded (128x64)` — correct size.

### Fix 2: Stale Projection Matrix

**File:** `pc_port/src/engine/race.rs:404-431`

`render()` was using the renderer's cached `proj_matrix` which was never updated after window resize or FOV changes.

**Fix:** Compute fresh perspective matrix each frame and update renderer:
```rust
let proj = Mat4::perspective(self.camera.fov, aspect, 0.1, 500.0);
rb.proj_matrix = proj.m;
```

### Fix 3: Triangles Behind Camera Filling Screen

**File:** `pc_port/src/engine/race.rs`

The "red screen" was caused by triangles behind the camera being projected to giant screen-filling shapes after perspective divide.

**Fix:** Added `project_vertex_safe()` which returns `None` if:
- `w < 0.001` (behind camera or too close)
- `ndc_z < -1.0 || ndc_z > 1.0` (outside near/far planes)

The rendering loop now skips triangles where any vertex fails the clip test.

### Fix 4: `race.mdl` is a Test Mesh, Not a Track

**File:** `pc_port/src/engine/model.rs:639`

`race.mdl` is only 28KB with 199 vertices / 28 triangles — a shared test mesh, not a drivable course. Real courses are multi-MB `crs/cXXXx` files.

**Fix:** `load_course()` now:
1. Attempts to load `crs/cXXXx` file (real course model)
2. Falls back to procedural 400×400m track (21×21 grid, 441v/800tri)

### Fix 5: Screen-Space Clipping in `fill_triangle()`

**File:** `pc_port/src/engine/race.rs:766-837`

Added NDC range check (`-2.0` to `2.0`) and screen bounds clamping to prevent scanline overflow.

### Fix 6: Back-Face Culling

Added cheap back-face test using screen-space cross product:
```rust
let area = (b.0 - a.0) * (c.1 - a.1) - (b.1 - a.1) * (c.0 - a.0);
if area <= 0.0 { continue; }
```

## Current State

- ✅ No more red screen
- ✅ No more crashes during race
- ✅ Stable 60 fps (SDL2 software path)
- ✅ Proper 3D track rendering (green grass, gray road)
- ✅ Car visible at spawn position
- ✅ Start/finish line drawn
- ✅ HUD readable (speed, time, laps, track status)
- ❌ Real `cXXXx` course models not yet parsed (procedural fallback used)
- ❌ No texture mapping on track (solid colors only in SDL2 path)
- ❌ OpenGL path not enabled (`init_opengl()` defined in `graphics.rs` but not called in `main.rs`)

## Render Flow (Current)

```
main.rs:732  → renderer.clear()
    ↓
main.rs:738  → loop_state.tick()
    ↓
main_loop.rs:317 → self.race.render()
    ↓
race.rs:404  → get_opengl_renderer()? → None (OpenGL not initialized)
                          ↓ (fallback)
                       → render_sdl2() → project_vertex_safe() → fill_triangle()
    ↓
main.rs:860  → renderer.end_scene() → present()
```

## Files Involved (Updated)

| File | Lines | Role |
|------|-------|------|
| `pc_port/src/engine/race.rs` | 404-431 | `render()` — projection matrix fix |
| `pc_port/src/engine/race.rs` | 616-715 | `render_sdl2()` — 3D solid fill rendering |
| `pc_port/src/engine/race.rs` | 766-837 | `fill_triangle()` — scanline + clipping |
| `pc_port/src/engine/race.rs` | 839-864 | `draw_triangle_outline()` — wireframe debug |
| `pc_port/src/engine/race.rs` | 866-909 | `project_vertex_safe()` — depth clipping |
| `pc_port/src/engine/sprite.rs` | 159-200 | `compute_real_dim()` — texture size fix |
| `pc_port/src/engine/model.rs` | 639-670 | `load_course()` — cXXXx + procedural fallback |
| `pc_port/src/engine/graphics.rs` | 207-213 | `clear()` function |
| `pc_port/src/main.rs` | 490-495 | `init_opengl()` — still commented out |

## Log Excerpt (After Fixes)

```
[Race] Loading course c001.ad...
[COURSE] Using procedural track for c001 (spawn -65,-415)
[COURSE] Procedural: 441v 800tri grid 21x21
[Race] Loading course texture...
[Race] Course texture loaded (128x64)
[Race] Init: car (-64.8,0.6,-414.6) heading 0.0° lap 1/3
[Game] Frame 3000 | 59.2 fps | insn: 14K | seq: ONLINE | proj: arcade | phase: RaceRunning
```

## Linked Documents

- [[10_PC_Port/02_Race_Engine|Race Engine]] — race rendering and physics
- [[10_PC_Port/05_Graphics|Graphics]] — SDL2/OpenGL renderer details
- [[10_PC_Port/10_OpenGL_Backend|OpenGL Backend]] — OpenGL path (not enabled)
- [[10_PC_Port/07_Playable_Game_Guide|Playable Game Guide]] — how to run race
- [[10_PC_Port/03_Model_Parser|3D Model Parser]] — course/car model loading
- [[10_PC_Port/11_Course_Models|Course Models]] — cXXXx file format research

---

*Created: 2026-04-29 — Issue resolved, documented for future reference*
*See also: [[10_PC_Port/00_Index|PC Port Index]]*

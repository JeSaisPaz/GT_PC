---
tags: [pc-port, rust, engine, physics]
type: documentation
project: GT PSP PC Port
section: Core Engine
---

# Race Engine — PC Port

> Race gameplay implementation in Rust (`pc_port/src/engine/race.rs`).

## Overview

Native race physics engine that drives the `RaceRunning` phase in the PC Port.

## Physics Constants

| Constant | Value | Description |
|----------|-------|-------------|
| `ACCEL_RATE` | 35.0 | Acceleration rate |
| `BRAKE_RATE` | 60.0 | Braking rate |
| `STEER_RATE` | 3.0 | Steering responsiveness |
| `DRAG_COEFF` | 0.35 | Air resistance |
| `OFF_TRACK_DRAG` | 5.0 | Off-track penalty |
| `MAX_SPEED` | 90.0 | Speed cap |
| `TRACK_BOUNDARY_DIST` | 8.0 | Off-track threshold |
| `LAP_CROSS_DIST` | 15.0 | Lap detection distance |

## CarState

```rust
pub struct CarState {
    pub x: f32, pub y: f32, pub z: f32,  // Position
    pub heading: f32,                      // Facing direction
    pub speed: f32,                        // Current speed
    pub steer_angle: f32,                  // Steering input
    pub throttle: f32,                    // Throttle input
    pub brake: f32,                       // Brake input
    pub car_id: u32,                      // Car ID
    pub on_track: bool,                    // Track status
}
```

## RaceState

```rust
pub struct RaceState {
    pub car: CarState,
    pub camera: ChaseCamera,
    pub track: Option<TrackState>,
    pub car_model: Option<CarModel>,
    pub car_texture: Option<LoadedTexture>,
    pub course_texture: Option<LoadedTexture>,
    pub course_id: u32,
    pub current_lap: i32,
    pub total_laps: i32,
    pub elapsed: f32,
    pub finished: bool,
    pub started: bool,
    pub initialized: bool,
    // Lap detection
    start_z: f32,
    start_x: f32,
    in_start_zone: bool,
    last_lap_z: f32,
    // Spatial index
    track_grid: Option<TrackGrid>,
}
```

## Initialization

```rust
pub fn initialize(&mut self, course_id: u32, car_code: u32)
```

1. Load course via `load_course(course_id)`
2. Build spatial index via `build_track_grid(&track)`
3. Load car model via `load_car_model(car_code)`
4. Load car texture via `load_car_texture(car_code)`
5. Load course texture via `load_course_texture()`
6. Position car at track center

## Update Loop

```rust
pub fn update(&mut self, dt: f32, throttle: bool, brake: bool, steer_left: bool, steer_right: bool)
```

### Per-frame Physics

1. **Input processing** — throttle/brake/steer from keyboard
2. **Acceleration** — `speed += throttle * ACCEL_RATE * dt`
3. **Braking** — `speed -= brake * BRAKE_RATE * dt`
4. **Drag** — `speed -= speed * DRAG_COEFF * dt` (or `OFF_TRACK_DRAG`)
5. **Steering** — speed-dependent: `turn_rate = steer * STEER_RATE * (speed / (speed + 8.0))`
6. **Position** — `x += sin(heading) * speed * dt`, `z += cos(heading) * speed * dt`
7. **Track height** — `find_track_height(x, z)`
8. **Off-track check** — nearest triangle distance
9. **Lap detection** — Enter/Exit/Re-enter state machine

## ChaseCamera

```rust
pub struct ChaseCamera {
    pub distance: f32,  // 15.0
    pub height: f32,   // 6.0
    pub fov: f32,      // 1.2
}
```

## TrackGrid (Spatial Index)

Fast polygon lookup:

```rust
struct TrackGrid {
    min_x, max_x, min_z, max_z: f32,
    cells_x, cells_z: usize,
    cell_w, cell_h: f32,
    cell_tris: Vec<Vec<usize>>,  // Triangle indices per cell
}
```

Grid accelerates:
- Height lookup: O(1) via cell lookup
- Off-track detection: nearest triangle test

## See Also

- [[10_PC_Port/00_Index|PC Port Index]]
- [[10_PC_Port/12_Render_Issue|Render Issue]] (red screen debugging)
- [[10_PC_Port/05_Graphics|Graphics]]
- [[30_Technical/01_3LDM_Format|3LDM Format]]
- [[30_Technical/02_Textures|Textures]]
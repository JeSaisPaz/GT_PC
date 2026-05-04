---
tags: [index, workflow, boot, gameplay]
type: guide
project: GT PSP PC Port
section: Guide
status: active
---

# Playable Game Guide

> Step-by-step guide from startup to gameplay.

## Overview

This guide details every step from launching the PC Port to playing a race, covering:
1. Boot sequence
2. Script loading
3. VM initialization
4. Main loop execution
5. Race mode
6. Input handling

---

## Step 1: Launch & Assets

### Entry Point

```bash
cargo run --release -- --boot
```

### Assets Path

```rust
// main.rs
let default_gt_vol = "files/decompiled/Gran Turismo/PSP_GAME/USRDIR/GT.VOL";
engine::init_assets_root(default_gt_vol);
```

### Asset Structure Required

```
GT.VOL/
├── scripts/gt5m/       # Application.adc, bootstrap.adc, etc.
├── projects/gt5m/      # UI projects
├── products/gt5m/      # MenuClassDefine
├── crs/               # Track data (c001/, c002/, ...)
├── car/               # Car models
├── specdb/            # Database tables
└── ...
```

---

## Step 2: VM Initialization

### Native Registration

```rust
// main.rs
let specdb = Rc::new(RefCell::new(engine::specdb::SpecDB::new()));
let mut natives = NativeRegistry::new();

// Register all native modules
engine::audio::register_audio(&mut natives);
engine::pdistd::register_pdistd(&mut natives);
engine::pdiext::register_pdiext(&mut natives);
engine::pdiapp::register_pdiapp(&mut natives);
engine::menu::register_menu(&mut natives);
engine::gtengine::register_gtengine(&mut natives, specdb.clone());
```

### Native Modules

| Module | Functions | Purpose |
|--------|-----------|----------|
| `pdistd` | 39 | File I/O, strings, math, XML |
| `pdiext` | 58 | Product info, font, sound |
| `gtengine` | 58 | SpecDB, car/track data |
| `menu` | 146 | UI widgets (stubs) |
| `pdiapp` | 31 | App utilities |
| `audio` | 18 | Audio playback |

---

## Step 3: SpecDB Loading

```rust
// Load database tables
specdb.load_all("specdb/GT_PSP_JP2817")?;

// Tables loaded:
// - GENERIC_CAR (837 rows)
// - ENGINE, CHASSIS, SUSPENSION, etc.
// - COURSE (85 tracks)
// - VARIATION (6042 colors)
// - CAR_NAME_* (localized names)
```

### SpecDB Structure

```
GT_PSP_JP2817/
├── GENERIC_CAR.dbt     # Car specs
├── ENGINE.dbt
├── CHASSIS.dbt
├── SUSPENSION.dbt
├── COURSE.dbt          # Tracks
├── RACE.dbt            # Events
├── VARIATION.dbt       # Colors
├── CAR_NAME_american.dbt
├── CAR_NAME_japanese.dbt
└── ...
```

---

## Step 4: Bootstrap Scripts

### Load Sequence

```
Application.adc (root)
    ↓ load
bootstrap.adc (singletons)
    ↓ load
packed_main_loop.adc (main loop)
    ↓ load
bootstrap_phase2.adc (phase 2 init)
```

### Root Frame Execution

```rust
// Load Application.adc
let bytecode = std::fs::read("Application.adc")?;
let code_frame = Loader::load(&bytecode)?;

// Add to registry
let frame_idx = registry.add_frame(code_frame);

// Execute root frame (registers modules)
vm.exec_root(frame_idx)?;
```

### What bootstrap.adc Does

```adhoc
module ::main {
    static manager;    // Widget manager
    static sound;      // Audio system
    static ORG;        // Race organizer
    static RaceOperator;
    
    module main::menu {
        class EventLoop { ... }
    }
}
```

### Modules Registered

- `main::pdistd` — Standard library
- `main::pdiext` — Extended library  
- `main::gtengine` — Game engine
- `main::menu` — UI framework
- `main::GameSequence` — State machine
- `main::GlobalStatus` — Save data

---

## Step 5: Main Loop Initialization

### Native MainLoopState

```rust
let mut main_loop = MainLoopState::new();

// Initial state:
main_loop.phase = CheckConditions;
main_loop.game_sequence.current_sequence = SEQUENCE_MENU;
main_loop.game_sequence.current_project = "arcade".to_string();
```

### State Machine Phases

```
CheckConditions
    ↓ (SEQUENCE_MENU)
MenuAllocateReplay
    ↓
MenuLoadResource
    ↓
MenuSetMode
    ↓
MenuStartProject      ← Loads arcade project
    ↓
MenuRunInit           ← Runs onLoad callbacks
    ↓
MenuSync             ← Waits for user (VM runs here)
    ↓
...
```

---

## Step 6: Menu Project Loading

### MGOM.start()

When `MenuStartProject` phase executes:

```rust
// Native call to load UI project
if let Some(nf) = vm.natives.get("main,menu,MMenuGameObjectManager,start") {
    nf(&[Value::String(project_name)]);
}
```

### Project Scripts Loaded

```
arcade/
├── arcade.ad           # Entry point
├── TopRoot.ad          # Main menu
├── CarRoot.ad          # Car selection
├── CourseRoot.ad       # Track selection
└── ...
```

### MenuSync Phase

- VM executes project scripts over multiple frames
- Each frame: execute up to 5000 instructions
- If VM has work → return Ok(true) for next frame
- When VM completes → advance to next phase

---

## Step 7: Starting a Race

### User Action → Race Sequence

When user selects race:

```adhoc
main::GameSequence::setNextSequence(RACE);
main::GameSequence::setNextProject("arcade");
```

### Phase Transition

```
MenuSync (VM running)
    ↓ (sequence = RACE)
CheckConditions
    ↓
RaceBGM
    ↓
RaceSetMode
    ↓
RaceExecute
    ↓
RaceRunning ← Native physics
```

---

## Step 8: Race Initialization

### RaceState.initialize()

```rust
pub fn initialize(&mut self, course_id: u32, car_code: u32) {
    // 1. Load track
    self.track = Some(load_course(course_id)?);
    self.track_grid = build_track_grid(&track);
    
    // 2. Load car model
    self.car_model = Some(load_car_model(car_code)?);
    
    // 3. Load textures
    self.car_texture = load_car_texture(car_code);
    self.course_texture = load_course_texture();
    
    // 4. Position car at track center
    self.car.x = track.center.0;
    self.car.z = track.center.2;
    self.car.y = find_track_height(self.car.x, self.car.z);
    
    // 5. Initialize lap tracking
    self.start_x = self.car.x;
    self.start_z = self.car.z;
    self.current_lap = 1;
    self.in_start_zone = true;
}
```

### Track Loading

```
crs/c001/
├── race.mdl      # Track geometry (3LDM)
├── race.txs     # Track texture
├── c001.ad      # Course metadata
├── c001.cam     # Camera data
└── c001.cinf    # Course info
```

### Car Loading

```
car/<car_code>/
├── body          # Car model (3LDM)
├── wheel_f       # Front wheel
├── wheel_r       # Rear wheel
└── (embedded TXS3 texture)
```

---

## Step 9: Race Execution

### Per-Frame Update

```rust
pub fn update(&mut self, dt: f32, throttle: bool, brake: bool, 
              steer_left: bool, steer_right: bool) {
    
    // 1. Input
    self.car.throttle = throttle as f32;
    self.car.brake = brake as f32;
    self.car.steer_angle = ...;
    
    // 2. Physics
    // Acceleration
    self.car.speed += throttle * 35.0 * dt;
    // Braking
    self.car.speed -= brake * 60.0 * dt;
    // Drag
    self.car.speed -= self.car.speed * DRAG * dt;
    
    // 3. Steering
    let turn_rate = steer * 3.0 * (speed / (speed + 8.0));
    self.car.heading += turn_rate * dt;
    
    // 4. Movement
    self.car.x += sin(heading) * speed * dt;
    self.car.z += cos(heading) * speed * dt;
    
    // 5. Track surface
    if let Some(h) = self.find_track_height(x, z) {
        self.car.y = h;
    }
    
    // 6. Off-track check
    let dist = self.nearest_triangle_distance(x, z);
    self.car.on_track = dist < TRACK_BOUNDARY_DIST;
    
    // 7. Lap detection
    self.check_lap_crossing();
}
```

### Physics Constants

| Constant | Value | Description |
|----------|-------|-------------|
| `ACCEL_RATE` | 35.0 | Acceleration |
| `BRAKE_RATE` | 60.0 | Braking |
| `STEER_RATE` | 3.0 | Steering |
| `DRAG_COEFF` | 0.35 | Air resistance |
| `OFF_TRACK_DRAG` | 5.0 | Off-track penalty |
| `MAX_SPEED` | 90.0 | Speed cap |
| `TRACK_BOUNDARY_DIST` | 8.0 | Off-track threshold |

---

## Step 10: Rendering

### Per-Frame Render

```rust
fn render(&mut self) {
    // Clear
    self.clear();
    
    // Setup camera
    let cam = &self.race.camera;
    let eye = (car.x - sin(h)*dist, car.y + height, car.z - cos(h)*dist);
    let target = (car.x, car.y, car.z);
    self.view_matrix = look_at(eye, target);
    
    // Draw track (wireframe)
    for tri in &track.triangles {
        draw_triangle(vertices[tri], GREEN);
    }
    
    // Draw car (wireframe)
    for tri in &car_model.triangles {
        draw_triangle(vertices[tri], RED);
    }
    
    // Draw HUD
    draw_text(format!("{} km/h", speed), 10, 10);
    draw_text(format!("Lap {}/{}", lap, total), 10, 30);
    
    self.present();
}
```

### Camera

```rust
struct ChaseCamera {
    distance: 15.0,  // Behind car
    height: 6.0,     // Above car
    fov: 1.2,        // Field of view
}
```

---

## Step 11: Input Handling

### Keyboard Mapping

| Key | Action | PSP Equivalent |
|-----|--------|---------------|
| W / ↑ | Throttle | CROSS |
| S / ↓ | Brake | CIRCLE |
| A / ← | Steer left | LEFT |
| D / → | Steer right | RIGHT |
| Shift | Boost | L1 |
| Enter | Confirm | CROSS |
| Escape | Exit | START |

### Input State

```rust
struct InputState {
    throttle: bool,
    brake: bool,
    steer_left: bool,
    steer_right: bool,
    boost: bool,
    confirm: bool,
    quit: bool,
}
```

---

## Step 12: Race Completion

### Lap Detection

```rust
fn check_lap_crossing(&mut self) {
    let dist_to_start = distance(car.x, car.z, start_x, start_z);
    
    if in_start_zone && dist_to_start > LAP_CROSS_DIST {
        // Left start zone
        in_start_zone = false;
    } else if !in_start_zone && dist_to_start < LAP_CROSS_DIST {
        // Re-entered = lap complete
        in_start_zone = true;
        current_lap += 1;
        
        if current_lap > total_laps {
            finished = true;
        }
    }
}
```

### Finish

```rust
if finished {
    // Show results
    // Then restart or exit
    restart_race();
}
```

---

## Complete Flow Diagram

```
┌─────────────────────────────────────────────────────────────────┐
│ cargo run --release -- --boot                                 │
└─────────────────────────────────────────────────────────────────┘
                            ↓
┌─────────────────────────────────────────────────────────────────┐
│ 1. Initialize Assets Root                                     │
│    Load GT.VOL path, set up file system                     │
└─────────────────────────────────────────────────────────────────┘
                            ↓
┌─────────────────────────────────────────────────────────────────┐
│ 2. Register Native APIs (380 functions)                      │
│    pdistd, pdiext, gtengine, menu, pdiapp                 │
└─────────────────────────────────────────────────────────────────┘
                            ↓
┌─────────────────────────────────────────────────────────────────┐
│ 3. Load SpecDB                                              │
│    Parse 45+ tables (GENERIC_CAR, COURSE, etc.)            │
└─────────────────────────────────────────────────────────────────┘
                            ↓
┌─────────────────────────────────────────────────────────────────┐
│ 4. Load Bootstrap Scripts                                   │
│    Application.adc → bootstrap.adc → packed_main_loop      │
│    Registers modules, functions, statics                     │
└─────────────────────────────────────────────────────────────────┘
                            ↓
┌─────────────────────────────────────────────────────────────────┐
│ 5. Create MainLoopState                                     │
│    phase = CheckConditions, sequence = MENU                 │
└─────────────────────────────────────────────────────────────────┘
                            ↓
        ┌─────────────────────────────────────────────────┐
        │           MAIN LOOP (per frame)                 │
        ├─────────────────────────────────────────────────┤
        │ tick():                                      │
        │   1. Drain loaded VM frames                   │
        │   2. Execute VM (5K insns)                  │
        │   3. State machine:                        │
        │      CheckConditions                         │
        │        ↓                                     │
        │      MenuAllocateReplay                      │
        │        ↓                                     │
        │      MenuLoadResource → Load fonts          │
        │        ↓                                     │
        │      MenuSetMode                            │
        │        ↓                                     │
        │      MenuStartProject → Load UI             │
        │        ↓                                     │
        │      MenuRunInit → onLoad callbacks        │
        │        ↓                                     │
        │      MenuSync ← VM runs here                │
        │        ↓ (user selects race)                 │
        │      RaceBGM                                │
        │        ↓                                     │
        │      RaceExecute                            │
        │        ↓                                     │
        │      RaceRunning ← Native physics!         │
        │        ↓                                     │
        │      RaceEndReplay                          │
        │        ↓                                     │
        │      ClearFontCache                         │
        └─────────────────────────────────────────────────┘
                            ↓
┌─────────────────────────────────────────────────────────────────┐
│ Race Running:                                               │
│   • Load track (crs/c001/race.mdl)                       │
│   • Load car (car/xxx/body)                               │
│   • Per-frame:                                             │
│       - Read input                                         │
│       - Physics (accel, brake, steer)                     │
│       - Update position                                     │
│       - Check off-track                                    │
│       - Check lap crossing                                 │
│       - Render (track + car + HUD)                        │
└─────────────────────────────────────────────────────────────────┘
```

---

## Current Status

| Step | Status | Notes |
|------|--------|-------|
| 1-3 | ✅ | Assets, natives, SpecDB (54 tables) |
| 4 | ✅ | Bootstrap scripts load |
| 5-6 | ✅ | MainLoopState works |
| 7 | ✅ | Menu project loads |
| 8 | ✅ | Track/car loading works |
| 9 | ✅ | Physics works |
| 10 | ✅ | SDL2 rendering + GL context |
| 11 | ✅ | Input works |
| 12 | ✅ | Lap detection works |

## Implemented Features

### HUD (Task 2)
- **Speedometer**: Large green display
- **Best Lap**: Blue, shows fastest lap
- **Lap Time**: Per-lap timing
- **Track Status**: Red when off-track

### Timing (Task 3)
- **Best Lap**: Automatically tracked
- **Last Lap**: Per-lap time
- **Checkpoints**: 3 checkpoints with sector splits
- Console logging for timing

### OpenGL (Task 1)
- SDL2 window creates GL context
- Ready for OpenGL rendering

---

## See Also

- [[10_PC_Port/00_Index|PC Port Index]]
- [[10_PC_Port/02_Race_Engine|Race Engine]]
- [[10_PC_Port/12_Render_Issue|Render Issue]] (debugging)
- [[20_ADHOC_VM/03_Main_Loop|Main Loop]]
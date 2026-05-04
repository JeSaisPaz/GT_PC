---
tags: [pc-port, rust, vm, main-loop, state-machine]
type: documentation
project: GT PSP PC Port
section: Core Engine
---

# Main Loop State Machine

> Native game loop replacement (`pc_port/src/engine/main_loop.rs`).

## Overview

Replaces `packed_main_loop.adc` bytecode interpretation with native Rust for the core game loop while keeping the VM available for project scripts.

## LoopPhase States

```rust
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum LoopPhase {
    /// Check exit conditions and determine next phase
    CheckConditions,
    /// MENU: allocate replay buffer
    MenuAllocateReplay,
    /// MENU: load menu resources (fonts)
    MenuLoadResource,
    /// MENU: set game mode
    MenuSetMode,
    /// MENU: start project (MGOM.start)
    MenuStartProject,
    /// MENU: run init callbacks (onLoad, onInit)
    MenuRunInit,
    /// MENU: wait for user interaction (MGOM.sync spanning frames)
    MenuSync,
    /// MENU: unload menu resources
    MenuUnloadResource,
    /// MENU: free unused replay buffer
    MenuFreeReplay,
    /// RACE: play before-race BGM
    RaceBGM,
    /// RACE: set game mode
    RaceSetMode,
    /// RACE: execute race — pushes race scripts to VM
    RaceExecute,
    /// RACE: waiting for race VM scripts to finish
    RaceRunning,
    /// RACE: end replay buffer
    RaceEndReplay,
    /// RACE: execute next game plan event
    RaceExecuteNext,
    /// Clear font cache (end of each loop iteration)
    ClearFontCache,
}
```

## MainLoopState

```rust
pub struct MainLoopState {
    pub phase: LoopPhase,
    pub menu_resource_loaded: bool,
    pub game_sequence: GameSequenceCtx,
    pub race_started: bool,
    pub should_exit: bool,
    pub race: RaceState,  // Native race physics
}

pub struct GameSequenceCtx {
    pub finished: bool,
    pub current_sequence: i32,
    pub prev_sequence: i32,
    pub current_project: String,
    pub prev_project: String,
}
```

## Sequence Constants

Matches `GameSequence.ad`:

```rust
pub const SEQUENCE_UNDEFINED: i32 = 0;
pub const SEQUENCE_MENU: i32 = 1;
pub const SEQUENCE_SINGLE_RACE: i32 = 2;
pub const SEQUENCE_RACE: i32 = 4;

// NOT in native code yet (reserved):
// SEQUENCE_ONLINE_BATTLE = 3
// SEQUENCE_REPLAY_THEATER = 5
// SEQUENCE_LEAVE_DEMO = 6
```

## Per-Frame Tick

```rust
pub fn tick(&mut self, vm: &mut Engine) -> Result<bool, String>
```

### Flow per frame:

1. **Drain loaded frames** — Add new VM scripts from queue
2. **Execute VM frames** — Run up to 5K instructions
3. **State machine** — Process current phase

## Phase Transitions

```
CheckConditions ───┬──→ SEQUENCE_MENU ──→ MenuAllocateReplay
    │               │                        │→ MenuLoadResource
    │               │                        │→ MenuSetMode
    │               │                        │→ MenuStartProject (loads project)
    │               │                        │→ MenuRunInit (onLoad callbacks)
    │               │                        │→ MenuSync (blocking: waits for VM)
    │               │                        │→ MenuUnloadResource
    │               │                        │→ MenuFreeReplay
    │               │                        │         ↓
    │               └──→ SEQUENCE_RACE ──→ RaceBGM
    │                                       │→ RaceSetMode
    │                                       │→ RaceExecute
    │                                       │→ RaceRunning (native physics)
    │                                       │→ RaceEndReplay
    │                                       │→ ClearFontCache
    │                                       │           ↓
    └──────────── Error / Exit ────────────────────→ Exit
```

## VM Integration

### Menu Mode (MenuSync)

- Project scripts (arcade, race, dialog) run in VM
- Each frame: run up to 5000 VM instructions
- If VM still has work → return Ok(true) to continue next frame
- State machine waits at MenuSync until VM completes

### Race Mode (RaceRunning)

- Native race physics in Rust (`RaceState`)
- Background VM handles race script execution
- Input mapped: W/↑=accelerate, S/↓=brake, A/←=left, D/→=right

```rust
// Input bit constants
const BIT_CROSS: u32 = 10;  // S key — accelerate
const BIT_CIRCLE: u32 = 9;  // D key — brake
const BIT_UP: u32 = 2;      // accelerate
const BIT_DOWN: u32 = 4;    // brake
const BIT_LEFT: u32 = 5;    // steer left
const BIT_RIGHT: u32 = 3;   // steer right
```

## Native Function Calls

Each phase calls native functions registered in VM:

| Phase | Native Call | Purpose |
|-------|-------------|---------|
| CheckConditions | `GameSequence.getCurrentSequence` | Get current state |
| CheckConditions | `MSystemCondition.IsExitGame` | Check exit |
| MenuSetMode | `GameSequence.setMode` | Set mode |
| MenuStartProject | `MGOM.start(project)` | Load UI project |
| RaceExecute | `ORG.enterCourse` | Initialize race |
| RaceRunning | Native (RaceState.update) | Physics tick |

## Frame Execution

```rust
pub fn execute_frame(&mut self, max_insns: u64) -> Result<Option<Value>, String> {
    // 1. Drain newly loaded frames
    let new_frames = crate::vm::loader::drain_loaded_frames();
    for cf in new_frames {
        let cf_idx = self.modules.add_frame(cf);
        let fi = self.modules.get_frame(cf_idx);
        let ef = Frame::new(cf_idx, fi.stack_size, fi.local_var_count, 0);
        self.call_stack.push(ef);
    }
    
    // 2. Execute frame
    let frame_result = self.exec_frame(self.call_stack.len() - 1)?;
    
    // 3. Handle result
    match frame_result {
        FrameAction::Continue => Ok(None),
        FrameAction::Return(val) => Ok(Some(val)),
        FrameAction::Exit => Ok(Some(Value::Void)),
        FrameAction::Yield => Ok(None),
    }
}
```

## Input Mapping

Real PSP button codes mapped to keyboard:

| PSP Button | Key | Purpose |
|------------|-----|---------|
| CROSS | S / ↓ | Accelerate |
| CIRCLE | D / ↑ | Brake |
| LEFT | A / ← | Steer left |
| RIGHT | D / → | Steer right |
| LSHIFT | L | Quit race |
| ENTER | Enter | Confirm |

## See Also

- [[10_PC_Port/00_Index|PC Port Index]]
- [[10_PC_Port/02_Race_Engine|Race Engine]]
- [[20_ADHOC_VM/01_VM_Engine|VM Engine]]
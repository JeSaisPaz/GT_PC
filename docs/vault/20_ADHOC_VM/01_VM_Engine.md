---
tags: [pc-port, rust, vm, engine, execution]
type: documentation
project: GT PSP PC Port
section: Core Engine
---

# VM Engine — Bytecode Execution

> Core execution engine (`pc_port/src/vm/engine.rs`).

## Engine Structure

```rust
pub struct Engine {
    pub modules: ModuleRegistry,           // All loaded code frames
    pub natives: NativeRegistry,          // Native function table
    pub static_storage: StaticStorage,   // Module-level statics
    pub call_stack: Vec<Frame>,         // Runtime call frames
    pub current_module_index: usize,     // Next static base offset
    pub module_stack: Vec<String>,       // Module path stack
    pub def_counter: usize,               // Frame definition counter
    pub func_frame_map: Vec<usize>,       // Function → frame index
    pub loader: Option<Fn(&str) -> Result<i32>>,  // Script loader
    pub global_functions: HashMap<String, FunctionValue>,  // Named functions
    pub trace: bool,                      // Instruction tracing
    pub log_native: bool,                 // Native call logging
    pub max_instructions: u64,             // Safety limit (500M)
    insn_count: u64,                       // Executed instruction count
    
    // Module static scoping
    pub module_static_bases: HashMap<String, usize>,
    pub next_static_slot: usize,
    
    // Child frame indexing (O(1) lookup)
    pub child_frame_index: HashMap<(usize, usize), usize>,
    pub frame_parent: HashMap<usize, usize>,
}
```

## Execution Modes

### `execute_frame(max_insns)`

Non-blocking frame execution for real-time game loops:

```rust
pub fn execute_frame(&mut self, max_insns: u64) -> Result<Option<Value>, String>
```

1. **Drain loaded frames** — Check for new scripts loaded by VM
2. **Execute frame** — Run up to `max_insns` instructions
3. **Handle result** — Continue/Return/Exit/Yield

### FrameAction Results

| Action | Meaning |
|--------|---------|
| `Continue` | Keep executing same frame |
| `Return(val)` | Pop frame, push result to caller |
| `Exit` | Program terminated |
| `Yield` | Pause for next frame (non-blocking) |

## Loading Scripts

### `load_module(filename, assets_path)`

```rust
pub fn load_module(&mut self, filename: &str, assets_path: &str) -> Result<usize, String>
```

1. Read `.adc` file from disk
2. Parse via `Loader::load()` → `CodeFrame`
3. Add to `ModuleRegistry` → get frame index
4. Execute root frame to register modules/functions
5. Return frame index

### `call_global(fn_name, args)`

```rust
pub fn call_global(&mut self, fn_name: &str, args: Vec<Value>) -> Result<Value, String>
```

1. Find function in `global_functions` map
2. Clone child frame
3. Add to registry with new frame index
4. Create `Frame` with locals for arguments
5. Push to call stack
6. Execute in non-blocking loop (30s timeout)

## Child Frame Indexing

O(1) lookup instead of O(n) search:

```rust
pub child_frame_index: HashMap<(parent_frame_idx, child_local_idx), usize>
pub frame_parent: HashMap<usize, usize>  // Reverse index
```

## Debug Features

| Flag | Purpose |
|------|---------|
| `trace` | Print every instruction |
| `log_native` | Log FFI calls |
| `max_instructions` | Safety limit (default 500M) |

## Execution Flow

```
main.rs run_cli()
  ├─ headless_boot() / run_headless_race()
  │   ├─ Load Application.adc via Loader::load()
  │   ├─ Create Engine(modules, natives)
  │   ├─ Register native APIs (pdistd, pdiext, gtengine, menu, pdiapp)
  │   ├─ specdb.load_all()
  │   ├─ Add root frame to ModuleRegistry
  │   └─ Engine.execute_frame()
  │
  └─ MainLoopState::tick()
      ├─ Drain loaded frames
      ├─ Execute VM frames (up to 5K insns)
      └─ Process state machine
```

## See Also

- [[20_ADHOC_VM/00_Index|Adhoc VM]]
- [[10_PC_Port/01_Documentation|Documentation]]
- [[10_PC_Port/00_Index|PC Port Index]]
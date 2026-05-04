---
tags: [vm, initialization, native, registry, engine]
type: documentation
project: GT PSP PC Port
section: Technical
---

# VM Initialization

> Detailed process of initializing the Rust Adhoc VM, native registry, and module loading.

## Overview

VM initialization happens in 3 phases:
1. **Native API registration** — Register 380+ native Rust functions
2. **Engine creation** — Create VM with modules + natives
3. **Module loading** — Load .adc bootstrap scripts

---

## Phase 1: Native API Registration

### Source Location

`pc_port/src/vm/native.rs`

### Data Structures

```rust
// Native function signature: takes Adhoc values, returns Adhoc value
pub type NativeFn = Rc<dyn Fn(&[Value]) -> Value>;

pub struct NativeRegistry {
    functions: HashMap<String, NativeFn>,  // Keyed by "module,class,method" path
    fallbacks: Vec<(String, NativeFn)>,      // Wildcard patterns
}
```

### Key Format

Native functions are keyed by **fully-qualified path**:

```
module,class,method
 Example: "main,pdistd,MRandom,GetValuei"
```

| Component | Example | Description |
|----------|---------|------------|
| `module` | `main` | Root module |
| `class` | `pdistd`, `gtengine` | Native module |
| `method` | `GetValuei`, `getCarSpec` | Method name |

### Registration Process

```rust
// main.rs
let mut natives = NativeRegistry::new();

// Register each module
engine::audio::register_audio(&mut natives);       // 18 functions
engine::pdistd::register_pdistd(&mut natives);      // 39 functions
engine::pdiext::register_pdiext(&mut natives);      // 58 functions
engine::gtengine::register_gtengine(&mut natives);  // 58 functions
engine::menu::register_menu(&mut natives);           // 146 functions
engine::pdiapp::register_pdiapp(&mut natives);     // 31 functions
```

### Native Module Breakdown

| Module | Functions | Purpose |
|--------|----------|---------|
| `pdistd` | 39 | File I/O, strings, math, XML, system |
| `pdiext` | 58 | Product info, font, sound, config |
| `gtengine` | 58 | SpecDB, car/track data, race logic |
| `menu` | 146 | UI widgets, menu navigation |
| `pdiapp` | 31 | App lifecycle |
| `audio` | 18 | Audio playback |
| **Total** | **350** | |

### Example: pdistd Registration

```rust
// engine/pdistd.rs
pub fn register_pdistd(natives: &mut NativeRegistry) {
    // MFileSystem
    natives.register("main,pdistd,MFileSystem,Open", |args| {
        // Opens file, returns handle
    });
    natives.register("main,pdistd,MFileSystem,Read", |args| {
        // Read bytes from handle
    });
    natives.register("main,pdistd,MFileSystem,Close", |args| {
        // Close file handle
    });
    
    // MString
    natives.register("main,pdistd,MString,Create", |args| {
        Value::String(Rc::new(String::new()))
    });
    natives.register("main,pdistd,MString,Format", |args| {
        // String formatting
    });
    
    // MRandom
    natives.register("main,pdistd,MRandom,GetValuei", |args| {
        Value::Int(rand::random::<i32>())
    });
}
```

### Fallback Registration

For methods that share common behavior:

```rust
natives.register_fallback("main,pdistd,MArray,", |args| {
    // Handle all MArray methods generically
});
```

---

## Phase 2: Engine Creation

### Source Location

`pc_port/src/vm/engine.rs`

### Data Structures

```rust
pub struct Engine {
    // Loaded module registry (frames with bytecode)
    pub modules: ModuleRegistry,
    
    // Native functions
    pub natives: NativeRegistry,
    
    // Static variables (global across all frames)
    pub static_storage: StaticStorage,
    
    // Current execution call stack
    pub call_stack: Vec<Frame>,
    
    // Module-level static scoping
    pub module_static_bases: HashMap<String, usize>,
    pub next_static_slot: usize,
    
    // Child frame indexing: O(1) lookup for nested functions
    pub child_frame_index: HashMap<(parent_frame_idx, child_local_idx), usize>,
    pub frame_parent: HashMap<usize, usize>,
    
    // Execution control
    pub max_instructions: u64,  // Default: 500M
    insn_count: u64,
}
```

### Engine Creation

```rust
pub fn new(modules: ModuleRegistry, natives: NativeRegistry) -> Self {
    // Pre-allocate static storage for all frames
    let static_count = modules.frames.iter()
        .map(|f| f.static_var_count)
        .sum::<i32>();
    let static_slots = if static_count > 0 { static_count as usize } else { 1 };
    
    Engine {
        modules,
        natives,
        static_storage: StaticStorage::new(static_slots as i32),
        call_stack: Vec::new(),
        module_static_bases: HashMap::new(),
        next_static_slot: 0,
        child_frame_index: HashMap::new(),
        frame_parent: HashMap::new(),
        max_instructions: 500_000_000,
        insn_count: 0,
        // ... other fields
    }
}
```

### Static Storage

Static variables are global across all frames:

```rust
pub struct StaticStorage {
    data: Vec<Value>,
}

impl StaticStorage {
    pub fn new(size: i32) -> Self {
        StaticStorage { data: vec![Value::Nil; size as usize] }
    }
    
    pub fn read(&self, idx: i32) -> Value {
        self.data.get(idx as usize).cloned().unwrap_or(Value::Nil)
    }
    
    pub fn write(&mut self, idx: i32, val: Value) {
        if idx as usize < self.data.len() {
            self.data[idx as usize] = val;
        }
    }
}
```

---

## Phase 3: Module Loading

### Load Sequence

```
Application.adc        → Root frame (registers modules)
bootstrap.adc       → Singleton initializers (execBoot())
bootstrap_phase2.adc → Secondary initialization (execBootPhase2())
MenuClassDefine.adc  → Widget class registration
packed_main_loop.adc  → Main loop definitions
```

### Bootstrap Execution

With the `IteratorNext` opcode implementation, the VM now properly executes the bootstrap sequence:

```rust
// Phase 1: Load and execute bootstrap.adc
eprintln!("[Game] Calling execBoot()...");
if let Some(func) = vm_engine.global_functions.get("execBoot") {
    let func_val = vm::value::Value::Function(func.clone());
    match vm_engine.call_function_value(func_val, vec![]) {
        Ok(_) => {
            match vm_engine.execute_frame(1000000) {
                Ok(Some(_)) => eprintln!("[Game] execBoot() completed successfully"),
                Ok(None) => eprintln!("[Game] execBoot() finished (no return value)"),
                Err(e) => eprintln!("[Game] execBoot() error: {}", e),
            }
        }
        Err(e) => eprintln!("[Game] execBoot() setup error: {}", e),
    }
    vm_engine.call_stack.clear();
}

// Phase 2: Load and execute bootstrap_phase2.adc
eprintln!("[Game] Calling execBootPhase2()...");
if let Some(func) = vm_engine.global_functions.get("execBootPhase2") {
    let func_val = vm::value::Value::Function(func.clone());
    match vm_engine.call_function_value(func_val, vec![]) {
        Ok(_) => {
            match vm_engine.execute_frame(1000000) {
                Ok(Some(_)) => eprintln!("[Game] execBootPhase2() completed successfully"),
                Ok(None) => eprintln!("[Game] execBootPhase2() finished (no return value)"),
                Err(e) => eprintln!("[Game] execBootPhase2() error: {}", e),
            }
        }
        Err(e) => eprintln!("[Game] execBootPhase2() setup error: {}", e),
    }
    vm_engine.call_stack.clear();
}
```

**What execBoot() does:**
- `initModules()` — Initialize all modules
- `initArgs()` — Process command-line args (previously caused infinite loop due to missing IteratorNext)
- `initSpecDB()` — Load SpecDB databases
- `initMenuSystem()` — Initialize MGOM (Menu Game Object Manager)
- `initNetwork()` — Initialize PDINetwork

**What execBootPhase2() does:**
- `initResidentProject()` — Load dialog project
- `initConfig()` — Load game configuration
- `initOrganizer()` — Initialize MOrganizer (race mode organizer)
- `initRaceOperator()` — Initialize MRaceOperator (race execution)
- `initSound()` — Initialize sound system (MSound)
- `GlobalStatus::initialize()` — Initialize global game state
- `initMemoryAssignment()` — Set memory allocation variables

### Loading Process

```rust
pub fn load_module(&mut self, filename: &str, assets_path: &str) -> Result<usize, String> {
    // 1. Build full path
    let path = if Path::new(filename).is_absolute() {
        filename.to_string()
    } else {
        format!("{}/{}", assets_path, filename)
    };
    
    // 2. Read bytecode
    let data = fs::read(&path)?;
    
    // 3. Parse with Loader
    let code_frame = Loader::load(&data)?;
    
    // 4. Add to registry
    let frame_idx = self.modules.add_frame(code_frame);
    
    // 5. Execute root frame (registers modules/functions)
    self.exec_root(frame_idx)?;
    
    Ok(frame_idx)
}
```

### Execution Mode

| Mode | Description | Use Case |
|------|------------|----------|
| `exec_root` | One-shot execution | Bootstrap (runs once) |
| `execute` | Blocking loop | Function calls |
| `execute_frame` | Non-blocking, per-frame | Render loop |

```rust
// Non-blocking: execute up to N instructions
pub fn execute_frame(&mut self, max_insns: u64) -> Result<Option<Value>, String> {
    let target = self.insn_count + max_insns;
    
    while self.insn_count < target {
        // Check for newly loaded frames
        let new_frames = loader::drain_loaded_frames();
        for cf in new_frames {
            let cf_idx = self.modules.add_frame(cf);
            self.call_stack.push(Frame::new(cf_idx, ...));
        }
        
        // Execute one frame
        match self.exec_frame(self.call_stack.len() - 1)? {
            FrameAction::Continue => continue,
            FrameAction::Return(val) => return Ok(Some(val)),
            FrameAction::Exit => return Ok(Some(Value::Void)),
            FrameAction::Yield => return Ok(None),
        }
    }
    
    Ok(None)
}
```

---

## Module Registration

When `Application.adc` executes (`exec_root`), it registers these modules:

### Modules Registered

```adhoc
module ::main {
    module pdistd;     // Standard library
    module pdiext;    // Extended library
    module gtengine;   // Game engine
    module menu;       // UI framework
    
    static manager;    // Widget manager
    static sound;      // Audio system
    static ORG;       // Race organizer
    static RaceOperator;
    
    module GameSequence {
        static current_sequence;
        static current_project;
        static next_sequence;
        static next_project;
    }
}
```

### Function Registration

During root execution, function definitions (`FunctionDefine` opcode) are registered:

```rust
Instruction::FunctionDefine { symbols } => {
    let name = symbols.last().cloned().unwrap_or_default();
    let path = self.module_stack.last().clone();
    
    // Create function value
    let fv = Rc::new(FunctionValue {
        module_path: path.clone(),
        name: name.clone(),
        code_frame: local_def_counter,
        is_method: false,
        static_base: frame.static_base,
        parent_frame: frame.frame_index,
    });
    
    // Register in global functions registry
    // Both qualified and unqualified names
    global_functions.insert(format!("{}::{}", path, name), fv.clone());
    global_functions.insert(name, fv);
    
    Ok(FrameAction::Continue)
}
```

---

## Child Frame Indexing

### Problem

When calling a nested function, we need O(1) lookup instead of O(n) search.

### Solution

HashMap from `(parent_frame_idx, child_local_idx)` to `child_frame_idx`:

```rust
pub struct Engine {
    // O(1) lookup for child frames
    pub child_frame_index: HashMap<(usize, usize), usize>,
    pub frame_parent: HashMap<usize, usize>,  // Reverse index
}

// On function define:
Instruction::FunctionDefine { symbols } => {
    let child_idx = frame.local_def_counter;
    frame.local_def_counter += 1;
    
    // Index: parent frame + local index → child frame
    child_frame_index.insert(
        (frame.frame_index, child_idx),
        child_frame_idx
    );
    
    // Reverse: child → parent
    frame_parent.insert(child_frame_idx, frame.frame_index);
    
    Ok(FrameAction::Continue)
}
```

### Lookup

```rust
pub fn get_child_frame(&self, parent: usize, local_idx: usize) -> Option<usize> {
    self.child_frame_index.get(&(parent, local_idx)).copied()
}
```

---

## Frame Actions

| Action | Meaning | Return from |
|--------|---------|------------|
| `Continue` | Keep executing | Same frame |
| `Return(val)` | Pop frame, push result | To caller |
| `Exit` | Program terminated | Top-level |
| `Yield` | Pause execution | Let caller resume |

```rust
pub enum FrameAction {
    Continue,
    Return(Value),
    Exit,
    Yield,  // For async/await
}
```

### Control Flow: SetState Opcode

```rust
Instruction::SetState { state } => {
    match state {
        0 => Ok(FrameAction::Continue),           // EXIT scope
        1 => {                                // RETURN
            let ret = pop(self, fi);
            return Ok(FrameAction::Return(ret));
        }
        2 => return Ok(FrameAction::Yield),       // YIELD
        _ => Ok(FrameAction::Continue),
    }
}
```

---

## Execution Flow Diagram

```
┌─────────────────────────────────────────────────────────────────┐
│ natives = NativeRegistry::new()                                │
│ natives.register("main,pdistd,...") × 200                    │
│ natives.register("main,pdiext,...") × 223                   │
│ natives.register("main,gtengine,...") × 694                  │
└──────────────────────────────────────���─���────────────────────────┘
                            ↓
┌─────────────────────────────────────────────────────────────────┐
│ modules = ModuleRegistry::new()                               │
│ modules.add_frame(Loader::load("Application.adc")?)           │
└─────────────────────────────────────────────────────────────────┘
                            ↓
┌─────────────────────────────────────────────────────────────────┐
│ engine = Engine::new(modules, natives)                       │
│ static_storage: StaticStorage::new(total_static_count)      │
│ child_frame_index: HashMap::new()                           │
└─────────────────────────────────────────────────────────────────┘
                            ↓
┌─────────────────────────────────────────────────────────────────┐
│ engine.load_module("Application.adc", assets_path)?           │
│     ↓ Loader::load() → code_frame                           │
│     ↓ modules.add_frame(code_frame) → frame_idx           │
│     ↓ engine.exec_root(frame_idx)                           │
│         ↓ Frame execute (all FunctionDefine opcodes)         │
│         ↓ Registers functions into global_functions          │
│         ↓ Registers modules into module_static_bases         │
└─────────────────────────────────────────────────────────────────┘
                            ↓
┌─────────────────────────────────────────────────────────────────┐
│ Main Loop:                                                     │
│     while running {                                           │
│         engine.execute_frame(5000)?                           │
│         drain_loaded_frames() → push new frames                │
│         state_machine.tick()                                   │
│         render()                                             │
│     }                                                       │
└─────────────────────────────────────────────────────────────────┘
```

---

## Optimization Notes

### Pre-allocation

1. **Static storage**: Sized at engine creation based on module totals
2. **Frame stacks**: Via `Vec::with_capacity(stack_size)`
3. **Child frame index**: HashMap sized for expected function count

### Caching

1. **Natives**: Direct HashMap lookup, O(1)
2. **Module statics**: HashMap lookup by path
3. **Child frames**: HashMap lookup by (parent, local_idx)

### Lazy vs Eager

| Approach | Used | Tradeoff |
|----------|------|---------|
| Eager | Static storage | Fast access, more memory |
| Lazy | Load on demand | Slower first access, less memory |

---

## See Also

- [[20_ADHOC_VM/01_VM_Engine|VM Engine]]
- [[20_ADHOC_VM/02_Bytecode_Loader|Bytecode Loader]]
- [[20_ADHOC_VM/05_Frames|Frames]]
- [[10_PC_Port/06_Native_API|Native API]]
- [[10_PC_Port/11_Menu_UI|Menu UI]] — Focus navigation system
---
tags: [pc-port, rust, vm, frame, call-stack]
type: documentation
project: GT PSP PC Port
section: Core Engine
---

# Call Frames & Stack

> Frame management and call stack (`pc_port/src/vm/frame.rs`).

## Frame Structure

```rust
pub struct Frame {
    pub locals: LocalStorage,           // Local variables
    pub instruction_ptr: i32,          // Current instruction index
    pub frame_index: usize,            // Frame index in registry
    pub stack: Vec<Value>,             // Operand stack
    pub return_value: Option<Value>,   // Return value
    pub state: FrameState,            // Running/Yielded/Returned/Exited
    pub try_catch_target: Option<i32>, // Exception handler
    pub static_base: usize,            // Static variable base offset
    pub local_def_counter: usize,     // Per-frame child frame index
    pub last_var_push_index: i32,    // For AssignPop write-back
    pub last_var_push_static: bool,
    pub switch_value: Option<Value>,  // Switch state
    pub switch_default: Option<i32>,  // Default jump target
    pub switch_remaining: usize,     // Cases left
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum FrameState {
    Running,
    Yielded,
    Returned,
    Exited,
}
```

## Frame Creation

```rust
impl Frame {
    pub fn new(frame_index: usize, stack_size: i32, local_count: i32, static_base: usize) -> Self {
        Frame {
            locals: LocalStorage::new(local_count),
            instruction_ptr: 0,
            frame_index,
            stack: Vec::with_capacity(stack_size as usize),
            return_value: None,
            state: FrameState::Running,
            try_catch_target: None,
            static_base,
            local_def_counter: 0,
            last_var_push_index: 0,
            last_var_push_static: false,
            switch_value: None,
            switch_default: None,
            switch_remaining: 0,
        }
    }
}
```

## Stack Operations

```rust
impl Frame {
    pub fn push(&mut self, val: Value) {
        self.stack.push(val);
    }
    
    pub fn pop(&mut self) -> Value {
        self.stack.pop().unwrap_or(Value::Nil)
    }
    
    pub fn peek(&self) -> Value {
        self.stack.last().cloned().unwrap_or(Value::Nil)
    }
}
```

## Call Stack

```rust
pub struct Engine {
    pub call_stack: Vec<Frame>,
    // ...
}
```

## Frame Actions

| Action | Meaning |
|--------|---------|
| `Continue` | Keep executing same frame |
| `Return(val)` | Pop frame, push result to caller |
| `Exit` | Program terminated |
| `Yield` | Pause for next frame |

## Child Frame Indexing

O(1) lookup for nested functions:

```rust
// Engine fields:
pub child_frame_index: HashMap<(parent_frame_idx, child_local_idx), usize>
pub frame_parent: HashMap<usize, usize>  // Reverse index
```

## Frame Lifecycle

1. **Created** — When function called or module loaded
2. **Running** — Execute instructions
3. **Yielded** — Paused for async (rare)
4. **Returned** — Function completed
5. **Popped** — Removed from call stack

## See Also

- [[20_ADHOC_VM/01_VM_Engine|VM Engine]]
- [[20_ADHOC_VM/03_Main_Loop|Main Loop]]
- [[10_PC_Port/00_Index|PC Port Index]]
use crate::vm::value::Value;
use crate::vm::storage::LocalStorage;

/// Runtime state for a single function/method call.
pub struct Frame {
    pub locals: LocalStorage,
    pub instruction_ptr: i32,
    pub frame_index: usize,
    pub stack: Vec<Value>,
    pub return_value: Option<Value>,
    pub state: FrameState,
    pub try_catch_target: Option<i32>,
    pub static_base: usize,
    pub local_def_counter: usize,  // per-frame child frame index
    pub last_var_push_index: i32,    // for AssignPop write-back
    pub last_var_push_static: bool,
    pub switch_value: Option<Value>,   // saved for consecutive Case matching
    pub switch_default: Option<i32>,   // fallback jump target for Switch
    pub switch_remaining: usize,        // cases left to check
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum FrameState {
    Running,
    Yielded,
    Returned,
    Exited,
}

impl Frame {
    pub fn new(frame_index: usize, stack_size: i32, local_count: i32, static_base: usize) -> Self {
        Frame {
            locals: LocalStorage::new(local_count),
            instruction_ptr: 0,
            frame_index,
            stack: Vec::with_capacity(stack_size.max(1) as usize),
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

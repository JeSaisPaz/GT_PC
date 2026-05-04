use crate::vm::value::Value;

/// Local variable storage for a single call frame.
#[derive(Clone)]
pub struct LocalStorage {
    cells: Vec<Value>,
}

impl LocalStorage {
    pub fn new(size: i32) -> Self {
        let n = if size > 0 { size as usize } else { 0 };
        LocalStorage { cells: vec![Value::Nil; n] }
    }

    pub fn read(&self, index: i32) -> Value {
        let idx = index as usize;
        if idx < self.cells.len() {
            self.cells[idx].clone()
        } else {
            Value::Nil
        }
    }

    pub fn write(&mut self, index: i32, val: Value) {
        let idx = index as usize;
        if idx < self.cells.len() {
            self.cells[idx] = val;
        } else if index >= 0 {
            eprintln!("[VM] LocalStorage OOB write: idx={} len={}", index, self.cells.len());
        }
    }

    pub fn size(&self) -> i32 {
        self.cells.len() as i32
    }
}

/// Module-level static variable storage.
#[derive(Clone)]
pub struct StaticStorage {
    cells: Vec<Value>,
}

impl StaticStorage {
    pub fn new(size: i32) -> Self {
        // Pre-allocate more slots than initially needed
        let min_size = size.max(0) as usize;
        let extra = 64; // Extra buffer for dynamic statics
        StaticStorage { cells: vec![Value::Nil; min_size + extra] }
    }

    pub fn read(&self, index: i32) -> Value {
        let idx = index as usize;
        self.cells.get(idx).cloned().unwrap_or(Value::Nil)
    }

    pub fn write(&mut self, index: i32, val: Value) {
        let idx = index as usize;
        // Auto-expand if needed
        if idx >= self.cells.len() {
            self.cells.resize(idx + 64, Value::Nil);
        }
        self.cells[idx] = val;
    }

    pub fn size(&self) -> i32 {
        self.cells.len() as i32
    }
}

/// Storage kind for variable access.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StorageKind {
    Local,
    Static,
}
use crate::vm::decoder::CodeFrame;
use crate::vm::storage::StaticStorage;
use std::collections::HashMap;
use std::sync::Arc;

/// A parsed module with its definitions and static storage.
#[derive(Clone)]
pub struct Module {
    pub name: String,
    pub full_path: String,
    pub classes: Vec<ClassDef>,
    pub functions: Vec<FuncDef>,
    pub statics: StaticStorage,
    pub child_modules: Vec<Module>,
}

#[derive(Clone)]
pub struct ClassDef {
    pub name: String,
    pub full_path: String,
    pub parent_path: String,
    pub attributes: Vec<AttrDef>,
    pub methods: Vec<FuncDef>,
}

#[derive(Clone)]
pub struct AttrDef {
    pub name: String,
    pub default_value_idx: usize,
}

#[derive(Clone)]
pub struct FuncDef {
    pub name: String,
    pub full_path: String,
    pub param_count: i32,
    pub code_frame_index: usize,
    pub is_method: bool,
    pub class_name: Option<String>,
}

/// Registry of all loaded modules.
pub struct ModuleRegistry {
    pub modules: Vec<Module>,
    pub frames: Vec<Arc<CodeFrame>>,
    pub name_to_index: HashMap<String, usize>,
    pub path_to_frame: HashMap<String, usize>,
}

impl ModuleRegistry {
    pub fn new() -> Self {
        ModuleRegistry {
            modules: Vec::new(),
            frames: Vec::new(),
            name_to_index: HashMap::new(),
            path_to_frame: HashMap::new(),
        }
    }

    pub fn add_frame(&mut self, frame: Arc<CodeFrame>) -> usize {
        let idx = self.frames.len();
        self.frames.push(frame);
        idx
    }

    pub fn get_frame(&self, idx: usize) -> &CodeFrame {
        self.frames.get(idx).unwrap_or_else(|| {
            eprintln!("[VM] get_frame OOB: idx={} len={}", idx, self.frames.len());
            &self.frames[0]
        })
    }
}

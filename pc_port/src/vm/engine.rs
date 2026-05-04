use crate::vm::decoder::*;
use crate::vm::frame::*;
use crate::vm::module::*;
use crate::vm::native::NativeRegistry;
use crate::vm::storage::*;
use crate::vm::value::*;
use std::rc::Rc;
use std::collections::HashMap;

/// The Adhoc bytecode execution engine.
pub struct Engine {
    pub modules: ModuleRegistry,
    pub natives: NativeRegistry,
    pub static_storage: StaticStorage,
    pub call_stack: Vec<Frame>,
    pub current_module_index: usize,
    pub module_stack: Vec<String>,
    pub def_counter: usize,
    pub func_frame_map: Vec<usize>,
    pub loader: Option<Rc<dyn Fn(&str) -> Result<i32, String>>>,
    pub global_functions: std::collections::HashMap<String, Rc<FunctionValue>>,
    pub trace: bool,
    pub log_native: bool,
    pub max_instructions: u64,
    insn_count: u64,
    // Module-level static scoping: module path → base offset in flat static array
    pub module_static_bases: std::collections::HashMap<String, usize>,
    pub next_static_slot: usize,
    // Child frame index: (parent_frame_idx, child_local_idx) → child_frame_idx
    // Enables O(1) lookup instead of O(n) search across all frames
    pub child_frame_index: HashMap<(usize, usize), usize>,
    // Reverse index: child_frame_idx → parent_frame_idx (for lookups)
    pub frame_parent: HashMap<usize, usize>,
}

impl Engine {
    pub fn new(modules: ModuleRegistry, natives: NativeRegistry) -> Self {
        let static_count = modules.frames.iter().map(|f| f.static_var_count).sum::<i32>();
        let static_slots = if static_count > 0 { static_count as usize } else { 1 };
        Engine {
            modules,
            natives,
            static_storage: StaticStorage::new(static_slots as i32),
            call_stack: Vec::new(),
            current_module_index: 0,
            module_stack: Vec::new(),
            def_counter: 0,
            func_frame_map: Vec::new(),
            loader: None,
            global_functions: std::collections::HashMap::new(),
            trace: false,
            log_native: false,
            max_instructions: 500_000_000,
            insn_count: 0,
            module_static_bases: std::collections::HashMap::new(),
            next_static_slot: 0,
            child_frame_index: HashMap::new(),
            frame_parent: HashMap::new(),
        }
    }

    pub fn get_insn_count(&self) -> u64 {
        self.insn_count
    }

    /// Execute a limited number of instructions (for frame-by-frame execution).
    /// Returns Ok(()) if stopped due to instruction limit, or the final result if program terminated.
    pub fn execute_frame(&mut self, max_insns: u64) -> Result<Option<Value>, String> {
        let start_count = self.insn_count;
        let target_count = start_count + max_insns;

        // If we have no call stack, there's nothing to execute
        if self.call_stack.is_empty() {
            return Ok(None);
        }

        // Execute instructions until we hit our limit or the program terminates
        while self.insn_count < target_count {
            // Check for newly loaded frames (from Module::load native call)
            let new_frames = crate::vm::loader::drain_loaded_frames();
            for cf in new_frames {
                let cf_idx = self.modules.add_frame(cf);
                let fi = self.modules.get_frame(cf_idx);
                // static_base = current total allocated statics
                let new_static_base = self.current_module_index;
                self.current_module_index += fi.static_var_count.max(1) as usize;
                let mut ef = Frame::new(cf_idx, fi.stack_size, fi.local_var_count, new_static_base);
                self.call_stack.push(ef);
            }

            // Execute one frame (which may terminate)
            let frame_result = self.exec_frame(self.call_stack.len() - 1)?;
            
            // Check if we should continue
            match frame_result {
                FrameAction::Continue => {
                    // Continue executing instructions in the same frame
                    continue;
                }
                FrameAction::Return(val) => {
                    self.call_stack.pop();
                    if self.call_stack.is_empty() {
                        return Ok(Some(val));
                    }
                    if let Some(caller) = self.call_stack.last_mut() {
                        caller.push(val);
                    }
                    continue;
                }
                FrameAction::Exit => {
                    self.call_stack.pop();
                    if self.call_stack.is_empty() {
                        return Ok(Some(Value::Void));
                    }
                    continue;
                }
                FrameAction::Yield => {
                    return Ok(None);
                }
            }
        }

        Ok(None)
    }

    /// Load an .adc file from disk, add it to the module registry,
    /// and execute its root frame (which registers modules, functions, statics).
    /// Returns the frame index for later function calls.
    pub fn load_module(&mut self, filename: &str, assets_path: &str) -> Result<usize, String> {
        let path = if std::path::Path::new(filename).is_absolute() {
            filename.to_string()
        } else {
            let sep = if cfg!(windows) { "\\" } else { "/" };
            format!("{}{}{}", assets_path, sep, filename)
        };
        let data = std::fs::read(&path).map_err(|e| format!("Read {}: {}", path, e))?;
        let code_frame = crate::vm::loader::Loader::load(&data)?;
        let frame_idx = self.modules.add_frame(code_frame);
        // Execute root frame to register modules/functions
        self.exec_root(frame_idx)?;
        Ok(frame_idx)
    }

    /// Call a named global function loaded from a module.
    /// Uses execute_frame in a non-blocking loop.
    pub fn call_global(&mut self, fn_name: &str, args: Vec<Value>) -> Result<Value, String> {
        let fv = self.global_functions.get(fn_name)
            .cloned()
            .or_else(|| {
                let name_from_path = format!("main::{}", fn_name);
                self.global_functions.get(&name_from_path).cloned()
            })
            .ok_or_else(|| format!("Function '{}' not found", fn_name))?;

        let child_clone = {
            // fv.code_frame is an absolute index into self.modules.frames
            let fi = fv.code_frame as usize;
            if fi < self.modules.frames.len() {
                self.modules.frames[fi].clone()
            } else {
                // Fallback: search child frames of all root frames
                let mut found = None;
                for frame in &self.modules.frames {
                    if fi < frame.child_frames.len() {
                        found = Some(frame.child_frames[fi].clone());
                        break;
                    }
                }
                found.ok_or_else(|| format!("Child frame {} not found for '{}'", fv.code_frame, fn_name))?
            }
        };

        let child_frame_idx = self.modules.add_frame(child_clone.clone());
        let mut new_frame = crate::vm::frame::Frame::new(
            child_frame_idx,
            child_clone.stack_size,
            child_clone.local_var_count,
            fv.static_base,
        );
        for (i, arg) in args.into_iter().enumerate() {
            new_frame.locals.write(i as i32, arg);
        }
        self.call_stack.push(new_frame);

        // Non-blocking loop: execute_frame processes up to 10K insns per call
        let start = std::time::Instant::now();
        let timeout = std::time::Duration::from_secs(30);
        loop {
            let new_frames = crate::vm::loader::drain_loaded_frames();
            for cf in new_frames {
                let cf_idx = self.modules.add_frame(cf);
                let fi = self.modules.get_frame(cf_idx);
                let ef = Frame::new(cf_idx, fi.stack_size, fi.local_var_count, 0);
                self.call_stack.push(ef);
            }

            if self.call_stack.is_empty() {
                return Ok(Value::Void);
            }

            match self.execute_frame(10000)? {
                Some(val) => {
                    self.call_stack.clear();
                    return Ok(val);
                }
                None => {}
            }

            if start.elapsed() > timeout {
                self.call_stack.clear();
                return Err("Timeout in call_global".to_string());
            }
        }
    }

    pub fn execute(&mut self, frame_index: usize, args: Vec<Value>) -> Result<Value, String> {
        let frame_info = self.modules.get_frame(frame_index);
        let mut frame = Frame::new(frame_index, frame_info.stack_size,
            frame_info.local_var_count, 0);
        for (i, arg) in args.into_iter().enumerate() {
            frame.locals.write(i as i32, arg);
        }
        if !self.call_stack.is_empty() {
            frame.static_base = self.current_module_index;
        }
        self.call_stack.push(frame);

        let mut safety_counter = 0u64;
        loop {
            safety_counter += 1;
            if safety_counter > 1_000_000 {
                return Err("Infinite loop detected (>1M frame iterations)".to_string());
            }
            // Check for newly loaded frames (from Module::load native call)
            let new_frames = crate::vm::loader::drain_loaded_frames();
            for cf in new_frames {
                let cf_idx = self.modules.add_frame(cf);
                // Push onto call stack to execute (defines modules/functions then returns)
                let fi = self.modules.get_frame(cf_idx);
                let mut ef = Frame::new(cf_idx, fi.stack_size, fi.local_var_count, 0);
                self.call_stack.push(ef);
            }

            let top_idx = self.call_stack.len() - 1;
            let result = self.exec_frame(top_idx)?;
            match result {
                FrameAction::Continue => continue,
                FrameAction::Return(val) => {
                    self.call_stack.pop();
                    if self.call_stack.is_empty() {
                        return Ok(val);
                    }
                    if let Some(caller) = self.call_stack.last_mut() {
                        caller.push(val);
                    }
                }
                FrameAction::Exit => {
                    self.call_stack.pop();
                    if self.call_stack.is_empty() {
                        return Ok(Value::Void);
                    }
                }
                FrameAction::Yield => {
                    // In blocking mode, continue execution after yield
                    continue;
                }
            }
        }
    }

    // ─── Initial root-code execution ───────────────────────────
    // Called once to run the top-level instructions that set up modules.
    pub fn exec_root(&mut self, frame_index: usize) -> Result<Value, String> {
        // Reset counters for root execution
        self.def_counter = 0;
        self.func_frame_map.clear();
        self.module_stack.clear();

        let frame_info = self.modules.get_frame(frame_index);
        let frame = Frame::new(frame_index, frame_info.stack_size,
            frame_info.local_var_count, 0);
        self.call_stack.push(frame);

        let mut safety_counter = 0u64;
        loop {
            safety_counter += 1;
            if safety_counter > 1_000_000 {
                return Err("Infinite loop detected (>1M root iterations)".to_string());
            }
            let top_idx = self.call_stack.len() - 1;
            let result = self.exec_frame(top_idx)?;
            match result {
                FrameAction::Continue => continue,
                FrameAction::Return(val) => {
                    self.call_stack.pop();
                    if self.call_stack.is_empty() {
                        return Ok(val);
                    }
                    if let Some(caller) = self.call_stack.last_mut() {
                        caller.push(val);
                    }
                }
                FrameAction::Exit => {
                    self.call_stack.pop();
                    if self.call_stack.is_empty() {
                        return Ok(Value::Void);
                    }
                }
                FrameAction::Yield => {
                    continue;
                }
            }
        }
    }

    fn exec_frame(&mut self, frame_idx: usize) -> Result<FrameAction, String> {
        
        let ip = self.call_stack[frame_idx].instruction_ptr as usize;
        let (insn, line) = {
            let f = &self.call_stack[frame_idx];
            let info = self.modules.get_frame(f.frame_index);
            if ip >= info.instructions.len() {
                return Ok(FrameAction::Exit);
            }
            let line = info.instruction_lines.get(ip).copied().unwrap_or(0);
            (info.instructions[ip].clone(), line)
        };

        if self.insn_count >= self.max_instructions {
            return Err(format!("Hit instruction limit ({})", self.max_instructions));
        }
        self.insn_count += 1;

        // Default: advance to next instruction. Jump/SetState can override.
        self.call_stack[frame_idx].instruction_ptr = (ip + 1) as i32;



        if self.trace {
            eprintln!("  [{:04}] L{} {:?}", ip, line, insn);
        }

        match self.exec_insn(frame_idx, &insn) {
            Ok(FrameAction::Continue) => Ok(FrameAction::Continue),
            Ok(other) => Ok(other),
                Err(e) => {
                    let should_catch = self.call_stack.len() > frame_idx && self.call_stack[frame_idx].try_catch_target.is_some();
                    if should_catch {
                        let target = self.call_stack[frame_idx].try_catch_target.unwrap();
                    self.call_stack[frame_idx].instruction_ptr = target;
                    self.call_stack[frame_idx].push(Value::String(Rc::new(e)));
                    self.call_stack[frame_idx].try_catch_target = None;
                    Ok(FrameAction::Continue)
                } else {
                    Err(e)
                }
            }
        }
    }

    fn exec_insn(&mut self, fi: usize, insn: &Instruction) -> Result<FrameAction, String> {
        // ─── Helper accessors ────────────────────────────────────
        fn push(me: &mut Engine, fi: usize, v: Value) {
            me.call_stack[fi].push(v);
        }
        fn pop(me: &mut Engine, fi: usize) -> Value {
            me.call_stack[fi].pop()
        }
        fn peek(me: &Engine, fi: usize) -> Value {
            me.call_stack[fi].peek()
        }
        fn local_read(me: &Engine, fi: usize, idx: i32) -> Value {
            me.call_stack[fi].locals.read(idx)
        }
        fn local_write(me: &mut Engine, fi: usize, idx: i32, v: Value) {
            me.call_stack[fi].locals.write(idx, v);
        }
        fn static_read(me: &Engine, fi: usize, idx: i32) -> Value {
            let sb = me.call_stack[fi].static_base as i32;
            me.static_storage.read(sb + idx)
        }
        fn static_write(me: &mut Engine, fi: usize, idx: i32, v: Value) {
            let sb = me.call_stack[fi].static_base as i32;
            me.static_storage.write(sb + idx, v);
        }

        match insn {
            // ─── Constants ──────────────────────────────────────
            Instruction::NilConst => { push(self, fi, Value::Nil); Ok(FrameAction::Continue) }
            Instruction::VoidConst => { push(self, fi, Value::Void); Ok(FrameAction::Continue) }
            Instruction::BoolConst { value } => { push(self, fi, Value::Bool(*value)); Ok(FrameAction::Continue) }
            Instruction::IntConst { value } => { push(self, fi, Value::Int(*value)); Ok(FrameAction::Continue) }
            Instruction::UIntConst { value } => { push(self, fi, Value::UInt(*value)); Ok(FrameAction::Continue) }
            Instruction::FloatConst { value } => { push(self, fi, Value::Float(*value)); Ok(FrameAction::Continue) }
            Instruction::DoubleConst { value } => { push(self, fi, Value::Double(*value)); Ok(FrameAction::Continue) }
            Instruction::LongConst { value } => { push(self, fi, Value::Long(*value)); Ok(FrameAction::Continue) }
            Instruction::StringConst { symbols } => {
                push(self, fi, Value::String(Rc::new(symbols.last().cloned().unwrap_or_default())));
                Ok(FrameAction::Continue)
            }
            Instruction::SymbolConst { symbol } => {
                push(self, fi, Value::Symbol(Rc::new(symbol.clone())));
                Ok(FrameAction::Continue)
            }

            // ─── Variables ──────────────────────────────────────
            Instruction::VariablePush { symbols, kind, index } => {
                let is_static = matches!(kind, StorageKind::Static);
                let v = if is_static {
                    // Try full-path lookup (last symbol), then first-symbol lookup
                    let full_path = symbols.last().cloned().unwrap_or_default();
                    let abs_pos = self.module_static_bases.get(&full_path).copied()
                        .or_else(|| symbols.first().and_then(|s| self.module_static_bases.get(s.as_str()).copied()));
                    if let Some(abs) = abs_pos {
                        self.static_storage.read(abs as i32)
                    } else {
                        self.static_storage.read(*index)
                    }
                } else {
                    local_read(self, fi, *index)
                };
                push(self, fi, v);
                // Track for AssignPop write-back (store absolute position)
                let abs_pos = if is_static {
                    let full_path = symbols.last().cloned().unwrap_or_default();
                    self.module_static_bases.get(&full_path).copied()
                        .or_else(|| symbols.first().and_then(|s| self.module_static_bases.get(s.as_str()).copied()))
                        .unwrap_or(*index as usize) as i32
                } else { *index };
                self.call_stack[fi].last_var_push_index = abs_pos;
                self.call_stack[fi].last_var_push_static = is_static;
                Ok(FrameAction::Continue)
            }
            Instruction::VariableEval { symbols, kind, index } => {
                let mut v = if matches!(kind, StorageKind::Static) {
                    let full_path = symbols.last().cloned().unwrap_or_default();
                    let abs_pos = self.module_static_bases.get(&full_path).copied()
                        .or_else(|| symbols.first().and_then(|s| self.module_static_bases.get(s.as_str()).copied()));
                    if let Some(abs) = abs_pos {
                        self.static_storage.read(abs as i32)
                    } else {
                        self.static_storage.read(*index)
                    }
                } else {
                    local_read(self, fi, *index)
                };
                if let Some(sym) = symbols.first() {
                    let sym = sym.as_str();
                    if self.global_functions.contains_key(sym) {
                        if let Some(fv) = self.global_functions.get(sym) {
                            v = Value::Function(fv.clone());
                        }
                    } else if matches!(v, Value::Nil) && sym == "__module__" {
                        if let Some(path) = self.module_stack.last() {
                            v = Value::Object(Rc::new(ObjectInstance {
                                class_path: path.clone(), fields: vec![],
                            }));
                        }
                    } else if matches!(v, Value::Nil) {
                        if let Some(pos) = self.module_stack.iter().position(|m| m.contains(sym)) {
                            let path = self.module_stack[pos].clone();
                            v = Value::Object(Rc::new(ObjectInstance { class_path: path, fields: vec![] }));
                        } else {
                            for prefix in &["main,pdiext,", "main,pdistd,", "main,gtengine,", "main,menu,", "main,pdiapp,"] {
                                let native_path = format!("{}{}", prefix, sym);
                                if self.natives.has(&native_path) {
                                    v = Value::Object(Rc::new(ObjectInstance { class_path: native_path, fields: vec![] }));
                                    break;
                                }
                            }
                        }
                    }
                }
                push(self, fi, v);
                Ok(FrameAction::Continue)
            }
            Instruction::StaticDefine { symbols } => {
                // Map variable to absolute static position (with module scope)
                let name = symbols.last().cloned().unwrap_or_default();
                self.module_static_bases.insert(name.clone(), self.next_static_slot);
                // Also store with module-qualified name (short form)
                if let Some(mod_path) = self.module_stack.last() {
                    let short_mod = mod_path.split(',').last().unwrap_or(mod_path);
                    let qualified = format!("{}::{}", short_mod, name);
                    self.module_static_bases.insert(qualified, self.next_static_slot);
                }
                self.next_static_slot += 1;
                push(self, fi, Value::Nil);
                Ok(FrameAction::Continue)
            }
            Instruction::AttributeDefine { symbols } => {
                let default = pop(self, fi);
                push(self, fi, Value::Nil);
                Ok(FrameAction::Continue)
            }

            // ─── Assignment ─────────────────────────────────────
            Instruction::AssignPop => {
                let val = pop(self, fi);
                let _target = pop(self, fi);
                // Write-back to storage slot tracked by VariablePush
                let idx = self.call_stack[fi].last_var_push_index;
                if self.call_stack[fi].last_var_push_static {
                    self.static_storage.write(idx, val.clone());
                }
                push(self, fi, val);
                Ok(FrameAction::Continue)
            }
            Instruction::Assign { symbols: _, storage_index } => {
                // V12: same behavior as AssignPop — opcode 0x2C pops value
                // and writes to the target tracked by VariablePush's last_var_push_index.
                let val = pop(self, fi);
                let _old_target = pop(self, fi);
                let idx = self.call_stack[fi].last_var_push_index;
                if self.call_stack[fi].last_var_push_static {
                    self.static_storage.write(idx, val.clone());
                } else {
                    self.call_stack[fi].locals.write(idx, val.clone());
                }
                push(self, fi, val);
                Ok(FrameAction::Continue)
            }
            Instruction::AssignOld { symbols: _, index } => {
                let val = pop(self, fi);
                local_write(self, fi, *index, val.clone());
                push(self, fi, val);
                Ok(FrameAction::Continue)
            }
            Instruction::BinaryAssignOperator { op: _, symbols: _ } => {
                let val = pop(self, fi);
                push(self, fi, val);
                Ok(FrameAction::Continue)
            }

            // ─── Arrays / Maps / Strings ──────────────────────
            Instruction::ArrayConst { count } | Instruction::ArrayConstOld { count } => {
                let n = *count as usize;
                let mut elems: Vec<Value> = Vec::with_capacity(n);
                for _ in 0..n {
                    elems.push(pop(self, fi));
                }
                elems.reverse();
                push(self, fi, Value::Array(Rc::new(elems)));
                Ok(FrameAction::Continue)
            }
            Instruction::ArrayPush => {
                let val = pop(self, fi);
                let arr = pop(self, fi);
                match arr {
                    Value::Array(a) => {
                        let mut na = a.as_ref().clone();
                        na.push(val);
                        push(self, fi, Value::Array(Rc::new(na)));
                    }
                    _ => { push(self, fi, arr); }
                }
                Ok(FrameAction::Continue)
            }
            Instruction::MapConst => {
                push(self, fi, Value::Map(Rc::new(vec![])));
                Ok(FrameAction::Continue)
            }
            Instruction::MapConstOld => {
                push(self, fi, Value::Map(Rc::new(vec![])));
                Ok(FrameAction::Continue)
            }
            Instruction::MapInsert => {
                let val = pop(self, fi);
                let key = pop(self, fi);
                let map = pop(self, fi);
                match map {
                    Value::Map(m) => {
                        let mut nm = m.as_ref().clone();
                        nm.push((key, val));
                        push(self, fi, Value::Map(Rc::new(nm)));
                    }
                    _ => push(self, fi, map),
                }
                Ok(FrameAction::Continue)
            }
            Instruction::StringPush { count } => {
                let n = *count as usize;
                let mut parts: Vec<String> = Vec::with_capacity(n);
                for _ in 0..n {
                    let v = pop(self, fi);
                    parts.push(v.to_string());
                }
                parts.reverse();
                push(self, fi, Value::String(Rc::new(parts.concat())));
                Ok(FrameAction::Continue)
            }

            // ─── Stack Ops ──────────────────────────────────────
            Instruction::Pop | Instruction::PopOld => {
                pop(self, fi);
                Ok(FrameAction::Continue)
            }
            Instruction::Eval => {
                let v = pop(self, fi);
                push(self, fi, v);
                Ok(FrameAction::Continue)
            }
            Instruction::ObjectSelector { symbol: _ } => {
                let key = pop(self, fi);
                let obj = pop(self, fi);
                match obj {
                    Value::Array(a) => {
                        let idx = key.as_i32().unwrap_or(0) as usize;
                        if idx < a.len() { push(self, fi, a[idx].clone()); }
                        else { push(self, fi, Value::Nil); }
                    }
                    Value::Map(m) => {
                        let mut found = Value::Nil;
                        for (k, v) in m.iter() {
                            if values_equal(k, &key) { found = v.clone(); break; }
                        }
                        push(self, fi, found);
                    }
                    _ => push(self, fi, Value::Nil),
                }
                Ok(FrameAction::Continue)
            }
            Instruction::ElementEval => {
                let key = pop(self, fi);
                let obj = pop(self, fi);
                match obj {
                    Value::Array(a) => {
                        let idx = key.as_i32().unwrap_or(0) as usize;
                        if idx < a.len() { push(self, fi, a[idx].clone()); }
                        else { push(self, fi, Value::Nil); }
                    }
                    Value::Map(m) => {
                        let mut found = Value::Nil;
                        for (k, v) in m.iter() {
                            if values_equal(k, &key) { found = v.clone(); break; }
                        }
                        push(self, fi, found);
                    }
                    _ => push(self, fi, Value::Nil),
                }
                Ok(FrameAction::Continue)
            }
            Instruction::ElementPush => {
                let val = pop(self, fi);
                let key = pop(self, fi);
                let obj = pop(self, fi);
                match obj {
                    Value::Array(a) => {
                        let mut na = a.as_ref().clone(); na.push(val);
                        push(self, fi, Value::Array(Rc::new(na)));
                    }
                    Value::Map(m) => {
                        let mut nm = m.as_ref().clone(); nm.push((key, val));
                        push(self, fi, Value::Map(Rc::new(nm)));
                    }
                    _ => push(self, fi, obj),
                }
                Ok(FrameAction::Continue)
            }
            Instruction::LocalDefine => Ok(FrameAction::Continue),

            // ─── Unary ──────────────────────────────────────────
            Instruction::UnaryOperator { op } => {
                let a = pop(self, fi);
                push(self, fi, match op.as_str() {
                    "-@" | "__uminus__" => match a {
                        Value::Int(i) => Value::Int(-i),
                        Value::Float(f) => Value::Float(-f),
                        _ => Value::Int(0),
                    },
                    "!" | "__not__" => Value::Bool(!a.truthy()),
                    "~" | "__invert__" => match a {
                        Value::Int(i) => Value::Int(!i),
                        _ => Value::Int(0),
                    },
                    _ => a,
                });
                Ok(FrameAction::Continue)
            }

            // ─── Binary ─────────────────────────────────────────
            Instruction::BinaryOperator { op } => {
                let b = pop(self, fi); let a = pop(self, fi);
                push(self, fi, binary_op(&a, &b, op));
                Ok(FrameAction::Continue)
            }

            // ─── Control Flow ───────────────────────────────────
            Instruction::LogicalAnd { target } | Instruction::LogicalAndOld { target } => {
                let v = pop(self, fi);  // V12: pops left operand itself
                if !v.truthy() {
                    push(self, fi, v);  // Restore left operand as result (short-circuit)
                    self.call_stack[fi].instruction_ptr = *target;
                }
                Ok(FrameAction::Continue)
            }
            Instruction::LogicalOr { target } | Instruction::LogicalOrOld { target } => {
                let v = pop(self, fi);
                if v.truthy() {
                    push(self, fi, v);  // Restore left operand as result
                    self.call_stack[fi].instruction_ptr = *target;
                }
                Ok(FrameAction::Continue)
            }

            Instruction::Jump { target } => {
                self.call_stack[fi].instruction_ptr = *target;
                return Ok(FrameAction::Continue);
            }
            Instruction::JumpIfFalse { target } => {
                let v = pop(self, fi);
                if !v.truthy() {
                    self.call_stack[fi].instruction_ptr = *target;
                }
                Ok(FrameAction::Continue)
            }
            Instruction::JumpIfTrue { target } => {
                if pop(self, fi).truthy() {
                    self.call_stack[fi].instruction_ptr = *target;
                }
                Ok(FrameAction::Continue)
            }
            Instruction::JumpIfNil { target } => {
                if matches!(pop(self, fi), Value::Nil) {
                    self.call_stack[fi].instruction_ptr = *target;
                }
                Ok(FrameAction::Continue)
            }
            Instruction::SetState { state } => {
                match state {
                    0 => { // EXIT scope/module - just continue to next insn
                        Ok(FrameAction::Continue)
                    }
                    1 => {
                        let ret = pop(self, fi);
                        self.call_stack[fi].state = FrameState::Returned;
                        return Ok(FrameAction::Return(ret));
                    }
                    2 => { // YIELD — pause execution, let caller resume
                        // Save current state and return to caller
                        self.call_stack[fi].state = FrameState::Yielded;
                        return Ok(FrameAction::Yield);
                    }
                    _ => Ok(FrameAction::Continue),
                }
            }
            Instruction::SetStateOld => {
                Ok(FrameAction::Continue)
            }
            Instruction::IteratorNext { target, var_index } => {
                // IteratorNext: implements foreach loop iteration
                // Stack: [array] -> after: [array] or empty if done
                // If array has elements: store first in local var at var_index, push rest, continue
                // If empty: pop array and jump to target
                let arr_val = pop(self, fi);
                match arr_val {
                    Value::Array(arr) => {
                        let arr_ref = arr.clone();
                        if arr_ref.is_empty() {
                            // Done iterating - jump to target
                            self.call_stack[fi].instruction_ptr = *target;
                        } else {
                            // Get first element, store in local variable
                            let next_val = arr_ref[0].clone();
                            local_write(self, fi, *var_index, next_val);
                            // Push remaining elements back
                            let remaining: Vec<Value> = arr_ref.iter().skip(1).cloned().collect();
                            push(self, fi, Value::Array(Rc::new(remaining)));
                        }
                    }
                    _ => {
                        // Not an array - treat as done
                        self.call_stack[fi].instruction_ptr = *target;
                    }
                }
                Ok(FrameAction::Continue)
            }
            Instruction::Switch { default_target, case_count } => {
                let val = pop(self, fi);
                // Save switch value and count for Case instructions to match against
                self.call_stack[fi].switch_value = Some(val);
                self.call_stack[fi].switch_default = Some(*default_target);
                self.call_stack[fi].switch_remaining = *case_count as usize;
                // instruction_ptr already advanced to next (first Case) by exec_frame
                Ok(FrameAction::Continue)
            }
            Instruction::Case { value, target } => {
                let matched = self.call_stack[fi].switch_value.as_ref()
                    .and_then(|sv| sv.as_i32())
                    .map(|sv| sv == *value)
                    .unwrap_or(false);
                if matched {
                    self.call_stack[fi].instruction_ptr = *target;
                    self.call_stack[fi].switch_value = None;
                    self.call_stack[fi].switch_remaining = 0;
                } else {
                    self.call_stack[fi].switch_remaining -= 1;
                    if self.call_stack[fi].switch_remaining == 0 {
                        // No case matched — jump to default
                        if let Some(default) = self.call_stack[fi].switch_default {
                            self.call_stack[fi].instruction_ptr = default;
                        }
                        self.call_stack[fi].switch_value = None;
                    }
                }
                Ok(FrameAction::Continue)
            }
            Instruction::JumpTable { default_target: _ } => Ok(FrameAction::Continue),

            // ─── TailCall — tail call optimization ──────────────
            Instruction::TailCall => {
                let n = 0; // args are already on stack from prior instructions
                let args: Vec<Value> = (0..n).map(|_| pop(self, fi)).collect();
                let func_val = pop(self, fi);
                let result = self.call_value(func_val, args, fi)?;
                push(self, fi, result);
                Ok(FrameAction::Continue)
            }

            // ─── JumpUndefined / JumpDefined ────────────────────
            Instruction::JumpUndefined { target } => {
                if matches!(peek(self, fi), Value::Nil | Value::Void) {
                    self.call_stack[fi].instruction_ptr = *target;
                }
                Ok(FrameAction::Continue)
            }
            Instruction::JumpDefined { target } => {
                if !matches!(peek(self, fi), Value::Nil | Value::Void) {
                    self.call_stack[fi].instruction_ptr = *target;
                }
                Ok(FrameAction::Continue)
            }

            // ─── Modern features (not used in GT PSP, safe to skip) ──
            Instruction::AsyncFunction | Instruction::Await | Instruction::Yield
                | Instruction::Generator | Instruction::YieldFrom | Instruction::await_next
                | Instruction::ReturnToYield => Ok(FrameAction::Continue),
            Instruction::Super | Instruction::SuperCall => Ok(FrameAction::Continue),
            Instruction::NewTarget | Instruction::Spread => Ok(FrameAction::Continue),
            Instruction::ArrowFunction => Ok(FrameAction::Continue),
            Instruction::RestParameter | Instruction::OptionalParameter => Ok(FrameAction::Continue),
            Instruction::FastAttribute => Ok(FrameAction::Continue),
            Instruction::BigIntConst => { push(self, fi, Value::Int(0)); Ok(FrameAction::Continue) }
            Instruction::TemplateConst => { push(self, fi, Value::String(Rc::new(String::new()))); Ok(FrameAction::Continue) }
            Instruction::ImportMeta | Instruction::DynamicImport => Ok(FrameAction::Continue),
            Instruction::LogicalNullish { target: _ } | Instruction::NullishCoalescing { target: _ } => Ok(FrameAction::Continue),
            Instruction::OptionalChaining => Ok(FrameAction::Continue),

            // ─── Default ────────────────────────────────────────
            _ => Ok(FrameAction::Continue),
        }
    }

    /// Print current call stack for debugging.
    pub fn dump_stack(&self) {
        eprintln!("=== Call Stack ({} frames) ===", self.call_stack.len());
        for (i, f) in self.call_stack.iter().enumerate() {
            let info = self.modules.get_frame(f.frame_index);
            let ip = f.instruction_ptr;
            let insn_desc = if (ip as usize) < info.instructions.len() {
                format!("{:?}", &info.instructions[ip as usize])
            } else {
                "end".to_string()
            };
            eprintln!("  [{}] frame={} ip={} stack={} {}", i, f.frame_index, ip, f.stack.len(), insn_desc);
        }
    }

    fn call_value(&mut self, func: Value, args: Vec<Value>, cf: usize) -> Result<Value, String> {
        if self.log_native {
            match &func {
                Value::Native(_) => {
                    eprint!("[NATIVE] (native_fn)");
                    for a in &args { eprint!(" {:?}", a); }
                    eprintln!();
                }
                Value::Function(fv) => {
                    eprintln!("[CALL] {}::{}({})", fv.module_path, fv.name, args.len());
                }
                _ => {}
            }
        }
        match func {
            Value::Function(fv) => {
                // O(1) lookup via child_frame_index (fast path)
                let key = (fv.parent_frame, fv.code_frame);
                if let Some(&child_frame_idx) = self.child_frame_index.get(&key) {
                    let child_rc = self.modules.frames[child_frame_idx].clone(); // Rc clone (cheap)
                    let child_frame_idx_new = self.modules.add_frame(child_rc);
                    let child = self.modules.get_frame(child_frame_idx_new);
                    let mut new_frame = Frame::new(
                        child_frame_idx_new,
                        child.stack_size,
                        child.local_var_count,
                        fv.static_base,
                    );
                    for (i, arg) in args.into_iter().enumerate() {
                        new_frame.locals.write(i as i32, arg);
                    }
                    if fv.is_method { }
                    self.call_stack.push(new_frame);
                    return Ok(Value::Void);
                }
                // Slow path: O(n) search, then index for future O(1) lookups
                for fi in 0..self.modules.frames.len() {
                    let frame = &self.modules.frames[fi];
                    if (fv.code_frame as usize) < frame.child_frames.len() {
                        let child_rc = frame.child_frames[fv.code_frame as usize].clone(); // Rc clone
                        let child_frame_idx_new = self.modules.add_frame(child_rc);
                        self.child_frame_index.insert(key, child_frame_idx_new);
                        self.frame_parent.insert(child_frame_idx_new, fi);
                        let child = self.modules.get_frame(child_frame_idx_new);
                        let mut new_frame = Frame::new(
                            child_frame_idx_new,
                            child.stack_size,
                            child.local_var_count,
                            fv.static_base,
                        );
                        for (i, arg) in args.into_iter().enumerate() {
                            new_frame.locals.write(i as i32, arg);
                        }
                        if fv.is_method { }
                        self.call_stack.push(new_frame);
                        return Ok(Value::Void);
                    }
                }
                // Not found in any frame — try global registry / native fallback
                let fv_key = if fv.module_path.is_empty() {
                        fv.name.clone()
                    } else {
                        format!("{}::{}", fv.module_path, fv.name)
                    };
                    if let Some(global_fv) = self.global_functions.get(&fv_key) {
                        // Found in global registry — execute it
                        return self.call_function_value(Value::Function(global_fv.clone()), args);
                    }
                    // Try native lookup as last resort
                    let path = if fv.module_path.is_empty() {
                        fv.name.clone()
                    } else {
                        format!("{},{}", fv.module_path, fv.name)
                    };
                    if let Some(nf) = self.natives.get(&path) {
                        Ok(nf(&args))
                    } else {
                        // Final fallback: search global registry by name only
                        if let Some(global_fv) = self.global_functions.get(&fv.name) {
                            self.call_function_value(Value::Function(global_fv.clone()), args)
                        } else {
                            Ok(Value::Nil)
                        }
                    }
            }
            Value::Native(nf) => Ok(nf(&args)),
            Value::Object(o) => {
                // Object dispatch: class_path is the native function path
                let path = &o.class_path;
                if let Some(nf) = self.natives.get(path) {
                    Ok(nf(&args))
                } else {
                    // Try fallback: use class_path as path prefix
                    for &prefix in &["main,menu,", "main,pdistd,", "main,pdiext,", "main,pdiapp,", "main,gtengine,"] {
                        if path.starts_with(prefix) {
                            if let Some(nf) = self.natives.get(path) {
                                return Ok(nf(&args));
                            }
                        }
                    }
                    Ok(Value::Nil)
                }
            }
            Value::String(s) => {
                // Strings are callable? No, this shouldn't happen.
                // Try native lookup by string as path
                if let Some(nf) = self.natives.get(s.as_str()) {
                    Ok(nf(&args))
                } else {
                    Ok(Value::Nil)
                }
            }
            _ => Ok(Value::Nil),
        }
    }

    /// Execute a FunctionValue — dispatches to child frame with O(1) lookup via index.
    pub fn call_function_value(&mut self, func: Value, args: Vec<Value>) -> Result<Value, String> {
        if let Value::Function(fv) = func {
            let key = (fv.parent_frame, fv.code_frame);

            // Fast path: O(1) lookup via child_frame_index
            if let Some(&child_frame_idx) = self.child_frame_index.get(&key) {
                let child_rc = self.modules.frames[child_frame_idx].clone(); // Rc clone (cheap)
                let child_frame_idx_new = self.modules.add_frame(child_rc);
                let child = self.modules.get_frame(child_frame_idx_new);
                let mut new_frame = Frame::new(child_frame_idx_new, child.stack_size, child.local_var_count, fv.static_base);
                for (i, arg) in args.into_iter().enumerate() {
                    new_frame.locals.write(i as i32, arg);
                }
                self.call_stack.push(new_frame);
                return Ok(Value::Void);
            }

            // Slow path: O(n) search, then index for future calls
            for fi in 0..self.modules.frames.len() {
                let frame = &self.modules.frames[fi];
                if (fv.code_frame as usize) < frame.child_frames.len() {
                    let child_rc = frame.child_frames[fv.code_frame as usize].clone(); // Rc clone
                    let child_frame_idx_new = self.modules.add_frame(child_rc);
                    // Index for future O(1) lookups
                    self.child_frame_index.insert(key, child_frame_idx_new);
                    self.frame_parent.insert(child_frame_idx_new, fi);
                    let child = self.modules.get_frame(child_frame_idx_new);
                    let mut new_frame = Frame::new(child_frame_idx_new, child.stack_size, child.local_var_count, fv.static_base);
                    for (i, arg) in args.into_iter().enumerate() {
                        new_frame.locals.write(i as i32, arg);
                    }
                    self.call_stack.push(new_frame);
                    return Ok(Value::Void);
                }
            }
            Ok(Value::Nil)
        } else {
            Ok(Value::Nil)
        }
    }
}

pub enum FrameAction {
    Continue,
    Return(Value),
    Exit,
    Yield,
}

// ─── String Method Dispatch ──────────────────────────────────
// (caller already has the argument values on the stack; these methods
//  either take no args or their args are already consumed by the bytecode)
fn string_method(s: &str, method: &str) -> Option<Value> {
    match method {
        "length" => Some(Value::Int(s.len() as i32)),
        "isEmpty" => Some(Value::Bool(s.is_empty())),
        "toString" => Some(Value::String(Rc::new(s.to_string()))),
        "toLowerCase" => Some(Value::String(Rc::new(s.to_lowercase()))),
        "toUpperCase" => Some(Value::String(Rc::new(s.to_uppercase()))),
        "trim" => Some(Value::String(Rc::new(s.trim().to_string()))),
        _ => None,
    }
}

// ─── Binary Operators ────────────────────────────────────────
fn binary_op(a: &Value, b: &Value, op: &str) -> Value {
    match op {
        "__add__" | "+" => {
            match (a, b) {
                (Value::Int(x), Value::Int(y)) => Value::Int(x + y),
                (Value::UInt(x), Value::UInt(y)) => Value::UInt(x + y),
                (Value::Float(x), Value::Float(y)) => Value::Float(x + y),
                (Value::Double(x), Value::Double(y)) => Value::Double(x + y),
                (Value::Int(x), Value::Float(y)) => Value::Float(*x as f32 + y),
                (Value::Float(x), Value::Int(y)) => Value::Float(x + *y as f32),
                (Value::String(x), Value::String(y)) => Value::String(Rc::new(format!("{}{}", x, y))),
                _ => Value::Int(0),
            }
        }
        "__sub__" | "-" => {
            match (a, b) {
                (Value::Int(x), Value::Int(y)) => Value::Int(x - y),
                (Value::Float(x), Value::Float(y)) => Value::Float(x - y),
                (Value::Double(x), Value::Double(y)) => Value::Double(x - y),
                (Value::Int(x), Value::Float(y)) => Value::Float(*x as f32 - y),
                (Value::Float(x), Value::Int(y)) => Value::Float(x - *y as f32),
                _ => Value::Int(0),
            }
        }
        "__mul__" | "*" => {
            match (a, b) {
                (Value::Int(x), Value::Int(y)) => Value::Int(x * y),
                (Value::Float(x), Value::Float(y)) => Value::Float(x * y),
                (Value::Double(x), Value::Double(y)) => Value::Double(x * y),
                _ => Value::Int(0),
            }
        }
        "__div__" | "/" => {
            match (a, b) {
                (Value::Int(x), Value::Int(y)) => if *y != 0 { Value::Int(x / y) } else { Value::Int(0) },
                (Value::Float(x), Value::Float(y)) => Value::Float(if *y != 0.0 { x / y } else { 0.0 }),
                (Value::Double(x), Value::Double(y)) => Value::Double(if *y != 0.0 { x / y } else { 0.0 }),
                _ => Value::Int(0),
            }
        }
        "__mod__" | "%" => {
            match (a, b) {
                (Value::Int(x), Value::Int(y)) => if *y != 0 { Value::Int(x % y) } else { Value::Int(0) },
                _ => Value::Int(0),
            }
        }
        "__eq__" | "==" => Value::Bool(values_equal(a, b)),
        "__ne__" | "!=" => Value::Bool(!values_equal(a, b)),
        "__ge__" | ">=" => {
            match (a, b) {
                (Value::Int(x), Value::Int(y)) => Value::Bool(x >= y),
                (Value::Float(x), Value::Float(y)) => Value::Bool(x >= y),
                (Value::String(x), Value::String(y)) => Value::Bool(x >= y),
                _ => Value::Bool(false),
            }
        }
        "__le__" | "<=" => {
            match (a, b) {
                (Value::Int(x), Value::Int(y)) => Value::Bool(x <= y),
                (Value::Float(x), Value::Float(y)) => Value::Bool(x <= y),
                (Value::String(x), Value::String(y)) => Value::Bool(x <= y),
                _ => Value::Bool(false),
            }
        }
        "__gt__" | ">" => {
            match (a, b) {
                (Value::Int(x), Value::Int(y)) => Value::Bool(x > y),
                (Value::Float(x), Value::Float(y)) => Value::Bool(x > y),
                (Value::String(x), Value::String(y)) => Value::Bool(x > y),
                _ => Value::Bool(false),
            }
        }
        "__lt__" | "<" => {
            match (a, b) {
                (Value::Int(x), Value::Int(y)) => Value::Bool(x < y),
                (Value::Float(x), Value::Float(y)) => Value::Bool(x < y),
                (Value::String(x), Value::String(y)) => Value::Bool(x < y),
                _ => Value::Bool(false),
            }
        }
        "__and__" | "&&" => Value::Bool(a.truthy() && b.truthy()),
        "__or__" | "||" => Value::Bool(a.truthy() || b.truthy()),
        _ => Value::Nil,
    }
}

fn values_equal(a: &Value, b: &Value) -> bool {
    match (a, b) {
        (Value::Nil, Value::Nil) => true,
        (Value::Void, Value::Void) => true,
        (Value::Bool(x), Value::Bool(y)) => x == y,
        (Value::Int(x), Value::Int(y)) => x == y,
        (Value::Int(x), Value::UInt(y)) => *x == *y as i32,
        (Value::UInt(x), Value::Int(y)) => *x as i32 == *y,
        (Value::UInt(x), Value::UInt(y)) => x == y,
        (Value::Long(x), Value::Long(y)) => x == y,
        (Value::ULong(x), Value::ULong(y)) => x == y,
        (Value::Float(x), Value::Float(y)) => x == y,
        (Value::Double(x), Value::Double(y)) => x == y,
        (Value::String(x), Value::String(y)) => x == y,
        (Value::Symbol(x), Value::Symbol(y)) => Rc::ptr_eq(x, y) || **x == **y,
        (Value::Int(x), Value::Float(y)) => *x as f32 == *y,
        (Value::Float(x), Value::Int(y)) => *x == *y as f32,
        (Value::Array(x), Value::Array(y)) => Rc::ptr_eq(x, y),
        (Value::Map(x), Value::Map(y)) => Rc::ptr_eq(x, y),
        (Value::Object(x), Value::Object(y)) => Rc::ptr_eq(x, y),
        (Value::Function(x), Value::Function(y)) => Rc::ptr_eq(x, y),
        (Value::Native(x), Value::Native(y)) => Rc::ptr_eq(x, y),
        _ => false,
    }
}
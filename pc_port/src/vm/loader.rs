use std::sync::OnceLock;
use std::sync::Arc;
use crate::vm::decoder::*;
use crate::vm::storage::StorageKind;

static LOADED_FRAMES: OnceLock<std::sync::Mutex<Vec<Arc<CodeFrame>>>> = OnceLock::new();

fn loaded_frames() -> &'static std::sync::Mutex<Vec<Arc<CodeFrame>>> {
    LOADED_FRAMES.get_or_init(|| std::sync::Mutex::new(Vec::new()))
}

/// Load an .adc file from disk and register it as a loadable frame.
/// Returns the frame index, or -1 on error.
pub fn load_adc_file(path: &str) -> Result<i32, String> {
    let data = std::fs::read(path).map_err(|e| format!("Read {}: {}", path, e))?;
    let frame = Loader::load(&data)?;
    let mut frames = loaded_frames().lock().unwrap();
    let idx = frames.len() as i32;
    frames.push(frame);
    Ok(idx)
}

/// Drain any newly loaded frames (called by engine before execution).
pub fn drain_loaded_frames() -> Vec<Arc<CodeFrame>> {
    let mut frames = loaded_frames().lock().unwrap();
    std::mem::take(&mut *frames)
}

/// Reads .adc bytecode from a byte slice and produces CodeFrames.
pub struct Loader;

impl Loader {
    pub fn load(data: &[u8]) -> Result<Arc<CodeFrame>, String> {
        let mut cursor = Cursor::new(data);
        Self::read_root(&mut cursor)
    }

    fn read_root(cursor: &mut Cursor) -> Result<Arc<CodeFrame>, String> {
        let magic = cursor.read_bytes(4)?;
        if &magic != b"ADCH" {
            return Err(format!("Bad magic: {:02X?}", magic));
        }
        let ver_bytes = cursor.read_bytes(4)?;
        let ver_str = String::from_utf8_lossy(&ver_bytes[..3]);
        let _version: u32 = ver_str.trim().parse().map_err(|_| "Bad version")?;

        // Symbol table (v9-v12) - uses DecodeBitsAndAdvance style
        let symbol_count = cursor.read_adhoc_varint()? as usize;
        let mut symbols = Vec::with_capacity(symbol_count);
        for i in 0..symbol_count {
            let len = match cursor.read_multi_byte_length() {
                Ok(l) => l as usize,
                Err(e) => {
                    eprintln!("Warning: Symbol table truncated at {}: {}", i, e);
                    break;
                }
            };
            let bytes = match cursor.read_bytes(len) {
                Ok(b) => b,
                Err(e) => {
                    eprintln!("Warning: Symbol table truncated at {}: {}", i, e);
                    break;
                }
            };
            let sym = String::from_utf8_lossy(&bytes).into_owned();
            symbols.push(sym);
        }
        cursor.symbols = symbols;

        Self::read_code_frame(cursor, 12u8)
    }

    fn read_code_frame(cursor: &mut Cursor, parent_version: u8) -> Result<Arc<CodeFrame>, String> {
        // For V >= 8: first byte is has_debug, then version follows.
        // For V < 8: first byte is version (no has_debug in stream, forced true).
        let (has_debug, adhoc_version) = if parent_version >= 8 {
            let hd = cursor.read_bool().unwrap_or(true);
            let ver = cursor.read_u8().unwrap_or(12);
            (hd, ver)
        } else {
            let ver = cursor.read_u8().unwrap_or(12);
            (true, ver)
        };


        // Read source file path:
        // For V >= 8: only if has_debug && version != 8
        // For V < 8: ALWAYS read (forced true has_debug, version is the first byte)
        if adhoc_version >= 8 {
            if adhoc_version != 8 && has_debug {
                if let Ok(sym_idx) = cursor.read_varint() {
                    let _src_file = cursor.get_symbol(sym_idx as usize);
                }
            }
        } else {
            // V < 8: always read source file
            if let Ok(sym_idx) = cursor.read_varint() {
                let _src_file = cursor.get_symbol(sym_idx as usize);
            }
            // Also need to read the rest of the V < 8 header
            // (which includes params, etc. in a different format)
        }

        // v12+: read HasRestElement (only for V >= 8)
        let _has_rest = if adhoc_version >= 8 && adhoc_version >= 12 {
            cursor.read_bool().unwrap_or(false)
        } else {
            false
        };

        if adhoc_version >= 8 {
            // V >= 8 header: argCount, params with indices, captured vars, unkVarStackIndex
            let param_count = cursor.read_u32().unwrap_or(0) as i32;
            let safe_param_count = param_count.max(0).min(10000) as usize;
            let mut param_names = Vec::with_capacity(safe_param_count);
            for _ in 0..safe_param_count {
                if let Ok(sym_idx) = cursor.read_varint() {
                    let _name = cursor.get_symbol(sym_idx as usize);
                    let _param_idx = cursor.read_i32().unwrap_or(0);
                    param_names.push(String::new());
                }
            }

            let captured_count = cursor.read_u32().unwrap_or(0) as i32;
            let safe_captured = captured_count.max(0).min(1000) as usize;
            for _ in 0..safe_captured {
                let _ = cursor.read_varint();
                let _ = cursor.read_i32();
            }

            let _unk_stack_idx = cursor.read_u32().unwrap_or(0);

            // Split stack (V >= 8)
            let stack_size = cursor.read_i32().unwrap_or(0);
            let local_var_count = cursor.read_i32().unwrap_or(0);
            let static_var_count = cursor.read_i32().unwrap_or(0);
            let insn_count = cursor.read_u32().unwrap_or(0) as i32;
            Self::read_instructions(cursor, has_debug, insn_count, stack_size, local_var_count,
                static_var_count, param_count, param_names, adhoc_version)
        } else {
            // V < 8 header format (from reference):
            // CodeSize, InsnCount, ParamCount, StackSize, LocalVarCount
            let _code_size = cursor.read_i32().unwrap_or(0);
            let insn_count = cursor.read_i32().unwrap_or(0);
            let param_count = cursor.read_i32().unwrap_or(0);
            let safe_param_count = param_count.max(0).min(10000) as usize;
            let mut param_names = Vec::with_capacity(safe_param_count);
            for _ in 0..safe_param_count {
                if let Ok(name) = Self::read_symbol(cursor) {
                    param_names.push(name);
                }
            }
            let stack_size = cursor.read_i32().unwrap_or(0);
            let local_var_count = cursor.read_i32().unwrap_or(0);
            let static_var_count = 0; // V < 8 has no static vars

            // has_debug is forced true for V < 8 (set above)
            Self::read_instructions(cursor, true, insn_count, stack_size, local_var_count,
                static_var_count, param_count, param_names, adhoc_version)
        }
    }

fn read_instructions(
        cursor: &mut Cursor,
        has_debug: bool,
        insn_count: i32,
        stack_size: i32,
        local_var_count: i32,
        static_var_count: i32,
        param_count: i32,
        param_names: Vec<String>,
        adhoc_version: u8,
    ) -> Result<Arc<CodeFrame>, String> {
        let safe_insn_count = insn_count.max(0).min(100000) as usize;

        let mut instructions = Vec::with_capacity(safe_insn_count);
        let mut instruction_lines = Vec::with_capacity(safe_insn_count);
        let mut child_frames: Vec<Arc<CodeFrame>> = Vec::new();
        let mut loaded_count = 0i32;

        for _ in 0..safe_insn_count {
            let insn_start = cursor.position();
            let line = if has_debug { cursor.read_u32().unwrap_or(0) as i32 } else { 0 };
            instruction_lines.push(line);
            let opcode_byte = match cursor.read_u8() {
                Ok(b) => b,
                Err(e) => {
                    eprintln!("Warning: Stopping at instruction {} due to: {}", loaded_count, e);
                    break;
                }
            };
            if loaded_count == 0 {
                // Skip first-insn debug
            }
            let opcode = match Opcode::from_byte(opcode_byte) {
                Ok(op) => op,
                Err(e) => {
                    eprintln!("  [SKIP] instruction {} unknown opcode 0x{:02X} at 0x{:X}", loaded_count, opcode_byte, insn_start);
                    break;
                }
            };
            let insn = match Self::decode_instruction(cursor, &opcode, &mut child_frames, adhoc_version) {
                Ok(i) => i,
                Err(e) => {
                    let end_pos = cursor.position();
                    eprintln!("Warning: Stopping at instruction {} (opcode {:?}, offset 0x{:X}-0x{:X}) due to decode error: {}", loaded_count, opcode, insn_start, end_pos, e);
                    break;
                }
            };
            instructions.push(insn);
            loaded_count += 1;
        }

        Ok(Arc::new(CodeFrame {
            has_debug,
            adhoc_version,
            param_count,
            param_names,
            stack_size,
            local_var_count,
            static_var_count,
            instructions,
            child_frames,
            instruction_lines,
        }))
    }

    fn decode_instruction(
        cursor: &mut Cursor,
        opcode: &Opcode,
        child_frames: &mut Vec<Arc<CodeFrame>>,
        adhoc_version: u8,
    ) -> Result<Instruction, String> {
        // V12 (adhoc_version >= 8) uses NEW operand format where many instructions are stack-based.
        // V0 (adhoc_version < 8) uses OLD operand format with explicit symbol references.
        let is_v12 = adhoc_version >= 8;
        match opcode {
            Opcode::ArrayConstOld => {
                let count = cursor.read_u32()?;
                Ok(Instruction::ArrayConstOld { count })
            }
            Opcode::ArrayConst => {
                let count = cursor.read_u32()?;
                Ok(Instruction::ArrayConst { count })
            }
            Opcode::ArrayPush => Ok(Instruction::ArrayPush),
            Opcode::Assign => {
                if is_v12 {
                    Ok(Instruction::Assign { symbols: vec![], storage_index: 0 })
                } else {
                    let (symbols, storage_index) = Self::read_symbols_and_index(cursor)?;
                    Ok(Instruction::Assign { symbols, storage_index })
                }
            }
            Opcode::AssignPop => Ok(Instruction::AssignPop),
            Opcode::AssignOld => {
                if is_v12 {
                    Ok(Instruction::AssignOld { symbols: vec![], index: 0 })
                } else {
                    let (symbols, index) = Self::read_symbols_and_index(cursor)?;
                    Ok(Instruction::AssignOld { symbols, index })
                }
            }
            Opcode::AttributeDefine => {
                let symbol = Self::read_symbol(cursor)?;
                Ok(Instruction::AttributeDefine { symbols: vec![symbol] })
            }
            Opcode::AttributeEval => {
                let symbols = Self::read_symbols(cursor)?;
                Ok(Instruction::AttributeEval { symbols })
            }
            Opcode::AttributePush => {
                let symbols = Self::read_symbols(cursor)?;
                Ok(Instruction::AttributePush { symbols, kind: StorageKind::Local, index: 0 })
            }
            Opcode::BinaryAssignOperator => {
                if is_v12 {
                    let op = Self::read_symbol(cursor)?;
                    Ok(Instruction::BinaryAssignOperator { op, symbols: vec![] })
                } else {
                    let op = cursor.read_string()?;
                    let symbols = Self::read_symbols(cursor)?;
                    Ok(Instruction::BinaryAssignOperator { op, symbols })
                }
            }
            Opcode::BinaryOperator => {
                let op = cursor.read_string()?;
                Ok(Instruction::BinaryOperator { op })
            }
            Opcode::BoolConst => {
                let value = cursor.read_bool()?;
                Ok(Instruction::BoolConst { value })
            }
            Opcode::Call => {
                let arg_count = cursor.read_i32()?;
                Ok(Instruction::Call { arg_count })
            }
            Opcode::CallOld => {
                let arg_count = cursor.read_i32()?;
                Ok(Instruction::CallOld { arg_count })
            }
            Opcode::ClassDefine => {
                let _name = Self::read_symbol(cursor)?;
                let _extends = Self::read_symbols(cursor)?;
                Ok(Instruction::ClassDefine { symbols: vec![_name] })
            }
            Opcode::DelegateDefine => {
                let name = Self::read_symbol(cursor)?;
                let target = cursor.get_symbol(0);
                Ok(Instruction::DelegateDefine { name, target })
            }
            Opcode::DoubleConst => {
                let value = cursor.read_f64()?;
                Ok(Instruction::DoubleConst { value })
            }
            Opcode::ElementEval => Ok(Instruction::ElementEval),
            Opcode::ElementPush => Ok(Instruction::ElementPush),
            Opcode::Eval => Ok(Instruction::Eval),
            Opcode::FloatConst => {
                let value = cursor.read_f32()?;
                Ok(Instruction::FloatConst { value })
            }
            Opcode::FunctionConst => {
                // FunctionConst has a CodeFrame sub-block (parameters + instructions)
                let child_frame = Self::read_code_frame(cursor, adhoc_version)?;
                child_frames.push(child_frame);
                Ok(Instruction::FunctionConst)
            }
            Opcode::FunctionDefine => {
                let name = Self::read_symbol(cursor)?;
                let cf_start = cursor.position();
                let child_frame = Self::read_code_frame(cursor, adhoc_version)?;
                child_frames.push(child_frame);
                Ok(Instruction::FunctionDefine { symbols: vec![name] })
            }
            Opcode::Import => {
                let symbols = Self::read_symbols(cursor)?;
                Ok(Instruction::Import { symbols })
            }
            Opcode::IntConst => {
                let value = cursor.read_i32()?;
                Ok(Instruction::IntConst { value })
            }
            Opcode::Jump => {
                let target = cursor.read_i32()?;
                Ok(Instruction::Jump { target })
            }
            Opcode::JumpIfFalse => {
                let target = cursor.read_i32()?;
                Ok(Instruction::JumpIfFalse { target })
            }
            Opcode::JumpIfNil => {
                let target = cursor.read_i32()?;
                Ok(Instruction::JumpIfNil { target })
            }
            Opcode::JumpIfTrue => {
                let target = cursor.read_i32()?;
                Ok(Instruction::JumpIfTrue { target })
            }
            Opcode::Leave => {
                let depth = cursor.read_i32()?;
                let rewind_locals = cursor.read_i32()?;
                Ok(Instruction::Leave { depth, rewind_locals })
            }
            Opcode::ListAssign => {
                // V12: skip optional operands (ElemCount, HasRestElement) using adhoc varint
                if is_v12 {
                    let _elem_count = cursor.read_adhoc_varint()?;
                    let _has_rest = cursor.read_adhoc_varint()?;
                }
                Ok(Instruction::ListAssign)
            }
            Opcode::ListAssignOld => {
                let var_count = cursor.read_i32()?;
                Ok(Instruction::ListAssignOld)
            }
            Opcode::LocalDefine => Ok(Instruction::LocalDefine),
            Opcode::LogicalAnd => {
                let target = cursor.read_i32()?;
                Ok(Instruction::LogicalAnd { target })
            }
            Opcode::LogicalAndOld => {
                let target = cursor.read_i32()?;
                Ok(Instruction::LogicalAndOld { target })
            }
            Opcode::LogicalOptional => Ok(Instruction::LogicalOptional),
            Opcode::LogicalOr => {
                let target = cursor.read_i32()?;
                Ok(Instruction::LogicalOr { target })
            }
            Opcode::LogicalOrOld => {
                let target = cursor.read_i32()?;
                Ok(Instruction::LogicalOrOld { target })
            }
            Opcode::LongConst => {
                let value = cursor.read_i64()?;
                Ok(Instruction::LongConst { value })
            }
            Opcode::MapConst => Ok(Instruction::MapConst),
            Opcode::MapConstOld => {
                let value = cursor.read_i32()?;
                Ok(Instruction::MapConstOld)
            }
            Opcode::MapInsert => Ok(Instruction::MapInsert),
            Opcode::MethodConst => {
                let child_frame = Self::read_code_frame(cursor, adhoc_version)?;
                child_frames.push(child_frame);
                Ok(Instruction::MethodConst)
            }
            Opcode::MethodDefine => {
                let name = Self::read_symbol(cursor)?;
                let cf_start = cursor.position();
                let child_frame = Self::read_code_frame(cursor, adhoc_version)?;
                child_frames.push(child_frame);
                Ok(Instruction::MethodDefine { symbols: vec![name] })
            }
            Opcode::ModuleConstructor => Ok(Instruction::ModuleConstructor),
            Opcode::ModuleDefine => {
                let symbols = Self::read_symbols(cursor)?;
                Ok(Instruction::ModuleDefine { symbols })
            }
            Opcode::NilConst => Ok(Instruction::NilConst),
            Opcode::Nop => Ok(Instruction::Nop),
            Opcode::ObjectSelector => Ok(Instruction::ObjectSelector { symbol: String::new() }),
            Opcode::Pop => Ok(Instruction::Pop),
            Opcode::PopOld => Ok(Instruction::PopOld),
            Opcode::Print => {
                let _arg_count = cursor.read_i32()?;
                Ok(Instruction::Nop)
            }
            Opcode::Require => {
                if is_v12 {
                    let symbols = Self::read_symbols(cursor)?;
                    Ok(Instruction::Require { symbols })
                } else {
                    let symbols = Self::read_symbols(cursor)?;
                    Ok(Instruction::Require { symbols })
                }
            }
            Opcode::SetState => {
                let state_byte = cursor.read_u8()?;
                Ok(Instruction::SetState { state: state_byte })
            }
            Opcode::SetStateOld => {
                if is_v12 {
                    let _state = cursor.read_u8()?;
                } else {
                    let _state = cursor.read_i32()?;
                }
                Ok(Instruction::SetStateOld)
            }
            Opcode::SourceFile => {
                let symbol = Self::read_symbol(cursor)?;
                Ok(Instruction::SourceFile { symbols: vec![symbol] })
            }
            Opcode::StaticDefine => {
                let symbol = Self::read_symbol(cursor)?;
                Ok(Instruction::StaticDefine { symbols: vec![symbol] })
            }
            Opcode::StringConst => {
                let sym_idx = cursor.read_varint()? as usize;
                let symbol = cursor.get_symbol(sym_idx);
                Ok(Instruction::StringConst { symbols: vec![symbol] })
            }
            Opcode::StringPush => {
                let count = cursor.read_i32()?;
                Ok(Instruction::StringPush { count })
            }
            Opcode::SymbolConst => {
                let sym_idx = cursor.read_varint()? as usize;
                let symbol = cursor.get_symbol(sym_idx);
                Ok(Instruction::SymbolConst { symbol })
            }
            Opcode::Throw => Ok(Instruction::Nop),
            Opcode::TryCatch => {
                let target = cursor.read_i32()?;
                Ok(Instruction::TryCatch { target })
            }
            Opcode::UIntConst => {
                let value = cursor.read_u32()?;
                Ok(Instruction::UIntConst { value })
            }
            Opcode::ULongConst => {
                let value = cursor.read_u64()?;
                Ok(Instruction::ULongConst { value })
            }
            Opcode::UnaryAssignOperator => {
                let _op = cursor.read_string()?;
                Ok(Instruction::Nop)
            }
            Opcode::UnaryOperator => {
                let op = cursor.read_string()?;
                Ok(Instruction::UnaryOperator { op })
            }
            Opcode::VaCall => {
                let arg_count = cursor.read_i32()?;
                Ok(Instruction::VaCall { arg_count })
            }
            Opcode::VariableEval => {
                let symbols = Self::read_symbols(cursor)?;
                let index = cursor.read_i32()?;
                let kind = if symbols.len() > 1 { StorageKind::Static } else { StorageKind::Local };
                Ok(Instruction::VariableEval { symbols, kind, index })
            }
            Opcode::VariablePush => {
                let symbols = Self::read_symbols(cursor)?;
                let index = cursor.read_i32()?;
                let kind = if symbols.len() > 1 { StorageKind::Static } else { StorageKind::Local };
                Ok(Instruction::VariablePush { symbols, kind, index })
            }
            Opcode::VoidConst => Ok(Instruction::VoidConst),
            Opcode::ByteConst => {
                let value = cursor.read_i8()?;
                Ok(Instruction::ByteConst { value: value as i32 })
            }
            Opcode::UByteConst => {
                let value = cursor.read_u8()?;
                Ok(Instruction::UByteConst { value: value as i32 })
            }
            Opcode::ShortConst => {
                let value = cursor.read_i16()?;
                Ok(Instruction::ShortConst { value: value as i32 })
            }
            Opcode::UShortConst => {
                let value = cursor.read_u16()?;
                Ok(Instruction::UShortConst { value: value as i32 })
            }
            Opcode::IteratorNext => {
                // IteratorNext: target=jump if done, var_index=local var slot for element
                let target = cursor.read_i32()?;
                let var_index = cursor.read_i32()?;
                Ok(Instruction::IteratorNext { target, var_index })
            }
            _ => {
                //eprintln!("UNHANDLED OPCODE at 0x{:X}", cursor.position());
                Ok(Instruction::Nop)
            }
        }
    }

    fn read_symbols(cursor: &mut Cursor) -> Result<Vec<String>, String> {
        let count = cursor.read_u32()? as usize;
        let mut symbols = Vec::with_capacity(count);
        for _ in 0..count {
            let sym_idx = cursor.read_varint()? as usize;
            symbols.push(cursor.get_symbol(sym_idx));
        }
        Ok(symbols)
    }

    fn read_symbol(cursor: &mut Cursor) -> Result<String, String> {
        let sym_idx = cursor.read_varint()? as usize;
        Ok(cursor.get_symbol(sym_idx))
    }

    fn read_storage_ref(cursor: &mut Cursor) -> Result<(Vec<String>, StorageKind, i32), String> {
        let count = cursor.read_u32()? as usize;
        let mut symbols = Vec::with_capacity(count);
        for _ in 0..count {
            let sym_idx = cursor.read_varint()? as usize;
            symbols.push(cursor.get_symbol(sym_idx));
        }
        let index = cursor.read_i32()?;
        Ok((symbols, StorageKind::Local, index))
    }

    fn read_symbols_and_index(cursor: &mut Cursor) -> Result<(Vec<String>, i32), String> {
        let count = cursor.read_u32()? as usize;
        let mut symbols = Vec::with_capacity(count);
        for _ in 0..count {
            let sym_idx = cursor.read_varint()? as usize;
            symbols.push(cursor.get_symbol(sym_idx));
        }
        let index = cursor.read_i32()?;
        Ok((symbols, index))
    }
}

pub struct Cursor<'a> {
    data: &'a [u8],
    pos: usize,
    symbols: Vec<String>,
}

impl<'a> Cursor<'a> {
    fn new(data: &'a [u8]) -> Self {
        Cursor { data, pos: 0, symbols: Vec::new() }
    }

    fn position(&self) -> usize {
        self.pos
    }

    fn read_bytes(&mut self, count: usize) -> Result<&'a [u8], String> {
        if self.pos + count > self.data.len() {
            return Err(format!("Unexpected EOF at offset 0x{:X}", self.pos));
        }
        let slice = &self.data[self.pos..self.pos + count];
        self.pos += count;
        Ok(slice)
    }

    fn read_u8(&mut self) -> Result<u8, String> {
        let b = self.read_bytes(1)?;
        Ok(b[0])
    }

    fn read_i8(&mut self) -> Result<i8, String> {
        let b = self.read_bytes(1)?;
        Ok(b[0] as i8)
    }

    fn read_i16(&mut self) -> Result<i16, String> {
        let b = self.read_bytes(2)?;
        Ok(i16::from_le_bytes([b[0], b[1]]))
    }

    fn read_u16(&mut self) -> Result<u16, String> {
        let b = self.read_bytes(2)?;
        Ok(u16::from_le_bytes([b[0], b[1]]))
    }

    fn read_bool(&mut self) -> Result<bool, String> {
        Ok(self.read_u8()? != 0)
    }

    fn read_i32(&mut self) -> Result<i32, String> {
        let b = self.read_bytes(4)?;
        Ok(i32::from_le_bytes([b[0], b[1], b[2], b[3]]))
    }

    fn read_u32(&mut self) -> Result<u32, String> {
        let b = self.read_bytes(4)?;
        Ok(u32::from_le_bytes([b[0], b[1], b[2], b[3]]))
    }

    fn read_i64(&mut self) -> Result<i64, String> {
        let b = self.read_bytes(8)?;
        Ok(i64::from_le_bytes([b[0], b[1], b[2], b[3], b[4], b[5], b[6], b[7]]))
    }

    fn read_u64(&mut self) -> Result<u64, String> {
        let b = self.read_bytes(8)?;
        Ok(u64::from_le_bytes([b[0], b[1], b[2], b[3], b[4], b[5], b[6], b[7]]))
    }

    fn read_f32(&mut self) -> Result<f32, String> {
        Ok(f32::from_bits(self.read_u32()?))
    }

    fn read_f64(&mut self) -> Result<f64, String> {
        Ok(f64::from_bits(self.read_u64()?))
    }

    fn read_string(&mut self) -> Result<String, String> {
        let sym_idx = self.read_varint()? as usize;
        Ok(self.get_symbol(sym_idx))
    }

    fn get_symbol(&self, idx: usize) -> String {
        if idx < self.symbols.len() {
            self.symbols[idx].clone()
        } else {
            format!("!sym{}", idx)
        }
    }

    fn read_varint(&mut self) -> Result<u64, String> {
        self.read_adhoc_varint()
    }

    fn read_multi_byte_length(&mut self) -> Result<usize, String> {
        self.read_adhoc_varint().map(|v| v as usize)
    }

    /// Read Adhoc custom varint (DecodeBitsAndAdvance style).
    fn read_adhoc_varint(&mut self) -> Result<u64, String> {
        let mut value: u64 = self.read_u8()? as u64;
        let mut mask: u64 = 0x80;

        while (value & mask) != 0 {
            value = ((value - mask) << 8) | (self.read_u8()? as u64);
            mask <<= 7;
        }
        Ok(value)
    }
}
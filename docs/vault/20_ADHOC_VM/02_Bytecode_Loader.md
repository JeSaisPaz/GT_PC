---
tags: [pc-port, rust, vm, loader, bytecode, adc]
type: documentation
project: GT PSP PC Port
section: Core Engine
---

# Bytecode Loader — .ADC Format

> `.adc` bytecode parser (`pc_port/src/vm/loader.rs`).

## File Format

```
┌──────────────────────────────────────────┐
│ Magic: "ADCH"          (4 bytes)         │
│ Version: "vXX"         (4 bytes, ASCII)│
├──────────────────────────────────────────┤
│ Symbol Table                             │
│   • varint count                         │
│   • symbols[] (length-prefixed UTF-8)    │
├──────────────────────────────────────────┤
│ Code Frame                               │
│   • Header (params, captured vars)      │
│   • Stack sizes                         │
│   • Instruction count                   │
│   • Instructions[]                      │
│   • Child frames[] (nested)             │
└──────────────────────────────────────────┘
```

## Loader Structure

```rust
pub struct Loader;

impl Loader {
    pub fn load(data: &[u8]) -> Result<Arc<CodeFrame>, String> {
        let mut cursor = Cursor::new(data);
        Self::read_root(&mut cursor)
    }
}
```

## Header Parsing

### Magic & Version

```rust
let magic = cursor.read_bytes(4)?;
if &magic != b"ADCH" {
    return Err(format!("Bad magic: {:02X?}", magic));
}
let ver_bytes = cursor.read_bytes(4)?;
let version = String::from_utf8_lossy(&ver_bytes[..3]); // "v12"
```

### Symbol Table (v9-v12)

```rust
// LEB128-style varint for count
let symbol_count = cursor.read_adhoc_varint()? as usize;

// For each symbol:
let len = cursor.read_multi_byte_length()?;
let bytes = cursor.read_bytes(len)?;
let symbol = String::from_utf8_lossy(&bytes);
symbols.push(symbol);
```

### Code Frame Header (v8+)

```rust
let has_debug = cursor.read_bool()?;          // Has debug info
let adhoc_version = cursor.read_u8()?;       // e.g., 12

// Source file (if has_debug && version != 8)
if adhoc_version >= 8 && has_debug && adhoc_version != 8 {
    let sym_idx = cursor.read_varint()?;
    let src_file = cursor.get_symbol(sym_idx)?;
}

// Parameters
let param_count = cursor.read_u32()? as i32;
for _ in 0..param_count {
    let sym_idx = cursor.read_varint()?;
    let name = cursor.get_symbol(sym_idx)?;
    let param_idx = cursor.read_i32()?;
}

// Captured variables
let captured_count = cursor.read_u32()? as i32;

// Stack sizes
let stack_size = cursor.read_i32()?;        // Operand stack
let local_var_count = cursor.read_i32()?;   // Local variables
let static_var_count = cursor.read_i32()?; // Static variables
let insn_count = cursor.read_u32()? as i32;
```

## Instruction Parsing

```rust
fn read_instructions(cursor, ...) -> CodeFrame {
    let mut instructions = Vec::new();
    
    for _ in 0..safe_insn_count {
        let line = cursor.read_u32()?;        // Debug line number
        let opcode_byte = cursor.read_u8()?;
        let opcode = Opcode::from_byte(opcode_byte)?;
        
        // Parse opcode-specific fields
        let insn = match opcode {
            VARIABLE_PUSH => {
                let src = cursor.read_varint()?;   // Source (local/static/constant)
                let idx = cursor.read_varint()?;   // Index
                let module_idx = cursor.read_varint()?;
                let is_static = cursor.read_bool()?;
                Instruction::VariablePush(...)
            }
            CALL => {
                let arg_count = cursor.read_varint()?;
                let method_flag = cursor.read_bool()?;
                Instruction::Call(...)
            }
            // ... handle all 72 opcodes
        };
        instructions.push(insn);
    }
}
```

## CodeFrame Structure

```rust
pub struct CodeFrame {
    pub version: u8,
    pub stack_size: i32,
    pub local_var_count: i32,
    pub static_var_count: i32,
    pub param_count: i32,
    pub param_names: Vec<String>,
    pub instructions: Vec<Instruction>,
    pub instruction_lines: Vec<i32>,  // Debug line numbers
    pub child_frames: Vec<Arc<CodeFrame>>,
    pub static_defs: Vec<StaticDef>,   // Static variable declarations
    // Compilation metadata
    pub source_file: Option<String>,
    pub has_rest_element: bool,
}
```

## Dynamic Script Loading

Frames loaded during execution are queued:

```rust
static LOADED_FRAMES: OnceLock<Mutex<Vec<Arc<CodeFrame>>>> = OnceLock::new();

pub fn load_adc_file(path: &str) -> Result<i32, String> {
    let data = std::fs::read(path)?;
    let frame = Loader::load(&data)?;
    let mut frames = loaded_frames().lock().unwrap();
    let idx = frames.len() as i32;
    frames.push(frame);
    Ok(idx)
}

pub fn drain_loaded_frames() -> Vec<Arc<CodeFrame>> {
    let mut frames = loaded_frames().lock().unwrap();
    std::mem::take(&mut *frames)
}
```

## Supported Versions

| Version | Status |
|---------|--------|
| v5 | Supported (GT4) |
| v7 | Supported (GT4) |
| v8 | Supported |
| v9 | Supported |
| v10 | Supported |
| v11 | Supported |
| v12 | Supported (GT PSP, GT5, GT6, GT Sport) |

## See Also

- [[20_ADHOC_VM/00_Index|Adhoc VM]]
- [[20_ADHOC_VM/01_VM_Engine|VM Engine]]
- [[10_PC_Port/00_Index|PC Port Index]]
---
tags: [index, adhoc, vm, rust]
type: index
project: GT PSP PC Port
subproject: adhoc-vm
---

# Adhoc VM — Index

> Rust implementation of Adhoc bytecode interpreter.

## Overview

Custom Adhoc VM that loads `.adc` bytecode files and executes them. Core of the PC Port.

## Structure

| Component | File | Lines | Status |
|-----------|------|-------|--------|
| Loader | [[20_ADHOC_VM/02_Bytecode_Loader\|loader.rs]] | 431 | ✅ |
| Engine | [[20_ADHOC_VM/01_VM_Engine\|engine.rs]] | 689 | ✅ |
| Opcodes | [[20_ADHOC_VM/04_Opcodes\|decoder.rs]] | 333 | ✅ |
| Frames | [[20_ADHOC_VM/01_VM_Engine\|frame.rs]] | 61 | ✅ |
| Value Types | [[20_ADHOC_VM/01_VM_Engine\|value.rs]] | 137 | ✅ |
| Modules | [[20_ADHOC_VM/01_VM_Engine\|module.rs]] | 60 | ✅ |
| Native FFI | [[20_ADHOC_VM/01_VM_Engine\|native.rs]] | 29 | ✅ |

## Key Documentation

| Topic | Description |
|-------|-------------|
| [[20_ADHOC_VM/01_VM_Engine\|VM Engine]] | Execution engine, call stack |
| [[20_ADHOC_VM/02_Bytecode_Loader\|Bytecode Loader]] | .ADC format parsing |
| [[20_ADHOC_VM/03_Main_Loop\|Main Loop]] | Native game loop |
| [[20_ADHOC_VM/04_Opcodes\|Opcodes]] | 72 opcode reference |

## Architecture

```
.adc File
  ├── Magic: "ADCH"
  ├── Version: "v12"
  ├── Symbol Table (LEB128)
  └── Code Frame
        ├── Header (params, stack sizes)
        ├── Instructions[]
        └── Child Frames[]
              ↓ Loader
        CodeFrame
              ↓ Decoder
        Instruction[]
              ↓ Engine
        Stack Evaluation
```

## Value Types (20 variants)

1. Nil, Void, Bool, Int, UInt, Long, ULong, Float, Double
2. Byte, UByte, Short, UShort
3. String, Symbol, Array, Map, Object
4. FunctionRef, NativeFn

## Opcodes (77 byte-decodable, 107 total enum variants)

| Category | Count |
|----------|-------|
| Stack (Push/Pop/Eval) | 20 |
| Control (Jump, Try/Catch, **IteratorNext**) | 16 |
| Call (Call, MethodCall) | 12 |
| Define (Module/Function/Class) | 10 |
| Math (Binary/Unary) | 8 |
| Logical (And/Or) | 4 |

**Recent Addition:** `IteratorNext` (0x50) — Implements foreach loop iteration, fixes `execBoot()` infinite loop in bootstrap.adc.

## Main Loop

Native state machine replaces `packed_main_loop.adc`:

```rust
enum LoopPhase {
    CheckConditions,
    MenuAllocateReplay,
    MenuLoadResource,
    MenuSetMode,
    MenuStartProject,
    MenuSync,        // ← Blocking: VM runs here
    RaceBGM,
    RaceSetMode,
    RaceExecute,
    RaceRunning,     // ← Native physics
    RaceEndReplay,
    ClearFontCache,
}
```

## Running

```bash
# Dump script structure
cargo run -- --dump file.adc

# Trace execution
cargo run -- --trace file.adc

# List native functions
cargo run -- --list-native
```

## See Also

- [[10_PC_Port/00_Index|PC Port Index]]
- [[10_PC_Port/02_Race_Engine|Race Engine]]
- [[10_PC_Port/05_Graphics|Graphics]]

---

*Updated: 2026-04-29 (IteratorNext opcode added)*
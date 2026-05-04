---
tags: [pc-port, rust, vm, opcodes, bytecode]
type: documentation
project: GT PSP PC Port
section: Core Engine
---

# Adhoc Opcodes — Full Reference

> 77 byte-decodable opcode variants (107 total enum variants) (`pc_port/src/vm/decoder.rs`).

## Opcode Enum (0x00-0x4F)

| Byte | Name | Category |
|------|------|----------|
| 0x00 | `ArrayConstOld` | Constants |
| 0x01 | `ArrayConst` | Constants |
| 0x02 | `ArrayPush` | Array |
| 0x03 | `Assign` | Assignment |
| 0x04 | `AssignPop` | Assignment |
| 0x05 | `AssignOld` | Assignment |
| 0x06 | `AttributeDefine` | Class |
| 0x07 | `AttributeEval` | Object |
| 0x08 | `AttributePush` | Object |
| 0x09 | `BinaryAssignOperator` | Math |
| 0x0A | `BinaryOperator` | Math |
| 0x0B | `Call` | Call |
| 0x0C | `ClassDefine` | Class |
| 0x0D | `Eval` | Stack |
| 0x0E | `FloatConst` | Constants |
| 0x0F | `FunctionDefine` | Function |
| 0x10 | `Import` | Module |
| 0x11 | `IntConst` | Constants |
| 0x12 | `Jump` | Control |
| 0x13 | `JumpIfTrue` | Control |
| 0x14 | `JumpIfFalse` | Control |
| 0x15 | `ListAssign` | Assignment |
| 0x16 | `LocalDefine` | Variable |
| 0x17 | `LogicalAnd` | Logical |
| 0x18 | `LogicalOr` | Logical |
| 0x19 | `MethodDefine` | Function |
| 0x1A | `ModuleDefine` | Module |
| 0x1B | `NilConst` | Constants |
| 0x1C | `Nop` | Control |
| 0x1D | `Pop` | Stack |
| 0x1E | `PopOld` | Stack |
| 0x1F | `Print` | Debug |
| 0x20 | `Require` | Module |
| 0x21 | `SetStateOld` | Control |
| 0x22 | `StaticDefine` | Variable |
| 0x23 | `StringConst` | Constants |
| 0x24 | `StringPush` | Stack |
| 0x25 | `Throw` | Exception |
| 0x26 | `TryCatch` | Exception |
| 0x50 | `IteratorNext` | **Control** |

## Categories

### Constants (Immediates)

| Opcode | Operands | Description |
|--------|----------|-------------|
| `IntConst` | value:i32 | Integer constant |
| `FloatConst` | value:f32 | Float constant |
| `DoubleConst` | value:f64 | Double constant |
| `BoolConst` | value:bool | Boolean |
| `NilConst` | — | Null/undefined |
| `StringConst` | id:varint | String constant (symbol table) |
| `UIntConst` | value:u32 | Unsigned integer |
| `ArrayConst` | count:varint | Array literal |
| `MapConst` | count:varint | Map literal |

### Stack Operations

| Opcode | Operands | Description |
|--------|----------|-------------|
| `Push` | value | Push to stack |
| `Pop` | — | Pop from stack |
| `Eval` | — | Evaluate top of stack |
| `VariablePush` | src, idx, mod, static | Push variable |
| `VariableEval` | src, idx, mod, static | Evaluate variable |

### Control Flow

| Opcode | Operands | Description |
|--------|----------|-------------|
| `Jump` | target | Unconditional jump |
| `JumpIfTrue` | target | Branch if true |
| `JumpIfFalse` | target | Branch if false |
| `JumpIfNil` | target | Branch if null |
| `JumpUndefined` | target | Branch if undefined |
| `JumpDefined` | target | Branch if defined |
| `IteratorNext` | target, var_index | Foreach loop iteration |
| `Switch` | case_count | Switch statement |
| `Case` | value, target | Case label |
| `Leave` | — | Function exit |
| `SetState` | state | Set return state |
| `TryCatch` | try_target, catch_target | Exception handler |

### IteratorNext — Foreach Loop Support

The `IteratorNext` opcode (0x50) implements foreach-style iteration over arrays:

```rust
Instruction::IteratorNext { target, var_index }
```

**Operands:**
- `target: i32` — Jump target when iteration is complete (array empty)
- `var_index: i32` — Local variable index to store current element

**Execution:**
1. Pop array from stack
2. If array has elements:
   - Store first element in local variable at `var_index`
   - Push remaining elements back onto stack
   - Continue to next instruction (loop body)
3. If array is empty:
   - Jump to `target` (exit loop)

**Purpose:** This opcode was missing and caused infinite loops in `bootstrap.adc`'s `initArgs()` function, preventing proper game initialization. With this implementation, `execBoot()` and `execBootPhase2()` now execute correctly.

**Example bytecode pattern for foreach:**
```
ArrayConst { count }     # Push array to iterate
IteratorNext {           # Check if array has elements
    target = loop_end,   # Jump here when done
    var_index = 0        # Store element in local[0]
}
# ... loop body using local[0] ...
Jump { target = IteratorNext }  # Continue iteration
loop_end:
```

### Function Calls

| Opcode | Operands | Description |
|--------|----------|-------------|
| `Call` | arg_count, method_flag | Call function |
| `VaCall` | arg_count | Variadic call |
| `MethodCall` | arg_count, method_flag | Call method |
| `TailCall` | arg_count | Tail call (optimization) |

### Definitions

| Opcode | Operands | Description |
|--------|----------|-------------|
| `FunctionDefine` | name_id, params, body | Define function |
| `MethodDefine` | name_id, params, body | Define method |
| `ModuleDefine` | name_id | Define module |
| `ClassDefine` | name_id, parent | Define class |
| `LocalDefine` | idx, value | Define local var |
| `StaticDefine` | name_id, value | Define static var |
| `AttributeDefine` | name_id, value | Define property |

### Object/Array Operations

| Opcode | Operands | Description |
|--------|----------|-------------|
| `AttributePush` | name_id | Push attribute reference |
| `AttributeEval` | — | Evaluate attribute |
| `ElementPush` | key | Array/map element |
| `ElementEval` | key | Evaluate element |
| `ArrayPush` | — | Push to array |

### Math Operators

| Opcode | Operands | Description |
|--------|----------|-------------|
| `BinaryOperator` | op_id | Binary operation (+, -, *, /) |
| `UnaryOperator` | op_id | Unary (-, +, !) |
| `BinaryAssignOperator` | op_id | Compound assignment (+=, -=) |

### Logical

| Opcode | Operands | Description |
|--------|----------|-------------|
| `LogicalAnd` | target | Short-circuit AND |
| `LogicalOr` | target | Short-circuit OR |
| `LogicalNullish` | target | Nullish coalescing (??) |

### Module Operations

| Opcode | Operands | Description |
|--------|----------|-------------|
| `Import` | module_id | Import module |
| `Require` | module_id | Require module |
| `ModuleDefine` | name_id | Define module |

### Advanced (v12+)

| Opcode | Description |
|--------|-------------|
| `AsyncFunction` | Async function |
| `Await` | Await expression |
| `Yield` | Yield to caller |
| `YieldFrom` | Yield from sub-generator |
| `SuperCall` | Super class method call |
| `Spread` | Spread operator (...) |
| `ArrowFunction` | Arrow function syntax |
| `Generator` | Generator function |
| `OptionalChaining` | ?. operator |

## Execution in Engine

```rust
fn exec_frame(&mut self, frame_idx: usize) -> Result<FrameAction, String> {
    let frame = &mut self.call_stack[frame_idx];
    let fi = self.modules.get_frame(frame.frame_index);
    
    loop {
        let insn = &fi.instructions[frame.instruction_ptr as usize];
        
        match &insn.op {
            Opcode::Jump => {
                frame.instruction_ptr = insn.target;
            }
            Opcode::Call => {
                // Push new frame
            }
            Opcode::Return => {
                return Ok(FrameAction::Return(frame.pop()));
            }
            // ... handle all opcodes
        }
    }
}
```

## See Also

- [[20_ADHOC_VM/00_Index|Adhoc VM]]
- [[20_ADHOC_VM/01_VM_Engine|VM Engine]]
- [[10_PC_Port/00_Index|PC Port Index]]
- [[10_PC_Port/11_Menu_UI|Menu UI]] — Widget focus system that uses VM execution
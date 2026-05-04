# ADC Varint Error Debug — Instruction 100 / Offset 0x13588

## Known Facts

| Item | Value |
|---|---|
| Varint error offset | `0x13588` |
| Failing instruction | Local index **100** (within child frame), opcode = `AssignOld` |
| Varint size consumed | **10 bytes** (`0x13588` → `0x13592`) |
| Root frame EOF | `0x1646B` (actual file end) |
| Root frame total instructions | 113 |

---

## Key Distinction — Local vs Root Index

Instruction index 100 is **local to a child code frame**, not the root.

```
Root frame (113 instructions)
  └─ Instruction N = FunctionDefine / MethodDefine
        └─ Child frame  ← error is HERE
              ├─ instruction 0
              ├─ ...
              ├─ instruction 99  ← what is this?
              └─ instruction 100 = AssignOld → varint explodes @ 0x13588
```

---

## How to Find Instruction 99

### Step 1 — Locate the child frame start

Find the parent `FunctionDefine` / `MethodDefine` instruction. Its layout is:

```
[opcode:                1 byte ]
[line number:           varint ]
[function name:         symbol ]
[param count:           varint ]
[param names...                ]
[child instruction count: varint]  ← declared size
[child frame body starts HERE  ]   ← your parsing origin
```

Note the byte offset where the child frame body begins.

### Step 2 — Walk forward to instruction 99

Parse sequentially from the child frame start:

```
for i in 0..99:
    opcode = read_byte()
    line   = read_varint()
    <read opcode-specific operands>
```

At `i = 99`, record: **opcode**, **start offset**, **bytes consumed**.
Instruction 100 must then land exactly at `0x13588`. If it doesn't → the frame is already misaligned before instruction 100.

---

## Likely Culprit — Nested Frame with Wrong Instruction Count

If instruction 99 is itself a `FunctionDefine` / `MethodDefine`:

```
[opcode]
[line varint]
[name / params]
[nested child instruction count: varint]  ← if understated...
[nested child frame body]                 ← ...parser exits too early
[cursor resumes HERE]                     ← mid-stream inside nested body
→ instruction 100's varint read hits garbage → never-ending varint
```

**Understated count = N declared, M actual (M > N):**

```
Bytes 0..end_of_N   ← parser reads and exits
Bytes N+1..M        ← leftover, cursor now pointing inside here
                       ↑ this is what gets misread as instruction 100's operand
```

---

## Quick Binary Check at 0x13588

Check the byte at `0x13588` and surrounding bytes:

| Pattern at / before 0x13588 | Meaning |
|---|---|
| Leading bytes have MSB set (`& 0x80 != 0`) | Cursor landed **mid-varint** inside another instruction's payload |
| Clean opcode byte + readable data | Cursor is correct; the varint itself is corrupt |
| `0xFF` repeated | Buffer padding or corruption |

**The 10-byte varint** (`0x13588`→`0x13592`) means all 9 continuation bytes had `0x80` set — classic sign the cursor is inside another instruction's data, not at a clean boundary.

---

## Recommended Fix Path

1. Parse forward from child frame start, logging `[index, opcode, start_offset, end_offset]` for every instruction.
2. At index 99, check if `end_offset == 0x13588`.
   - **Yes** → instruction 99 is fine; the varint at `0x13588` itself is corrupt.
   - **No** → misalignment happened at or before index 99; find which instruction's operand size is wrong.
3. If instruction 99 is a `FunctionDefine`/`MethodDefine`, verify its declared `child instruction count` matches the actual number of instructions in its body.
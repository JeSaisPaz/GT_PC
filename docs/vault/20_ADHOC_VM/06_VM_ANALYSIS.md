# VM Analysis Report - 2026-04-29

## Issues Identified

### 1. **execBoot/execBootPhase2 Skipped Due to Infinite Loops**

Located in `main.rs` lines 524-526 and 588:
- `execBoot()` has infinite loop in V12 bytecode
- `execBootPhase2()` also has infinite loop
- Both are manually skipped in the native main loop

### 2. **Root Cause Analysis**

The infinite loops are likely caused by:

#### A. **Missing Foreach Instruction**
- No `Foreach` instruction exists in the decoder (checked `decoder.rs`)
- Foreach loops must be compiled to: Jump/JumpIfFalse + Iterator pattern
- If the iterator pattern isn't correctly recognized, loops never exit

#### B. **Jump Target Issues**
- Jump targets are read as `i32` from bytecode (loader.rs lines 344-358)
- The VM execution flow appears correct:
  1. Read instruction at current IP
  2. Advance IP to next instruction
  3. Execute instruction (which may override IP)
- However, if jump targets are miscalculated during compilation/disassembly, loops could jump to wrong locations

#### C. **LogicalAnd/LogicalOr Short-Circuit**
- Lines 689-703 in engine.rs
- These are used for control flow in loops
- If targets are wrong, short-circuit may jump incorrectly

#### D. **Require/Import Are No-Ops**
- Lines 1018-1023: Both do nothing
- If the bytecode expects module loading to happen, subsequent code may fail

### 3. **Current Workaround**

The native main loop manually skips execBoot/execBootPhase2:
```rust
// main.rs lines 525-532
eprintln!("[Game] execBoot() skipped (VM bootstrap has infinite loop in initArgs)");
// Manually walk through what execBoot does:
//   initModules() → all modules are native stubs
//   initArgs() → AppOpt is unused in native mode
//   initSpecDB() → SpecDB was loaded above
//   initMenuSystem() → MGOM is a native stub
//   initNetwork() → PDINetwork is a native stub
```

### 4. **Impact on Main Loop**

The main loop IS executing, but with limitations:
- ✅ VM loads and executes Application.adc
- ✅ packed_main_loop.adc loads (registers 163 global functions)
- ✅ MenuClassDefine.adc loads (UI widget classes)
- ✅ config/gt5m.adc loads (configuration)
- ✅ init_sound.adc loads (sound system)
- ❌ execBoot infinite loop - SKIPPED
- ❌ execBootPhase2 infinite loop - SKIPPED
- ❌ Some menu interactions may not work (project scripts depend on bootstrap state)

### 5. **Evidence VM is Working**

From the boot sequence in main.rs:
1. `bootstrap.adc` loads successfully
2. `packed_main_loop.adc` loads → "{} global functions" printed
3. `MenuClassDefine.adc` loads
4. `config/gt5m.adc` loads
5. `init_sound.adc` loads
6. Native main loop starts and runs

### 6. **Recommendation**

The VM is working for the native main loop's purposes because:
- All critical functions are native stubs
- The game logic runs in native Rust (main_loop.rs)
- Project scripts (arcade, race) are loaded as needed

However, to fix the infinite loops for full compatibility:

1. **Add VM trace logging** to identify the exact stuck instruction:
   ```rust
   vm_engine.trace = true;
   ```

2. **Check jump target calculations** - the GTAdhocToolchain may emit relative targets but the VM expects absolute

3. **Implement iterator protocol** - foreach loops likely use an iterator pattern that needs proper state management

4. **Verify bytecode version compatibility** - V12-specific instructions may have different semantics

### 7. **Files Affected**

- `src/vm/engine.rs` - VM execution logic
- `src/vm/loader.rs` - Bytecode loading
- `src/vm/decoder.rs` - Instruction decoding
- `src/main.rs` - Boot sequence (workarounds in place)

### 8. **Testing**

To diagnose the exact issue:
```bash
cd pc_port
cargo run --release -- --trace scripts/bootstrap.adc 2>&1 | head -1000
```

This will show the instruction trace and identify where the loop occurs.

# PPSSPP Runtime Debugger Guide for GT PSP EBOOT Analysis

This guide explains how to use PPSSPP's built-in debugger to locate the VFS load function, heap allocation, and other key addresses in the EBOOT — without needing Ghidra.

## Prerequisites

- PPSSPP installed at `C:\Program Files\PPSSPP\`
- GT PSP ISO or extracted EBOOT
- The mod loader's `build_modded_core.ps1` already deployed (so the mod hooks fire known events)

## 1. Enable PPSSPP Debugger

1. Launch PPSSPP
2. **Settings → System → Enable Debugger → ON** (you may need to enable developer options first)
3. Load GT PSP
4. Press **Ctrl+B** to open the breakpoint dialog
5. Press **Ctrl+Alt+D** to open the debugger window

## 2. Key PSP Syscalls for EBOOT Analysis

These are the PSP kernel functions the game calls, which we can set breakpoints on:

| Syscall NID | Name | What We Can Learn |
|---|---|---|
| `0x109F50BC` | `sceIoOpen` | Every file open call — catches GT.VOL access |
| `0x810C84BC` | `sceIoClose` | File close calls |
| `0x6A638D83` | `sceIoRead` | File read calls — data being read from GT.VOL |
| `0x42EC03AC` | `sceIoWrite` | Save data writes |
| `0xD1FF982A` | `sceKernelLoadModule` | PRX module loading |
| `0xDBB2C9CB` | `sceKernelAllocatePartitionMemory` | Heap/memory allocation |
| `0x237DBD4F` | `sceKernelFreePartitionMemory` | Memory deallocation |
| `0x4B9C18FA` | `scePowerGetCpuClockFrequency` | CPU clock reads |
| `0x2C9152A5` | `sceKernelCreateThread` | Thread creation |

## 3. Finding the VFS Load Function

The VFS load function is what resolves `load("bootstrap")` → opens GT.VOL → returns bytecode.

### Method A: Breakpoint on sceIoOpen

1. Set a breakpoint on `sceIoOpen` (syscall NID in the import stub area)
2. Run the game until the boot logo
3. When the breakpoint hits:
   - Look at `a0` register → this is the filename being opened
   - Look at `ra` register → this is the return address (caller)
4. Continue stepping until you see GT.VOL-related paths
5. Note the calling function's address from the stack trace

The calling flow will be:
```
Adhoc::load("bootstrap")
    → resolve to "scripts/gt5m/bootstrap.adc"
    → VFS::openFile("scripts/gt5m/bootstrap.adc")
    → sceIoOpen("disc0:/PSP_GAME/USRDIR/GT.VOL/scripts/gt5m/bootstrap.adc", ...)
```

**Key**: The function that calls sceIoOpen with the GT.VOL path is close to what we need.
The function ABOVE that (that calls the file resolver) is the actual VFS load function.

### Method B: Breakpoint on sceKernelAllocatePartitionMemory

1. Set a breakpoint on memory allocation
2. Look for large allocations (several MB) — these are typically loading scripts or track data
3. Trace the call stack to find the game's memory manager

### Method C: Using Known Script Load Strings

1. After the game boots and the Adhoc system is running, you can trigger loads:
   - Enter Arcade mode → it loads `projects/gt5m/arcade/arcade.adc`
   - Enter options → it loads `projects/gt5m/option/option.adc`
2. Set breakpoints near the VFS and watch for these loads

## 4. PPSSPP Breakpoint Types

| Type | Description | Use Case |
|---|---|---|
| **Execute** | Breaks when code at this address runs | Function entry points |
| **Read** | Breaks when memory at this address is read | Constant values, data structs |
| **Write** | Breaks when memory at this address is written | Variables, save data |

## 5. Cheat Patch Creation

Once you find an address, create a PPSSPP cheat patch:

```
_S UCES-01245
_G Gran Turismo (PSP)

_C0 Your Patch Name
_L 0x08XXXXXX 0xYYYYYYYY
```

Where:
- `0x08XXXXXX` = the RAM address to modify (in user memory space 0x08800000-0x09FFFFFF)
- `0xYYYYYYYY` = the new value to write

### Patch Types

| Type | Format | Example |
|---|---|---|
| Write 32-bit | `_L 0x08XXXXXX 0xYYYYYYYY` | Change a constant |
| Write 16-bit | `_L 0x08XXXXXX 0x0000YYYY` | Change a halfword |
| NOP instruction | `_L 0x08XXXXXX 0x00000000` | Skip an instruction |
| Jump patch | `_L 0x08XXXXXX 0x08000000` | Redirect to new code |

## 6. Memory Scanning

PPSSPP has a memory viewer that lets you search for values:

1. **Ctrl+M** → Memory view
2. Right-click → Search for value
3. Enter known values:
   - `200` (0xC8) → garage size
   - `0x800000` → 8MB heap size
   - `"bootstrap"` → string reference
   - `"GT.VOL"` → archive reference

This works best AFTER the game has loaded and decrypted the EBOOT sections.

## 7. Practical Workflow

Here's the recommended step-by-step:

```
Step 1: Boot the game, pause with Ctrl+P
Step 2: Open the debugger (Ctrl+Alt+D)
Step 3: Add sceIoOpen breakpoint
Step 4: Resume execution
Step 5: Every breakpoint hit:
        - Note the filename (a0 register)
        - Note the caller address (ra register)
        - Continue until you see "bootstrap.adc" or similar
Step 6: Once found, set a permanent breakpoint at the caller
Step 7: Explore the calling function in the debugger
Step 8: Note the function address and add to vfs_addresses.json
```

## 8. Identifying the Adhoc Bytecode Interpreter

The Adhoc interpreter is a large function with a switch-case dispatch table.
To find it:

1. Look for the function that processes `MODULE_DEFINE` opcode
2. The first few instructions of a module load will be: `MODULE_DEFINE`, `STATIC_DEFINE`, etc.
3. Set breakpoints on the interpreter's dispatch loop
4. Watch for specific opcode values being dispatched

Known opcode values from the disassembly:
- `MODULE_DEFINE: main,main` → opcode for defining module namespace
- `STATIC_DEFINE: PROJECT_ROOT_DIR` → static variable definition
- `LOAD_MODULE: bootstrap` → module loading instruction

## 9. Saving Your Findings

After each PPSSPP session, update:

1. `mod_loader/eboot/vfs_addresses.json` — with discovered addresses
2. `mod_loader/eboot/cheat_patches.ini` — with working patch codes
3. `mod_loader/eboot/analysis_guide.md` — with notes on what was found

## 10. Troubleshooting

| Problem | Solution |
|---|---|
| Breakpoints don't hit | The syscall might be inlined; try the import stub address |
| Can't find strings | They may be in encrypted sections; wait for the game to decompress |
| Game crashes with patch | Check address alignment; ensure values are correct endianness |
| Debugger not responding | PPSSPP must be in windowed mode for debugger to work |

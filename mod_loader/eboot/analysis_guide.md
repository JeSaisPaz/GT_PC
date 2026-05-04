# EBOOT Analysis Guide: Finding the Adhoc VFS Load Function

## Objective

Locate the function in EBOOT.BIN that handles Adhoc script loading (`load()`, `module.load()`, `PROJECT.load()`). Once found, we can:

1. **VFS Redirect**: Patch it to load mod scripts from `ms0:/PSP/MODS/` instead of (or in addition to) GT.VOL
2. **Memory Patches**: Increase heap, unlock garage limits
3. **Performance Patches**: Max CPU, skip videos

## Why This Matters

The game's script system (`Application.ad` → `load("bootstrap")`, `PROJECT.load("ScriptName")`) reads from GT.VOL. To load mods dynamically from the PPSSPP memory stick without rebuilding GT.VOL, we need to intercept/intercept this function.

## EBOOT Structure

| Region | Offset | Size | Status |
|---|---|---|---|
| PRX Header | 0x00000 | 0x80 (128 bytes) | **Readable** — magic, module name "PDIAPP" |
| ELF Header + Sections | 0x00080 | ~6.7 MB | **ENCRYPTED** (PSP KIRK retail encryption) |

The EBOOT uses full PSP retail encryption. **You cannot read strings or code directly from the raw binary.** You MUST use Ghidra (which has built-in PSP decryption) or PPSSPP runtime analysis.

## Approach A: Ghidra Decompilation (Offline Analysis)

### Step 1: Launch Ghidra

```powershell
# GUI mode:
.\workflow\ghidra_12.0_PUBLIC\ghidraRun.bat

# Headless mode (automated):
.\workflow\ghidra_12.0_PUBLIC\support\analyzeHeadless.bat ^
    ghidra_projects gt_psp_eboot -import ^
    "files\original\Gran Turismo\PSP_GAME\SYSDIR\EBOOT.BIN" ^
    -processor "MIPS:little:32:default" ^
    -postScript mod_loader\eboot\ghidra_headless_analyze.py
```

### Step 2: Import the EBOOT

1. **File → New Project → Non-Shared Project**
2. **File → Import File → `<project>/EBOOT.BIN`**
3. Language: **"MIPS R4000, little-endian"** / "MIPS:little:32:default"
4. Ghidra detects PSP PRX format and auto-decrypts sections
5. **Yes** to analyze (auto-analysis will find strings, functions, etc.)

### Step 3: Find Key Strings

After Ghidra's auto-analysis decompresses all sections, search for:

| String | Purpose | How It's Used |
|---|---|---|
| `"bootstrap"` | Entry point boot module | `bootstrap_module.load("bootstrap")` |
| `"packed_main_loop"` | Main game loop | `__module__.load("packed_main_loop")` |
| `"bootstrap_phase2"` | Secondary init | `bootstrap2_module.load("bootstrap_phase2")` |
| `"shutdown"` | Game shutdown | `shutdown_module.load("shutdown")` |
| `".adc"` | Script extension | Used to build filenames |
| `"GT.VOL"` | Archive reference | Archive mount point |
| `"projects/gt5m/"` | UI project root | `PROJECT_ROOT_DIR` in bootstrap.ad |
| `"scripts/gt5m/"` | Script root | Script lookup path |

**How to find in Ghidra:**
- **Search → Program Text** or **Search → Memory**
- Search for each string above
- **Right-click the string → Show References to Address** → find cross-references (XREF)
- The function that references the string is either:
  - The VFS load function itself
  - A helper that builds paths before calling sceIoOpen

### Step 4: Identify the VFS Load Function

The function we're looking for likely has this flow:

```c
// Pseudocode for the Adhoc module loader:
int adhoc_load_script(const char* module_name, const char* script_name)
{
    char path[256];
    
    // Build the full path
    if (script_name starts with "/") {
        strcpy(path, script_name + 1);  // absolute: "/scripts/gt5m/util/GamePlanImpl"
    } else {
        sprintf(path, "scripts/gt5m/%s.adc", script_name);  // relative: "bootstrap"
    }
    
    // Open from GT.VOL (VFS layer)
    int fd = sceIoOpen(path, ...);
    if (fd < 0) {
        // Try alternative paths or error
        return ERROR;
    }
    
    // Read, parse, and register bytecode
    read_and_register_bytecode(fd, module_name);
    return OK;
}
```

**Signature hints:**
- Takes 2-3 string parameters (module name, script path, flags)
- Calls `sceIoOpen` internally (NID `0x109F50BC`)
- Contains string concatenation logic (building paths)
- Processes bytecode headers and registers modules

### Step 5: Trace the Function Chain

```
String: "bootstrap" at 0x08XXXXXX
    ↓ XREF
Calling function at 0x08XXXXXX+offset
    ↓ (this is the VFS resolver)
Calls internal read function
    ↓
Calls sceIoOpen (psp import stub)
    ↓
PSP kernel opens file from disc/umd
```

### Step 6: Record Addresses

Once identified, record in `vfs_addresses.json`:

```json
{
    "vfs_load_function": {
        "address": "0x08XXXXXX",
        "found": true
    }
}
```

## Approach B: PPSSPP Runtime Debugger (Dynamic Analysis)

Recommended as a complement to Ghidra analysis. See the dedicated guide:

➡️ **`ppsspp_debugger_guide.md`** — step-by-step instructions

**Quick start:**
1. Enable PPSSPP debugger: **Settings → System → Enable Debugger**
2. Load GT PSP
3. Set breakpoint on sceIoOpen import stub
4. Resume execution
5. When breakpoint hits with `a0 = "bootstrap.adc"` → trace the caller

## Approach C: Automated Scanner

Use the Python tools in this directory:

```powershell
# Step 1: Initial scan (shows header info, confirms encryption)
python mod_loader\eboot\eboot_analyzer.py

# Step 2: Run Ghidra headless analysis (after Ghidra import)
python mod_loader\eboot\ghidra_headless_analyze.py --parse results.json
```

## Patching Strategy (Once Found)

### VFS Redirect Patch

Replace the first instruction of the VFS load function with a jump to a trampoline:

```mips
# At the VFS load function entry (replace first instruction):
# Original:   addiu $sp, $sp, -0x30  → save stack
# Patched:    j     trampoline_addr    → jump to our hook
#             nop

# At trampoline (placed in unused memory, e.g., 0x08F00000):
# - Check if the file exists on ms0:/PSP/MODS/
# - If yes: load from ms0:/PSP/MODS/<path>
# - If no:  jump back to original function + 8 (skip patched insn)
```

PPSSPP cheat format:
```
_C0 Redirect Script Load to ms0:/PSP/MODS/
_L 0x08XXXXXX 0x08000000  // jump to trampoline (J-type instruction)
_L 0x08F00000 0x27BDFFD0  // trampoline code starts here
...
```

### Heap Expansion Patch

Find the call to `sceKernelAllocatePartitionMemory` and increase the size parameter:

```
_C0 Extended Memory Heap (32MB)
_L 0x08XXXXXX 0x02000000  // size = 32MB (0x2000000)
```

### Garage Size Patch

Find the constant 200 (`0xC8`) used in garage size check and replace:

```
_C0 Unlimited Garage (2000 cars)
_L 0x08XXXXXX 0x000007D0  // 2000 = 0x7D0
```

## Alternative: Simpler Patches (No VFS Required)

While VFS analysis is in progress, these patches are simpler and can be done now:

```
_C0 Max CPU Clock (333MHz)
_L 0x80000000 0x00000000

_C0 Skip Intro Videos
// Disable the PMF video player

_C0 UMD Speed Unlock
// NOP the sceUmdWaitDriveStat delay
```

## Reference: Key Program Addresses

| Item | Address | Description |
|---|---|---|
| EBOOT Load Base | 0x08800000 | Where EBOOT decrypted sections are loaded |
| User RAM Start | 0x08800000 | User memory begins |
| User RAM End | 0x0A000000 | User memory ends (32MB total) |
| Stack Top | 0x0A000000 | Stack grows downward from here |
| VRAM | 0x04000000 | 2MB video memory |
| Unused memory area | 0x08F00000 | Potential trampoline/code cave location |
| PPSSPP cheat address | 0x08XXXXXX | Memory address to modify |

## Progress Tracking

- [x] EBOOT binary analyzed (format: PSP PRX, encrypted sections)
- [x] Module name identified: **PDIAPP**
- [x] Script load patterns documented (from Application.ad source)
- [x] Automated scanner tools created (eboot_analyzer.py, ghidra_headless_analyze.py)
- [x] PPSSPP debugger guide created (ppsspp_debugger_guide.md)
- [ ] EBOOT loaded in Ghidra and decrypted
- [ ] String 'bootstrap' located in Ghidra
- [ ] VFS load function identified
- [ ] Load function signature documented
- [ ] sceIoOpen caller traced
- [ ] Memory stick redirection patch written
- [ ] Heap/garage size patches written
- [ ] Cheat patches tested in PPSSPP

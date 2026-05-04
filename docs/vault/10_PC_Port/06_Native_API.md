---
tags: [pc-port, rust, native, api]
type: documentation
project: GT PSP PC Port
section: Core Engine
---

# Native API Engine — PC Port

> Native function implementations (`pc_port/src/engine/*.rs`).

## Overview

350 native functions registered to the VM:

| Module | Functions | Description |
|--------|-----------|-------------|
| `pdistd` | 39 | Standard library |
| `pdiext` | 58 | Extended library |
| `gtengine` | 58 | Game engine |
| `menu` | 146 | UI framework (widget stubs) |
| `pdiapp` | 31 | Application utilities |
| `audio` | 18 | Audio playback |

## Registration Pattern

```rust
pub fn register_all(registry: &mut NativeRegistry) {
    pdistd::register_pdistd(registry);
    pdiext::register_pdiext(registry);
    gtengine::register_gtengine(registry, specdb);
    menu::register_menu(registry);
    pdiapp::register_pdiapp(registry);
}
```

## pdistd — Standard Library

| Function | Stub |
|-----------|------|
| `MRandom` | Random number generation |
| `MTime` | Time functions |
| `MLocale` | Localization |
| `MFile` | File I/O |
| `MXml` | XML parsing |
| `MEncryption` | Save encryption |

## pdiext — Extended Library

| Function | Stub |
|-----------|------|
| `MProductInformation` | Get version, environment |
| `LoadLatinFont` | Font loading |
| `MSystemBGM` | Background music |
| `MEngineSound` | Engine sounds |
| `MSaveDataUtilPSP` | Save data |
| `MUnit` | Unit conversion |
| `SuperPortButtonBit` | Input reading |

## gtengine — Game Engine

| Function | Status | Description |
|-----------|--------|-------------|
| `MSpecDB::initialize` | ✅ | Load SpecDB |
| `MSpecDB::getCar*` | ✅ | Car accessors |
| `MSpecDB::getCarName` | ✅ | Car name lookup |
| `MSpecDB::getCarCode` | ✅ | Code by name |
| `MCarParameter` | ✅ | Car config |
| `MCourse` | ✅ | Track access |
| `MRaceParameter` | ✅ | Race setup |
| `MOrganizer` | ✅ | Event organizer |
| `MRaceOperator` | ✅ | Race execution |

### MOrganizer — Game Mode Organizer

Manages game modes (arcade, campaign, etc.):

```rust
// Initialization
registry.register("main,menu,MOrganizer", Rc::new(|_| 
    Value::Object(Rc::new(ObjectInstance {
        class_path: "MOrganizer".to_string(),
        fields: vec![],
    }))
));

// Lifecycle
registry.register("main,menu,MOrganizer,initialize", Rc::new(|_| Value::Void));
registry.register("main,menu,MOrganizer,finalize", Rc::new(|_| Value::Void));
registry.register("main,menu,MOrganizer,start", Rc::new(|_| Value::Void));
registry.register("main,menu,MOrganizer,stop", Rc::new(|_| Value::Void));

// State queries
registry.register("main,menu,MOrganizer,isRunning", Rc::new(|_| Value::Bool(false)));
registry.register("main,menu,MOrganizer,getCurrent", Rc::new(|_| Value::Int(0)));
registry.register("main,menu,MOrganizer,setCurrent", Rc::new(|_| Value::Void));
```

### MRaceOperator — Race Execution

Manages race/arcade mode execution:

```rust
// Initialization
registry.register("main,menu,MRaceOperator", Rc::new(|_| 
    Value::Object(Rc::new(ObjectInstance {
        class_path: "MRaceOperator".to_string(),
        fields: vec![],
    }))
));

// Lifecycle
registry.register("main,menu,MRaceOperator,initialize", Rc::new(|_| Value::Void));
registry.register("main,menu,MRaceOperator,finalize", Rc::new(|_| Value::Void));
registry.register("main,menu,MRaceOperator,start", Rc::new(|_| Value::Void));
registry.register("main,menu,MRaceOperator,stop", Rc::new(|_| Value::Void));

// State
registry.register("main,menu,MRaceOperator,isRunning", Rc::new(|_| Value::Bool(false)));
```

### MSound — Audio System

Manages sound and music:

```rust
// Initialization
registry.register("main,menu,MSound", Rc::new(|_| 
    Value::Object(Rc::new(ObjectInstance {
        class_path: "MSound".to_string(),
        fields: vec![],
    }))
));

// Lifecycle
registry.register("main,menu,MSound,initialize", Rc::new(|_| Value::Void));
registry.register("main,menu,MSound,finalize", Rc::new(|_| Value::Void));
registry.register("main,menu,MSound,start", Rc::new(|_| Value::Void));
registry.register("main,menu,MSound,stop", Rc::new(|_| Value::Void));

// Audio playback
registry.register("main,menu,MSound,playBGM", Rc::new(|_| Value::Void));
registry.register("main,menu,MSound,stopBGM", Rc::new(|_| Value::Void));
registry.register("main,menu,MSound,playSE", Rc::new(|_| Value::Void));
```

## menu — UI Framework

| Function | Stub |
|-----------|------|
| `MMenuGameObjectManager` | Widget tree manager |
| `MRootTransition` | Screen transitions |
| `MMoveActor` | Animation |
| `MFadeActor` | Fade effects |
| `MInterpolator` | Value interpolation |
| `MScriptWatcher` | Async timers |

## audio — Audio System

| Function | Stub |
|-----------|------|
| `BGM` | Background music |
| `SE` | Sound effects |
| `Engine` | Engine sound |

## pdiapp — Application

| Function | Stub |
|-----------|------|
| `CreateGameRecordStructure` | Save structure |
| `XmlUtil` | XML helpers |

## Native Function Signature

```rust
type NativeFn = Rc<dyn Fn(&[Value]) -> Value>;

pub fn register(&mut self, path: &str, func: NativeFn) {
    self.functions.insert(path.to_string(), func);
}
```

## Calling from VM

```rust
// In VM engine.rs:
if let Some(func) = self.natives.get(&path) {
    let result = func(&args);
    stack.push(result);
}
```

## Example: SpecDB Field Accessor

```rust
// In gtengine.rs:
registry.register("main,gtengine,MSpecDB,getCarPrice", Rc::new(move |args| {
    let idx = args[0].as_i32() as usize;
    let sd = specdb.borrow();
    if let Some(t) = sd.get_table("GENERIC_CAR") {
        let row = &t.rows[idx];
        let price = read_u32_le(row, 8);
        return Value::Int(price as i32);
    }
    Value::Int(0)
}));
```

## Debug Commands

| Command | Purpose |
|---------|---------|
| `--trace <file>` | Print instructions |
| `--log-native` | Log FFI calls |
| `--list-native` | List all functions |
| `--call <path>` | Call function |

## See Also

- [[20_ADHOC_VM/00_Index|Adhoc VM Index]]
- [[10_PC_Port/00_Index|PC Port Index]]
- [[10_PC_Port/11_Menu_UI|Menu UI]] — UI framework using these natives
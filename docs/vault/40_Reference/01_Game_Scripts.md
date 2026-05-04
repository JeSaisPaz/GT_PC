---
tags: [gt-game, adhoc, scripts, source]
type: reference
game: gt-psp
section: Game Source
---

# GT PSP Game Scripts — Reference

> Adhoc source code from `source/scripts/gt5m/`.

## Core Scripts

| File | Purpose | Key Exports |
|------|---------|-------------|
| `Application.ad` | Entry point | Bootstrap loader |
| `bootstrap.ad` | Game init | `manager`, `sound`, `ORG`, `RaceOperator`, `EventLoop` |
| `main_loop.ad` | Main loop | `MainLoop()` |
| `bootstrap_phase2.ad` | Phase 2 init | `initOrganizer()`, `initRaceOperator()` |
| `init_sound.ad` | Audio init | CarSound system |
| `shutdown.ad` | Cleanup | `execShutdown()` |

## Data Structures

### GameSequence (`GameSequence.ad`)

```adhoc
module ::main::GameSequence {
    static UNDEFINED = 0;
    static MENU = 1;
    static SINGLE_RACE = 2;
    // static ONLINE_BATTLE = 3;  // Not in native code
    static RACE = 4;
    // static REPLAY_THEATER = 5;  // Not in native code
    // static LEAVE_DEMO = 6;      // Not in native code
    
    var st = STStructure();
    st.define("finished", STInt());
    st.define("current_sequence", STInt());
    st.define("current_mode", STInt());
    st.define("current_project", STString(16));
    st.define("arg", STString(32));
}
```

### GameContext (`GameContext.ad`)

```adhoc
module GameContext {
    function CreateStructure() {
        var st = STStructure("Impl");
        st.define("_car", STObject(gtengine::MCarParameter));
        st.define("course", STULong());
        st.define("game_mode", STByte());
        st.define("course_id", STString(16));
        st.define("race_difficulty", STByte());
        st.define("assist_asm", STByte());
        st.define("assist_tcs", STByte());
        // ... more fields
    }
}
```

### GlobalStatus (`global_status/*.ad`)

| File | Structure |
|------|-----------|
| `UserProfile.ad` | Player save data |
| `GameOption.ad` | Settings |
| `GameConfig.ad` | Config limits |

## Bootstrap Flow

```
Application.ad
  ├── load "bootstrap.ad"
  │   └── Creates singletons
  │       ├── main::manager    (MWidgetManager)
  │       ├── main::sound     (MSound)
  │       ├── main::ORG        (MOrganizer)
  │       └── main::RaceOperator
  ├── load "packed_main_loop"
  │   └── MainLoop() dispatch
  ├── load "bootstrap_phase2"
  │   └── initOrganizer, initRaceOperator
  └── load "shutdown.ad"
```

## Engine API (native bridge)

| Module | Purpose | Functions |
|--------|---------|-----------|
| `main::pdistd.*` | Std lib | MRandom, MTime, MFile |
| `main::pdiext.*` | Extended | MProductInformation, MFont |
| `main::gtengine.*` | Game | MSpecDB, MOrganizer, MCarParameter |
| `main::menu.*` | UI | MWidget, MInterpolator |

## Boot Project

```
boot.ad
  ├── Load save data
  ├── Process vouchers
  ├── Show intro movie
  ├── GT logo animation
  └── Jump to menuinit
      └── arcade (default)
```

## Race Events

Sequence: `MENU → SINGLE_RACE → RACE → MENU`

```adhoc
main::GameSequence::setNextSequence(RACE);
main::GameSequence::setNextProject("arcade");
```

## See Also

- [[40_Reference/00_Index|Reference]]
- [[20_ADHOC_VM/00_Index|Adhoc VM]]
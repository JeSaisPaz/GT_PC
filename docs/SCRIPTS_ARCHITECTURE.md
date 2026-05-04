# Gran Turismo PSP — Scripts Architecture & Modding Guide

## Table of Contents
1. [Source Tree Overview](#1-source-tree-overview)
2. [Engine ↔ Script Bridge](#2-engine--script-bridge)
3. [Boot Sequence](#3-boot-sequence)
4. [Core Scripts (scripts/gt5m/)](#4-core-scripts-scriptsgt5m)
5. [Utility Scripts (scripts/gt5m/util/)](#5-utility-scripts-scriptsgt5mutil)
6. [Global Status Data Structures](#6-global-status-data-structures)
7. [Project Architecture Pattern](#7-project-architecture-pattern)
8. [Project Reference](#8-project-reference)
9. [Dependency Graph](#9-dependency-graph)
10. [Modding Guide](#10-modding-guide)

---

## 1. Source Tree Overview

```
source/
├── scripts/gt5m/                    → Core game logic (loaded into ::main)
│   ├── Application.ad               → Entry point
│   ├── bootstrap.ad                 → Phase 1: singletons, core classes
│   ├── bootstrap_phase2.ad          → Phase 2: resident projects, config
│   ├── main_loop.ad                 → Main game loop (includes all core .ad)
│   ├── shutdown.ad                  → Cleanup sequence
│   ├── init_sound.ad                → Sound system init + CarSound classes
│   ├── road_sound_autogen.ad        → Auto-generated road surface audio data
│   ├── GameSequence.ad              → State machine (MENU/RACE/REPLAY, etc.)
│   ├── GameContext.ad               → Runtime car/course/settings state
│   ├── GamePlan.ad                  → Race event plan class definitions
│   ├── GlobalStatusEntry.ad         → Save data bridge
│   ├── SequenceUtil.ad              → Page/project transition system
│   ├── SoundUtil.ad                 → BGM/SFX playback manager
│   ├── DialogUtil.ad                → Confirm/error dialog helper
│   ├── WatcherUtil.ad               → Timer/ticker system
│   ├── ActorUtil.ad                 → Menu animation helpers
│   ├── RaceMenuUtil.ad              → Pre-race menu icon/listbox init
│   ├── RandomUtil.ad                → Fisher-Yates shuffle, random car color
│   ├── TireUtil.ad                  → Tire category/index mapping
│   ├── LeaveDetectUtil.ad           → Idle/away detection
│   ├── Debug.ad                     → Stub debug tools (all empty)
│   ├── global_status/
│   │   ├── GlobalStatus.ad          → Save data wrapper (pack/unpack)
│   │   ├── UserProfile.ad           → Player persistent data struct
│   │   ├── GameOption.ad            → Settings struct
│   │   └── GameConfig.ad            → Config limits struct
│   └── util/
│       ├── SaveDataUtilPSP.ad       → Full save/load system
│       ├── SpecDatabaseUtil.ad      → Car spec text builder
│       ├── GamePlanImpl.ad          → GamePlan → RaceParameter builder
│       ├── ArcadeDifficultyUtil.ad  → AI skill/boost tables
│       ├── LicenseUtil.ad           → License category constants
│       ├── EventFlagsUtil.ad        → Event/message flag bitfields
│       ├── RewardUtil.ad            → Race reward calculator
│       ├── MakerUtil.ad             → Car maker name filter
│       ├── OrdinalUtil.ad           → Ordinal number formatter
│       ├── VoucherUtil.ad           → DLC/voucher car handling
│       └── USBPSPCommPSP.ad         → USB export/import
│
├── projects/gt5m/                   → UI screen projects (15 total)
│   ├── menuinit.ad                  → Central boot project loader
│   ├── arcade/                      → Arcade mode (27 source files)
│   ├── boot/                        → Boot/startup sequence
│   ├── config/                      → Config placeholder (empty)
│   ├── cursor/                      → Page transitions (logo, color, X-fade)
│   ├── detail/                      → Detail popups (car, course, race, etc.)
│   ├── dialog/                      → Confirm/execution dialogs
│   ├── gtmode/                      → GT Mode top menu
│   ├── install/                     → System patch install
│   ├── manual/                      → Digital manual (12 languages)
│   ├── option/                      → Options/settings
│   ├── play_movie/                  → Movie player
│   ├── race/                        → Race HUD, results, modes (18 files)
│   ├── ranking/                     → Leaderboards
│   ├── share/                       → Shared EventLoop class + car render helpers
│   └── ui_kit/                      → Generic selection modal
│
└── products/gt5m/script/
    └── MenuClassDefine.ad           → Widget class registration
```

---

## 2. Engine ↔ Script Bridge

All game logic lives in Adhoc scripts. The native C++ engine exposes APIs through these namespace modules:

| Module Path | Purpose | Examples |
|---|---|---|
| `main::pdistd::*` | Standard library | `MRandom`, `MSound`, `MLocale`, `MFile` |
| `main::pdiext::*` | Extended library | `MProductInformation`, `MFont`, `MUSB`, patching |
| `main::pdiapp::*` | Application library | `MGameRecord`, `MXmlParser` |
| `main::gtengine::*` | Game engine | `MSpecDB`, `MOrganizer`, `MRaceOperator`, `MCarParameter`, `MCourse`, `MGameMode` |
| `main::menu::*` | Menu widgets | `MActor`, `MInterpolator`, `MScriptWatcher`, `MColorInterpolator` |

Key singleton objects created in `bootstrap.ad`:

```
::main::manager      → MManager (widget management)
::main::sound        → MSound (audio system)
::main::ORG          → MOrganizer (race event organizer)
::main::RaceOperator → MRaceOperator (race execution)
::main::GlobalStatus → Save data manager
```

---

## 3. Boot Sequence

```
Application.ad
  │
  ├── load "bootstrap.ad"
  │   └── Creates all global singletons
  │
  ├── load "packed_main_loop"  (maps to main_loop.ad which #include-s all core scripts)
  │   ├── MainLoop()
  │   │   ├── LoadFont()
  │   │   ├── LoadMenuResource()
  │   │   ├── execBootRace()       → plays boot project
  │   │   └── dispatches GameSequence
  │   │
  │   └── Core utils via #include:
  │       ActorUtil, Debug, DialogUtil, GameContext, GamePlan,
  │       GameSequence, GlobalStatusEntry, LeaveDetectUtil,
  │       RaceMenuUtil, RandomUtil, SequenceUtil, SoundUtil,
  │       TireUtil, WatcherUtil
  │
  ├── load "bootstrap_phase2.ad"
  │   ├── initResidentProject()    → loads dialog project
  │   ├── initConfig()             → config from XML
  │   ├── initOrganizer()          → MOrganizer setup
  │   ├── initRaceOperator()       → MRaceOperator setup
  │   ├── initSound()              → sound_context build
  │   └── initMemoryAssignment()   → memory partitioning
  │
  └── load "shutdown.ad"
      └── execShutdown() → finalizes all subsystems
```

**Boot project flow:**

```
boot.ad (BootProject)
  ├── Reads save data     → SaveDataUtilPSP::load()
  ├── Processes vouchers  → VoucherUtil::processCode()
  ├── Shows intro movie   → play_movie project
  ├── GT logo animation   → cursor::GTLogoTransition
  └── Jumps to menuinit   → after_boot_project
        │
        └── menuinit.ad
              ├── require "MenuClassDefine"   → register widget classes
              ├── #include "share/menu.ad"    → EventLoop class
              └── Opens arcade project        → start arcade::TopRoot
```

**GameSequence state machine** (defined in `GameSequence.ad`):

```
MENU ──→ SINGLE_RACE ──→ RACE ──→ MENU
         ONLINE_BATTLE ──→ RACE ──→ MENU
         REPLAY_THEATER ──→ RACE ──→ MENU
  │
  └──→ LEAVE_DEMO (idle attract mode)
```

---

## 4. Core Scripts (scripts/gt5m/)

These are loaded into the `::main` module at boot. They form the backbone of all game logic.

| File | Module | Lines | Purpose | Key Exports |
|---|---|---|---|---|
| `Application.ad` | `::main` | ~50 | Entry point. Loads bootstrap, main_loop, bootstrap_phase2, shutdown sequentially. | — |
| `bootstrap.ad` | `::main` | ~200 | Creates singletons: manager, sound, RaceOperator, ORG. Defines EventLoop class. | `manager`, `sound`, `RaceOperator`, `ORG`, `after_boot_project`, `menu::EventLoop` |
| `bootstrap_phase2.ad` | functions | ~100 | initResidentProject(), initConfig(), initOrganizer(), initRaceOperator(), initSound(), initMemoryAssignment() | — |
| `main_loop.ad` | `::main` | ~200 | MainLoop() function. Loads fonts, manages menu resources, executes boot race, dispatches game sequence. | `MainLoop()`, `LoadMenuResource()`, `UnloadMenuResource()`, `execBootRace()` |
| `shutdown.ad` | functions | ~50 | Finalizes ORG, PDINetwork, GlobalStatus, dialog project, MGOM, MSpecDB. | `execShutdown()` |
| `GameSequence.ad` | `::main::GameSequence` | ~150 | State machine: MENU, SINGLE_RACE, ONLINE_BATTLE, RACE, REPLAY_THEATER, LEAVE_DEMO. Tracks current/prev sequence, mode, project. | `context` (STStructure), sequence constants |
| `GameContext.ad` | `GameContext` | ~80 | Runtime state: car parameters, course, game mode, driving assists, physics, tires. | `CreateStructure()` |
| `GamePlan.ad` | class | ~120 | Race event plan: PlayStyle, Regulation, Opponent, Event, Reward classes. | `GamePlan` |
| `GlobalStatusEntry.ad` | `::main::GlobalStatus` | ~100 | Save data bridge: initialize, finalize, checkout/checkin, load/save. | `initialize()`, `finalize()`, `checkout()`, `checkin()`, `setLoadedData()` |
| `init_sound.ad` | `::main` | ~300 | CarSound module: RoadAttributeSoundParameter, SoundEffect, SoundEffectInstrument, SoundEffectControl. | `sound_context`, `race_sound`, `drc_preset`, `sound_runtime_parameter` |
| `road_sound_autogen.ad` | data | ~1000+ | Auto-generated road surface sound parameters. 16 RoadAttributeSoundParameter entries with SoundEffect arrays. | `road_attribute_sound_parameter` |
| `SequenceUtil.ad` | `::main::SequenceUtil` | ~80 | Page transition system: finder, push/pop, modal, start project. | `setTransitionFinder()`, `setPageTransition()`, `findPageTransition()`, `ModalPage()`, `pushPage()`, `popPage()`, `startPage()`, `startProject()` |
| `SoundUtil.ad` | `::main::SoundUtil` | ~200 | BGM/SFX manager: scene-based BGM, fading, BGM groups, race/menu volume. | `BGMFadeout()`, `RaceBGMPlayGroup()`, `PlayMenuBGM()`, `GetMovieVolume()` |
| `DialogUtil.ad` | `::main::DialogUtil` | ~60 | Opens confirm dialogs: OK, Query, Error, YesNo, YesNoClose, Abort. | `openConfirmDialog()`, `sayOKConfirmDialog()`, `cancelConfirmDialog()` |
| `WatcherUtil.ad` | `::main::WatcherUtil` | ~60 | MScriptWatcher wrapper, Suspender helper for async waits. | `Create()`, `Delete()`, `Suspender` class |
| `ActorUtil.ad` | `::main::ActorUtil` | ~60 | Menu animation: SetMoveActor, ResetInterpolators, ResetActors for MMoveActor/MInterpolator/MActor. | `SetMoveActor()`, `ResetInterpolators()`, `ResetActors()` |
| `RaceMenuUtil.ad` | `::main::RaceMenuUtil` | ~80 | Pre-race menu icon management. Icon class, listbox initialization. | `Icon` class, `createIcon()`, `initialize_icon()`, `initialize_listbox()` |
| `RandomUtil.ad` | `RandomUtil` | ~50 | Fisher-Yates shuffle, random car color from MSpecDB. | `RandomSequenceList()`, `GetRandomVariationOfCar()` |
| `TireUtil.ad` | `TireUtil` | ~40 | Tire category mapping: Tarmac/Dirt/Snow/NoOffset → index range. | `getIndexRangeFromCategory()`, `getCategoryFromIndex()`, `getTireName()` |
| `LeaveDetectUtil.ad` | `LeaveDetectUtil` | ~40 | Idle detection. Watches time_after_last_input, triggers callback. | `begin()`, `end()`, `on_tick()` |
| `Debug.ad` | `::main::DebugTool`, `::main::CheckVersion` | ~40 | All stubs: breakDialog, putLog, printHeapStatus, Assert, Test (all empty). | — |

---

## 5. Utility Scripts (scripts/gt5m/util/)

Loaded on-demand by projects via `PROJECT.load()`. Not included at boot.

| File | Module | Lines | Purpose | Key Exports |
|---|---|---|---|---|
| `SaveDataUtilPSP.ad` | `SaveDataUtilPSP` | ~500 | **Full save/load system.** Modes: FIXED, AUTO, SILENT_AUTO, LIST. Saves to Memory Stick. | `MODE`, `RETCODE`, `GameDataForSave`, `GameDataForLoad`, `save()`, `load()` |
| `SpecDatabaseUtil.ad` | `SpecDatabaseUtil` | ~200 | Builds car spec text from MSpecDB: power (PS/HP), torque, mass, displacement, dimensions. | `GetTextDataCarSpec()` |
| `GamePlanImpl.ad` | functions | ~2000+ | Converts GamePlan → RaceParameter. Builds all driver/race parameters. | `createRaceParameter()`, `createDriverParameter()`, `createRaceBuildParameter()` |
| `ArcadeDifficultyUtil.ad` | `ArcadeDifficultyUtil` | ~200 | AI skill & boost tables by difficulty (EASY–EXTREME) and rank (1–40). | `getAISkillByDifficulty()`, `getAISkillByRank()` |
| `LicenseUtil.ad` | `LicenseUtil` | ~100 | License category constants (A–Q, 17 categories). | `CATEGORY`, `GetLicenseCountOfCategory()`, `GetCategoryString()`, `isFirstStage()` |
| `EventFlagsUtil.ad` | `EventFlagsUtil` | ~80 | Event/message flag bitfield manipulation. | `FLAGS` constants, `setEventFlagON/OFF()`, `setMessageFlagON/OFF()` |
| `RewardUtil.ad` | `RewardUtil` | ~60 | Race reward calc: course length × laps × difficulty × penalty. | `calculate()` |
| `MakerUtil.ad` | `MakerUtil` | ~40 | Car maker name filtering (excludes non-display makers). | `excludeNonDisplayMaker()`, `changeNonDisplayMaker()` |
| `OrdinalUtil.ad` | `OrdinalUtil` | ~40 | Ordinal number formatting (1st, 2nd, 3rd) per locale. | `getOrdinalNumber()` |
| `VoucherUtil.ad` | `VoucherUtil` | ~200 | DLC car voucher processing. Maps voucher IDs → car codes. | `VoucherData`, `ResultData`, `processCode()`, `getVoucherCarMap()` |
| `USBPSPCommPSP.ad` | `USBPSPCommPSP` | ~100 | USB export/import for GT5 PSP link. | `exportData()`, `importData()` |

---

## 6. Global Status Data Structures (scripts/gt5m/global_status/)

These are `STStructure`-based binary-serializable data structures for save data.

| File | Type | Fields |
|---|---|---|
| `UserProfile.ad` | `STStructure` | cash, garage (car list with parts), records, calendar, event/message flags, license clears |
| `GameOption.ad` | `STStructure` | language, units (km/h/mph), volume (BGM/SE), key config, driving assists |
| `GameConfig.ad` | `STStructure` | cash limits, limited mode flag |
| `GlobalStatus.ad` | wrapper | pack/unpack, versioning, CRCs |

**Save data flow:** `GlobalStatusEntry → GlobalStatus (wrapper) → UserProfile + GameOption + GameConfig`

---

## 7. Project Architecture Pattern

Every UI screen is a "project." All projects follow the same pattern:

### Project Definition

```c
module ArcadeProject { }

// onLoad is called when the project starts
function onLoad(update_context) {
    // 1. Import core utils
    import main::DialogUtil;
    import main::SequenceUtil;
    // 2. Load dependencies
    PROJECT.load("SaveDataUtilPSP");
    // 3. Start the root page
    main::SequenceUtil::setPageTransition(CursorProject::XEffectTransition);
    ROOT::onInitialize(update_context);
}

// onUnload is called when the project exits
function onUnload(update_context) {
    ROOT::onFinalize(update_context);
}
```

### Page Definition

```c
module ROOT { }

function onInitialize(update_context) {
    // Setup widgets, load data
}
function onFinalize(update_context) {
    // Cleanup
}
function onKeyPress(type, x, y) {
    // Handle input
}
function onActivate(context) {
    // Activated (pushed to top)
}
function onCancel() {
    // Back button pressed
}
```

### Page Navigation

```c
// Simple page start (replaces current page)
main::SequenceUtil::startPage(update_context, optional_arg);

// Push page (maintains stack, back goes to previous)
main::SequenceUtil::pushPage(update_context);

// Pop page (return from push)
main::SequenceUtil::popPage(update_context);

// Modal page (blocks until dismissed)
main::SequenceUtil::ModalPage(context, ROOT);
```

### Transition Effects (CursorProject)

| Effect | Usage |
|---|---|
| `ColorTransition` | Simple color fade |
| `GTLogoTransition` | GT logo page-in/page-out |
| `XEffectTransition` | Cross-fade transition |

---

## 8. Project Reference

### arcade/ — Arcade Mode (27 files, largest)

| File | Module | Purpose | Notable |
|---|---|---|---|
| `arcade.ad` | `ArcadeProject` | Entry point, loads all util scripts | 27 `PROJECT.load` calls |
| `TopRoot.ad` | `ROOT` | Main hub: mode/car/course icons | Uses LeaveDetectUtil for idle |
| `CarRoot.ad` | `ROOT` | Car selection: 5 view modes | 1705 lines — largest single UI |
| `CourseRoot.ad` | `ROOT` | Course selection: category filters | 747 lines |
| `DrivingModeRoot.ad` | `ROOT` | Mode picker: TA/Single/Drift/Online | Uses gtengine::GameMode |
| `DealerRoot.ad` | `ROOT` | Used car dealership | Loads carset XML data, 673 lines |
| `BuyCarRoot.ad` | `ROOT` | Buy car popup + 3D preview | #include "MenuCarUtil.ad" |
| `LicenseRoot.ad` | `ROOT` | License menu: interactive map | 858 lines, #include LicenseMapPath.ad |
| `OnlineRoot.ad` | `ROOT` | Ad-hoc online lobby | 1958 lines — largest arcade file |
| `SelectRoomRoot.ad` | `ROOT` | Room scanning/selection | 667 lines |
| `ShareRoot.ad` | `ROOT` | Car sharing lobby | 690 lines |
| `TradeRoot.ad` | `ROOT` | Trade session UI | 884 lines |
| `LogsRoot.ad` | `ROOT` | Race history | 286 lines |
| `StatusRoot.ad` | `ROOT` | Player stats | 242 lines |
| `ReplayRoot.ad` | `ROOT` | Replay theater | 641 lines |
| `BranchRoot.ad` | `ROOT` | Mode branching: ADHOC/STATUS/THEATER/TRADE | 435 lines |
| `GTTopRoot.ad` | `ROOT` | GT Mode top menu (within arcade) | World Map, Tuning, Garage icons |
| `RaceConfigUtil.ad` | `RaceConfigUtil` | Serializable race config | 459 lines |
| `OnlineUtil.ad` | `OnlineUtil` | Online session setup | 46 lines |
| `PatchLogic.ad` | `PatchLogic` | System patch download/install | 270 lines |
| `MComponent.ad` | `MComponent` | Shared UI components: BalloonTip, OptionMenu, DetailPopup Pane | 541 lines |

### boot/ — Boot Sequence (4 files)

| File | Module | Purpose |
|---|---|---|
| `boot.ad` | `BootProject` | Entry: loads save, vouchers, shows movie, dispatches |
| `BootRoot.ad` | `ROOT` | Boot page: triggers save load + voucher processing |
| `BootProjectUtil.ad` | `BootProjectUtil` | State machine: autosave→op check→GT logo→movie→menuinit |

### cursor/ — Page Transitions (4 files)

| File | Module | Purpose |
|---|---|---|
| `CursorRoot.ad` | `ROOT` | Cursor state: wait/default |
| `ColorTransition.ad` | `ROOT` | Color fade transition (stub) |
| `GTLogoTransition.ad` | `ROOT` | GT logo animation |
| `XEffectTransition.ad` | `ROOT` | Cross-fade transition |

### detail/ — Detail Popups (16 files)

| File | Module | Purpose |
|---|---|---|
| `detail.ad` | `DetailProject` | Entry: loads SpecDB, RewardUtil |
| `CarDetailPopup.ad` / `CarDetailPopupImpl.ad` | `ROOT` | Car detail view (3D + specs) |
| `CarSpecPopup.ad` / `CarSpecPopupImpl.ad` | `ROOT` | Full car spec sheet |
| `CarDescriptionPopup.ad` / `CarDescriptionPopupImpl.ad` | `ROOT` | Car description text |
| `CourseDetailPopup.ad` / `CourseDetailPopupImpl.ad` | `ROOT` | Course info popup |
| `RaceDetailPopup.ad` / `RaceDetailPopupImpl.ad` | `ROOT` | Race settings info |
| `LicenseDetailPopup.ad` / `LicenseDetailPopupImpl.ad` | `ROOT` | License test details |
| `QuickTunePopup.ad` / `QuickTunePopupImpl.ad` | `ROOT` | Pre-race quick tuning |
| `RankingDetailPopup.ad` / `RankingDetailPopupImpl.ad` | `ROOT` | Ranking data popup |
| `MComponent.ad` | `MComponent` | Shared BalloonTip component |

**Pattern**: Each popup has a `Popup.ad` (template/stub) and `PopupImpl.ad` (implementation). This lets projects load the impl conditionally.

### dialog/ — Dialog System (4 files)

| File | Module | Purpose |
|---|---|---|
| `dialog.ad` | `DialogProject` | Entry: fade/move actor setup |
| `ConfirmDialog.ad` | `ROOT` | 7 dialog types: OK, Query, Error, DefaultNo, YesNo, YesNoClose, Abort |
| `ConfirmExecDialog.ad` | `ROOT` | Execution dialog: begin/waiting/success/failed + background |
| `MComponent.ad` | `MComponent` | BalloonTip |

### gtmode/ — GT Mode (2 files)

| File | Module | Purpose |
|---|---|---|
| `gtmode.ad` | `GTModeProject` | Entry: loads detail project |
| `TopRoot.ad` | `ROOT` | GT Mode top screen with license categories |

### install/ — System Patch Install (6 files)

| File | Module | Purpose |
|---|---|---|
| `install.ad` | `InstallProject` | Entry |
| `PatchRoot.ad` | `PatchRoot` | Patch check/apply UI |
| `PatchLogic.ad` | `PatchLogic` | Patch logic (same as arcade version) |
| `ProgressDialog.ad` | `ProgressDialog` | Progress bar + cancel |
| `ProgressUtil.ad` | `ProgressUtil` | Progress dialog wrapper |
| `MComponent.ad` | `MComponent` | BalloonTip |

### manual/ — Digital Manual (14 files)

| File | Module | Purpose |
|---|---|---|
| `manual.ad` | `ManualProject` | Entry |
| `ManualRoot.ad` | `ROOT` | Manual viewer: TOC, page nav, credits (617 lines) |
| `ManualConfig_XX.ad` | (data) | TOC per language: JP, US, GB, FR, DE, IT, ES, PT, NL, RU, KR, TW |

**Note**: `ManualConfig_JP.ad` (464 lines) is the master. All others `#include` it with overrides.

### option/ — Options/Settings (4 files + 2 headers)

| File | Module | Purpose |
|---|---|---|
| `option.ad` | `OptionProject` | Entry |
| `OptionRoot.ad` | `ROOT` | Full options: key config, sound, display, controller (1178 lines) |
| `MComponent.ad` | `MComponent` | Pulldown OptionMenu component |
| `KeyConfig.h` | header | Button code → localization key maps |
| `PSPController.h` | header | PSP controller action lists |

### play_movie/ — Movie Player (2 files)

| File | Module | Purpose |
|---|---|---|
| `play_movie.ad` | `PlayMovieProject` | Gets filename from GameSequence arg |
| `MovieRoot.ad` | `ROOT` | Playback + caption/subtitle |

### race/ — Race Mode (30 files, most complex project)

| File | Module | Purpose |
|---|---|---|
| `race.ad` | `RaceProject` | Entry: loads all modules, manages pause/result flow |
| `RaceRoot.ad` | `ROOT` | Main race HUD: buttons, BGM, pit menu, pause (874 lines) |
| `ResultRoot.ad` | `ROOT` | Race results: timeout, entry list (215 lines) |
| `OnboardMeterRoot.ad` | `OnboardMeterRoot` | HUD: course map, speedo, rev counter (426 lines) |
| `LoadingRoot.ad` | `ROOT` | Loading screen → ORG.enterCourse() |
| `LoadingUtil.ad` | `LoadingUtil` | Async coroutine loading sequence |
| `BoardUtil.ad` | `BoardUtil` | EntryInfo class for leaderboard |
| `DetailUtil.ad` | `DetailUtil` | Pre-race detail helpers |
| `Template.ad` | `Template` | UI template for quick menu items |
| `SoundArcade.ad` | data | Arcade finish BGM mappings |
| `PrizeUtil.ad` | `PrizeUtil` | Prize calculation + reward table |
| `PrizeRoot.ad` / `PrizeRootImpl.ad` | `PrizeRoot` | Prize screen (1103 lines impl) |
| `TrophyRoot.ad` / `TrophyRootImpl.ad` | `TrophyRoot` | Trophy/achievement screen |
| `NetworkEvent.ad` | functions | Network disconnect handler |
| `OnlineUtil.ad` | `OnlineUtil` | Online battle setup (unused?) |

**Race Mode Plugins** (loaded by `race.ad` via `__module__.load`):

| Module | Loads | Purpose |
|---|---|---|
| `ModuleArcade.ad` | ModuleReplay, PrizeUtil, QuickArcadeRootImpl | Single Race |
| `ModuleTimeAttack.ad` | ModuleReplay, QuickTimeAttackRootImpl | Time Attack |
| `ModuleLicense.ad` | ModuleReplay, PrizeRootImpl, QuickLicenseRootImpl, TrophyRootImpl | License Tests |
| `ModuleDriftAttack.ad` | ModuleReplay, QuickDriftAttackRootImpl, PrizeRootImpl | Drift Attack |
| `ModuleReplay.ad` | (none) | Replay base (provides onLoad/onUnload) |
| `ModuleAdhocBattle.ad` | NetworkEvent, PrizeUtil, PrizeRootImpl | Ad-hoc multiplayer |
| `ModuleOnlineBattle.ad` | NetworkEvent (unused?) | Online battle |

**Quick menu files** (pre-race screens, template + impl pattern):

| Template | Impl | Purpose |
|---|---|---|
| `QuickArcadeRoot.ad` | `QuickArcadeRootImpl.ad` | Arcade pre-race |
| `QuickTimeAttackRoot.ad` | `QuickTimeAttackRootImpl.ad` | Time Attack pre-race |
| `QuickLicenseRoot.ad` | `QuickLicenseRootImpl.ad` | License pre-race |
| `QuickDriftAttackRoot.ad` | `QuickDriftAttackRootImpl.ad` | Drift Attack pre-race |

### ranking/ — Leaderboards (3 files)

| File | Module | Purpose |
|---|---|---|
| `ranking.ad` | `RankingProject` | Entry |
| `RankingRoot.ad` | `ROOT` | Course/mode selection, leaderboard display (267 lines) |
| `MComponent.ad` | `MComponent` | Pulldown OptionMenu |

### share/ — Shared Code (2 files)

| File | Module | Purpose |
|---|---|---|
| `menu.ad` | `menu` | **EventLoop class** — the core loop used by all projects. Included by `menuinit.ad`. |
| `MenuCarUtil.ad` | `MenuCarUtil` | Car rendering in menus: camera, light, scene setup |

### ui_kit/ — UI Toolkit (2 files)

| File | Module | Purpose |
|---|---|---|
| `ui_kit.ad` | `UIKitProject` | Entry (empty onLoad/onUnload) |
| `SelectRoot.ad` | `ROOT` | Generic selection modal (147 lines) |

### products/gt5m/script/ — Product Definitions (1 file)

| File | Purpose |
|---|---|
| `MenuClassDefine.ad` | Registers all widget classes with MManager. Defines BasicColormap. Referenced by `menuinit.ad` via `require`. |

---

## 9. Dependency Graph

```
Application.ad
  ├── load bootstrap.ad              → ::main singletons
  ├── load "packed_main_loop"        → #include all core scripts
  │   ├── ActorUtil.ad               → import main::menu
  │   ├── Debug.ad
  │   ├── DialogUtil.ad              → import main::menu
  │   ├── GameContext.ad
  │   ├── GamePlan.ad                → import main::gtengine
  │   ├── GameSequence.ad
  │   ├── GlobalStatusEntry.ad       → #include UserProfile → #include GlobalStatus, GameOption, GameConfig
  │   ├── LeaveDetectUtil.ad         → import main::WatcherUtil
  │   ├── RaceMenuUtil.ad
  │   ├── RandomUtil.ad              → import main::pdistd::MRandom, main::gtengine::MSpecDB
  │   ├── SequenceUtil.ad            → import __projects__::CursorProject
  │   ├── SoundUtil.ad               → import main::sound
  │   ├── TireUtil.ad
  │   ├── WatcherUtil.ad             → import main::menu
  │   ├── init_sound.ad              → CarSound classes
  │   └── road_sound_autogen.ad      → data for init_sound
  ├── load bootstrap_phase2.ad       → init functions
  └── load shutdown.ad               → finalize functions

menuinit.ad (entry point for menu, loaded by boot)
  ├── require "/products/gt5m/script/MenuClassDefine"
  └── #include "share/menu.ad"

BootProject → SaveDataUtilPSP → DialogUtil, CursorProject
            → VoucherUtil → DialogUtil

ArcadeProject → SaveDataUtilPSP, EventFlagsUtil, LicenseUtil, MakerUtil,
                OrdinalUtil, RewardUtil, SpecDatabaseUtil, ArcadeDifficultyUtil
              → DialogUtil, SequenceUtil, SoundUtil, WatcherUtil, etc.

RaceProject → ModuleArcade/ModuleTimeAttack/ModuleLicense/ModuleDriftAttack
            → each loads ModuleReplay + appropriate Quick*RootImpl + PrizeRootImpl
            → DetailUtil, DialogUtil, SequenceUtil, SoundUtil, etc.

DetailProject → RewardUtil, SpecDatabaseUtil
              → (popup library, loaded by race modules + gtmode)

DialogProject → (resident, always available)

CursorProject → (cross-project transitions, imported by SequenceUtil)
```

**Key principles:**
- **No circular dependencies** — clean directed acyclic graph
- **Core scripts** are `#include`d into `main_loop.ad` → become `::main` members
- **Utilities** are loaded on-demand via `PROJECT.load()`
- **Projects** import only what they need from `::main`
- **Cross-project** access goes through `__projects__::ProjectName`

---

## 10. Modding Guide

### 10.1 Adding a New Car

**Files to modify:**
1. `source/scripts/gt5m/util/EventFlagsUtil.ad` — add event flags for the new car
2. `files/decompiled/USRDIR/GT.VOL/specdb/GT_PSP_JP2817/GENERIC_CAR.dbt` — add car spec row
3. `files/decompiled/USRDIR/GT.VOL/specdb/GT_PSP_JP2817/CAR_NAME_*.dbt` — add localized name per language
4. `files/decompiled/USRDIR/GT.VOL/specdb/GT_PSP_JP2817/VARIATION.dbt` — add color variations
5. `files/decompiled/USRDIR/GT.VOL/specdb/GT_PSP_JP2817/MAKER.dbt` — verify maker exists
6. `files/decompiled/USRDIR/GT.VOL/textdata/gt5m/carlist.xml` — add to car roster
7. Car model files in `car/hq/`, `car/race/`, `car/thumbnail/`

### 10.2 Adding a New Track

**Files to modify:**
1. `files/decompiled/USRDIR/GT.VOL/specdb/GT_PSP_JP2817/COURSE.dbt` — add track spec
2. `files/decompiled/USRDIR/GT.VOL/textdata/gt5m/courselist.xml` — add to course list
3. Track data in `crs/` directory — `.ad`, `.cam`, `.cinf`, `.envptr` files
4. `source/projects/gt5m/arcade/CourseRoot.ad` — may need category filter updates

### 10.3 Adding a New Race Event

**Files to modify:**
1. `files/decompiled/USRDIR/GT.VOL/specdb/GT_PSP_JP2817/RACE.dbt` — add race row
2. `files/decompiled/USRDIR/GT.VOL/textdata/gt5m/enemylist.xml` — add AI opponents
3. `source/scripts/gt5m/util/GamePlanImpl.ad` — `createRaceParameter()` maps GamePlan → race params
4. `source/scripts/gt5m/util/RewardUtil.ad` — adjust reward calculation if needed

### 10.4 Adding a New UI Screen

1. **Create a project directory** (or add to existing project):
   - Follow pattern from `arcade/` or `detail/`
   - Create `.ad` file with `module MyProject { }`, `onLoad()`, `onUnload()`
   - Create page files with `module ROOT { }`, `onInitialize()`, etc.

2. **Create build config**:
   - Copy an existing `.yaml` file (e.g., `arcade.yaml`)
   - List all source files in `sources: [...]`
   - Build with: `adhoc build -i source/projects/gt5m/myproject/ -c myproject.yaml -o output/`

3. **Wire into navigation**:
   - Add `import __projects__::MyProject` in the parent
   - Call `main::SequenceUtil::startProject(update_context, MyProject)`

### 10.5 Modifying UI Text/Localization

- **String databases**: `files/decompiled/USRDIR/GT.VOL/textdata/*_StrDB.sdb` (9 languages)
- **XML configs**: `files/decompiled/USRDIR/GT.VOL/textdata/gt5m/*.xml`
- **Manual text**: `source/projects/gt5m/manual/ManualConfig_*.ad`
- **License test text**: `source/projects/gt5m/arcade/LicenseCategoryData.ad`

### 10.6 Modifying Race Physics/AI

- **AI difficulty**: `source/scripts/gt5m/util/ArcadeDifficultyUtil.ad` — skill & boost tables
- **Race parameters**: `source/scripts/gt5m/util/GamePlanImpl.ad` — `createRaceBuildParameter()`
- **Reward scaling**: `source/scripts/gt5m/util/RewardUtil.ad`

### 10.7 Adding New Game Sequence State

1. **`source/scripts/gt5m/GameSequence.ad`** — add sequence constant, update `context`
2. **`source/scripts/gt5m/main_loop.ad`** — dispatch new sequence in `MainLoop()`
3. Create the project for the new sequence UI
4. Wire transitions in parent project

### 10.8 Build Pipeline

```powershell
# Build all projects
.\scripts\build_all.ps1

# Build single project
adhoc build -i source/projects/gt5m/arcade/ -c arcade.yaml -o out/

# Compare recompiled vs original
.\scripts\compare_all.ps1

# Repack GT.VOL
GTPSPVolTools.exe pack -i files/decompiled/USRDIR/GT.VOL/ -o files/original/Gran\ Turismo/PSP_GAME/USRDIR/GT.VOL
```

### 10.9 Common Pitfalls

| Issue | Cause | Fix |
|---|---|---|
| Script not found at runtime | `PROJECT.load("Name")` but `name.adc` missing from project build | Add file to `.yaml` sources |
| Widget not found | Widget class not registered | Check `MenuClassDefine.ad` |
| Save data crash | STStructure field mismatch | Increment `GlobalStatus::VERSION` |
| Cursor transition broken | `SequenceUtil` missing `__projects__::CursorProject` | Add `import __projects__::CursorProject` |
| Page won't open | `ROOT` module missing `onInitialize` method | Implement lifecycle methods |
| Dialog never closes | `ConfirmDialog` callback not implemented | Pass proper callback to `openConfirmDialog()` |

### 10.10 Key Files for Quick Reference

| What you want to change | File |
|---|---|
| Game entry point | `source/scripts/gt5m/Application.ad` |
| Global singletons | `source/scripts/gt5m/bootstrap.ad` |
| Main loop / sequence dispatch | `source/scripts/gt5m/main_loop.ad` |
| Page transitions | `source/projects/gt5m/cursor/` |
| Save/load system | `source/scripts/gt5m/util/SaveDataUtilPSP.ad` |
| Car spec display | `source/scripts/gt5m/util/SpecDatabaseUtil.ad` |
| User profile data | `source/scripts/gt5m/global_status/UserProfile.ad` |
| Game options data | `source/scripts/gt5m/global_status/GameOption.ad` |
| AI difficulty tables | `source/scripts/gt5m/util/ArcadeDifficultyUtil.ad` |
| License test data | `source/projects/gt5m/arcade/LicenseCategoryData.ad` |
| Reward calculation | `source/scripts/gt5m/util/RewardUtil.ad` |
| Race parameter builder | `source/scripts/gt5m/util/GamePlanImpl.ad` |
| Car list XML | `files/decompiled/.../textdata/gt5m/carlist.xml` |
| Course list XML | `files/decompiled/.../textdata/gt5m/courselist.xml` |
| Enemy/AI cars XML | `files/decompiled/.../textdata/gt5m/enemylist.xml` |
| Track data | `files/decompiled/.../crs/` |
| Car models | `files/decompiled/.../car/` |
| UI textures | `files/decompiled/.../piece_gt5m/` |
| Event flags enums | `source/scripts/gt5m/util/EventFlagsUtil.ad` |
| License constants | `source/scripts/gt5m/util/LicenseUtil.ad` |
| DLC/voucher cars | `source/scripts/gt5m/util/VoucherUtil.ad` |
| Sound initialization | `source/scripts/gt5m/init_sound.ad` |
| BGM/SFX playback | `source/scripts/gt5m/SoundUtil.ad` |
| Menu car rendering | `source/projects/gt5m/share/MenuCarUtil.ad` |
| EventLoop class | `source/projects/gt5m/share/menu.ad` |
| Widget class registry | `source/products/gt5m/script/MenuClassDefine.ad` |
| Main menu init | `source/projects/gt5m/menuinit.ad` |

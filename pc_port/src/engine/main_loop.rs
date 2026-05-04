// Native main game loop — replaces packed_main_loop.adc bytecode interpretation.
// Implements the MainLoop() function from main_loop.ad in native Rust,
// bypassing the VM for the core game loop while leaving the VM available
// for project scripts (arcade, race, dialog, etc.).

use crate::vm::engine::Engine;
use crate::vm::value::Value;
use crate::engine::race::RaceState;
use std::rc::Rc;

// ─── Sequence enum (matches GameSequence.ad) ──────────────────
pub const SEQUENCE_UNDEFINED: i32 = 0;
pub const SEQUENCE_MENU: i32 = 1;
pub const SEQUENCE_SINGLE_RACE: i32 = 2;
pub const SEQUENCE_RACE: i32 = 4;

// ─── Race input buttons ──────────────────────────────────────
// Bit positions in KEY_STATE matching the renderer mapping:
const BIT_CROSS: u32 = 10;  // S key — accelerate
const BIT_CIRCLE: u32 = 9;  // D key — brake
const BIT_UP: u32 = 2;      // accelerate
const BIT_DOWN: u32 = 4;    // brake
const BIT_LEFT: u32 = 5;    // steer left
const BIT_RIGHT: u32 = 3;   // steer right
const BIT_QUIT: u32 = 6;    // LShift — quit race

// ─── Main loop phase state machine ───────────────────────────
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum LoopPhase {
    /// Check exit conditions and determine next phase
    CheckConditions,
    /// MENU: allocate replay buffer
    MenuAllocateReplay,
    /// MENU: load menu resources (fonts)
    MenuLoadResource,
    /// MENU: set game mode
    MenuSetMode,
    /// MENU: start project (MGOM.start)
    MenuStartProject,
    /// MENU: run init callbacks (onLoad, onInit)
    MenuRunInit,
    /// MENU: wait for user interaction (MGOM.sync spanning frames)
    MenuSync,
    /// MENU: unload menu resources
    MenuUnloadResource,
    /// MENU: free unused replay buffer
    MenuFreeReplay,
    /// RACE: play before-race BGM
    RaceBGM,
    /// RACE: set game mode
    RaceSetMode,
    /// RACE: execute race — pushes race scripts to VM, runs over many frames
    RaceExecute,
    /// RACE: waiting for race VM scripts to finish
    RaceRunning,
    /// RACE: end replay buffer
    RaceEndReplay,
    /// RACE: execute next game plan event
    RaceExecuteNext,
    /// Clear font cache (end of each loop iteration)
    ClearFontCache,
}

/// Runtime state for the native main loop, persisted across frames.
pub struct MainLoopState {
    pub phase: LoopPhase,
    pub menu_resource_loaded: bool,
    pub game_sequence: GameSequenceCtx,
    pub race_started: bool,
    /// Set when the main loop should terminate
    pub should_exit: bool,
    /// Race gameplay state
    pub race: RaceState,
}

#[derive(Clone)]
pub struct GameSequenceCtx {
    pub finished: bool,
    pub current_sequence: i32,
    pub prev_sequence: i32,
    pub current_project: String,
    pub prev_project: String,
}

impl Default for MainLoopState {
    fn default() -> Self {
        MainLoopState {
            phase: LoopPhase::CheckConditions,
            menu_resource_loaded: false,
            game_sequence: GameSequenceCtx {
                finished: false,
                current_sequence: SEQUENCE_MENU,
                prev_sequence: SEQUENCE_UNDEFINED,
                current_project: "arcade".to_string(),
                prev_project: "undefined".to_string(),
            },
            race_started: false,
            should_exit: false,
            race: RaceState::new(),
        }
    }
}

impl MainLoopState {
    pub fn new() -> Self { Self::default() }

    /// Called once per frame. Advances the main loop state machine.
    /// Returns Ok(true) if game should continue, Ok(false) if game should exit.
    pub fn tick(&mut self, vm: &mut Engine) -> Result<bool, String> {
        // ── 0. Drain newly loaded frames (from MGOM.start script loading, etc.) ──
        // Even if the call stack is empty, new frames may be queued by load_adc_file
        loop {
            let new_frames = crate::vm::loader::drain_loaded_frames();
            if new_frames.is_empty() { break; }
            for cf in new_frames {
                let cf_idx = vm.modules.add_frame(cf);
                let fi = vm.modules.get_frame(cf_idx);
                let ef = crate::vm::frame::Frame::new(cf_idx, fi.stack_size, fi.local_var_count, 0);
                vm.call_stack.push(ef);
            }
        }

        // ── 1. If VM has active script frames (project scripts), run them first ──
        if !vm.call_stack.is_empty() {
            match vm.execute_frame(5000)? {
                Some(_val) => {
                    // VM frame finished — call stack may or may not be empty now
                }
                None => {
                    // VM still has work to do — keep ticking
                    return Ok(true);
                }
            }
        }

        // ── 2. Main loop state machine ──────────────────────────
        // Process as many immediate transitions as we can in one frame,
        // but stop at blocking phases (MenuSync, RaceRunning)
        loop {
            match self.phase {
                LoopPhase::CheckConditions => {
                    // Sync our local state with the shared GameSeq state
                    // (project scripts may have updated it through natives)
                    // Shared state accessor is registered under "main,GameSequence,context"
                    if let Some(nf) = vm.natives.get("main,GameSequence,getCurrentSequence") {
                        let val = nf(&[]);
                        if let Some(seq) = val.as_i32() {
                            self.game_sequence.current_sequence = seq;
                        }
                    }

                    // Check bootrace quick-start (AppOpt["bootrace"])
                    self.check_bootrace(vm)?;

                    // Check exit conditions
                    if self.game_sequence.finished {
                        self.should_exit = true;
                        return Ok(false);
                    }
                    if let Some(nf) = vm.natives.get("main,pdiext,MSystemCondition,IsExitGame") {
                        let exit = nf(&[]);
                        if exit.truthy() {
                            self.should_exit = true;
                            return Ok(false);
                        }
                    }

                    // Debug: print heap status
                    if let Some(nf) = vm.natives.get("main,DebugTool,printHeapStatus") {
                        nf(&[]);
                    }

                    // Determine next phase
                    self.phase = match self.game_sequence.current_sequence {
                        SEQUENCE_MENU => LoopPhase::MenuAllocateReplay,
                        SEQUENCE_RACE => LoopPhase::RaceBGM,
                        _ => LoopPhase::CheckConditions,
                    };
                }

                // ── MENU SEQUENCE ──────────────────────────────
                LoopPhase::MenuAllocateReplay => {
                    if let Some(nf) = vm.natives.get("main,GameSequence,allocateReplayBuffer") {
                        nf(&[]);
                    }
                    self.phase = LoopPhase::MenuLoadResource;
                }

                LoopPhase::MenuLoadResource => {
                    if !self.menu_resource_loaded {
                        // Load all Latin fonts
                        self.load_menu_fonts(vm);
                        self.menu_resource_loaded = true;
                    }
                    self.phase = LoopPhase::MenuSetMode;
                }

                LoopPhase::MenuSetMode => {
                    if let Some(nf) = vm.natives.get("main,GameSequence,setMode") {
                        nf(&[Value::Int(self.game_sequence.current_sequence)]);
                    }
                    self.phase = LoopPhase::MenuStartProject;
                }

                LoopPhase::MenuStartProject => {
                    if let Some(nf) = vm.natives.get("main,MGOM,start") {
                        nf(&[Value::String(Rc::new(self.game_sequence.current_project.clone()))]);
                    }
                    self.phase = LoopPhase::MenuRunInit;
                }

                /// Call project's onInit/onLoad function after script loads.
                LoopPhase::MenuRunInit => {
                    let proj = &self.game_sequence.current_project;
                    let capitalized = capitalize(proj);
                    let names = [
                        format!("{}::onLoad", capitalized),
                        format!("{}::onInitialize", capitalized),
                        "onLoad".to_string(),
                        "onInitialize".to_string(),
                    ];
                    for name in &names {
                        if try_call_global(vm, name, &[]) {
                            break;
                        }
                    }
                    self.phase = LoopPhase::MenuSync;
                }

                LoopPhase::MenuSync => {
                    // Check if project scripts have changed the game sequence
                    // (e.g., user selected "Start Race" → SequenceUtil.startSequence sets RACE)
                    if let Some(nf) = vm.natives.get("main,GameSequence,getCurrentSequence") {
                        let val = nf(&[]);
                        if let Some(seq) = val.as_i32() {
                            self.game_sequence.current_sequence = seq;
                            if seq == SEQUENCE_RACE {
                                crate::engine::ui::with_ui(|ui| ui.finished = true);
                            }
                        }
                    }

                    // Blocking: wait for user interaction to finish
                    let finished = crate::engine::ui::with_ui(|ui| ui.is_finished());
                    if finished {
                        self.phase = LoopPhase::MenuUnloadResource;
                    } else {
                        // Stay in this phase — will check again next frame
                        return Ok(true);
                    }
                }

                LoopPhase::MenuUnloadResource => {
                    if self.menu_resource_loaded {
                        self.unload_menu_fonts(vm);
                        self.menu_resource_loaded = false;
                    }
                    self.phase = LoopPhase::MenuFreeReplay;
                }

                LoopPhase::MenuFreeReplay => {
                    if let Some(nf) = vm.natives.get("main,GameSequence,freeUnusedReplayBuffer") {
                        nf(&[]);
                    }
                    self.phase = LoopPhase::ClearFontCache;
                }

                // ── RACE SEQUENCE ──────────────────────────────
                LoopPhase::RaceBGM => {
                    if let Some(nf) = vm.natives.get("main,SoundUtil,BGMPlayGroup") {
                        nf(&[
                            Value::String(Rc::new("menu".to_string())),
                            Value::String(Rc::new("before_race".to_string())),
                            Value::Bool(true),
                            Value::Float(2.0),
                        ]);
                    }
                    self.phase = LoopPhase::RaceSetMode;
                }

                LoopPhase::RaceSetMode => {
                    if let Some(nf) = vm.natives.get("main,GameSequence,setMode") {
                        nf(&[Value::Int(self.game_sequence.current_sequence)]);
                    }
                    self.phase = LoopPhase::RaceExecute;
                }

                LoopPhase::RaceExecute => {
                    if !self.race_started {
                        // Initialize race state
                        let course_id = 1; // c001
                        let car_code = 0x00010001; // default car
                        self.race.initialize(course_id, car_code);

                        // Call executeRace native — starts race project
                        if let Some(nf) = vm.natives.get("main,GameSequence,executeRace") {
                            nf(&[]);
                        }
                        self.race_started = true;
                    }
                    self.phase = LoopPhase::RaceRunning;
                }

                LoopPhase::RaceRunning => {
                    // ── Read race input from key state ──────────
                    use std::sync::atomic::Ordering;
                    let ks = crate::engine::menu::KEY_STATE.load(Ordering::Relaxed);
                    let throttle = ((ks >> BIT_UP) & 1) != 0 || ((ks >> BIT_CROSS) & 1) != 0;
                    let brake = ((ks >> BIT_DOWN) & 1) != 0 || ((ks >> BIT_CIRCLE) & 1) != 0;
                    let steer_left = ((ks >> BIT_LEFT) & 1) != 0;
                    let steer_right = ((ks >> BIT_RIGHT) & 1) != 0;

                    // ── Update race physics ─────────────────────
                    self.race.update(1.0 / 60.0, throttle, brake, steer_left, steer_right);

                    // ── Render 3D scene ─────────────────────────
                    self.race.render();

                    // ── HUD overlay via UI system ───────────────
                    crate::engine::ui::with_ui(|ui| {
                        if ui.active_project.as_deref() != Some("race") {
                            ui.start_project("race");
                        }
                    });

                    // ── Finish condition ────────────────────────
                    // Check if user pressed Circle to finish, or if race elapsed threshold hit
                    let quit = ((ks >> BIT_QUIT) & 1) != 0;
                    if quit || self.race.finished || self.race.elapsed > 120.0 {
                        if vm.call_stack.is_empty() || quit {
                            self.race_started = false;
                            self.phase = LoopPhase::RaceEndReplay;
                        }
                    } else {
                        return Ok(true);
                    }
                }

                LoopPhase::RaceEndReplay => {
                    if let Some(nf) = vm.natives.get("main,GameSequence,endUsedReplayBuffer") {
                        nf(&[]);
                    }
                    self.phase = LoopPhase::RaceExecuteNext;
                }

                LoopPhase::RaceExecuteNext => {
                    if let Some(nf) = vm.natives.get("main,GamePlan,executeNext") {
                        nf(&[]);
                    }
                    self.phase = LoopPhase::ClearFontCache;
                }

                // ── END OF LOOP ITERATION ──────────────────────
                LoopPhase::ClearFontCache => {
                    if let Some(nf) = vm.natives.get("main,pdiext,ClearFontCache") {
                        nf(&[]);
                    }
                    self.phase = LoopPhase::CheckConditions;
                }
            } // match self.phase
        } // loop
    }

    /// Check for AppOpt bootrace argument (quick-start race).
    fn check_bootrace(&self, vm: &mut Engine) -> Result<(), String> {
        // AppOpt is a module defined by bootstrap.adc bytecode.
        // Its statics are stored in module_static_bases under "AppOpt".
        // The key "bootrace" would be at module_static_bases["AppOpt::bootrace"].
        // Since we run the native main loop after bootstrap/adc loaded AppOpt,
        // the static may already exist. But the ElementEval path (AppOpt["bootrace"])
        // doesn't go through natives — it's handled in exec_insn.
        //
        // For now: skip bootrace in native mode. It's a rare CLI feature.
        Ok(())
    }

    /// Execute the boot-race quick start (AppOpt["bootrace"]).
    fn exec_boot_race(&mut self, vm: &mut Engine, arg: &str) -> Result<(), String> {
        if arg.is_empty() { return Ok(()); }
        let parts: Vec<&str> = arg.split(',').collect();
        if parts.len() < 2 { return Ok(()); }

        let car_label = parts[0];
        let course_label = parts[1];

        // Get car code from SpecDB
        let car_code = if let Some(nf) = vm.natives.get("main,gtengine,MSpecDB,getCarCode") {
            nf(&[Value::String(Rc::new(car_label.to_string()))])
        } else { Value::Int(0) };

        // Get course code from SpecDB
        let course_code = if let Some(nf) = vm.natives.get("main,gtengine,MSpecDB,getCourseCode") {
            nf(&[Value::String(Rc::new(course_label.to_string()))])
        } else { Value::Int(0) };

        // Create car parameter
        if let Some(nf) = vm.natives.get("main,gtengine,MCarParameter,create") {
            nf(&[car_code]);
        }

        // Create driver parameter
        if let Some(nf) = vm.natives.get("main,gtengine,MCarDriverParameter,create") {
            nf(&[]);
        }
        if let Some(nf) = vm.natives.get("main,gtengine,MCarDriverParameter,setPlayer") {
            nf(&[Value::Int(0)]);
        }

        // Begin game plan
        if let Some(nf) = vm.natives.get("main,GamePlan,begin") {
            nf(&[]);
        }

        // Create single race
        if let Some(nf) = vm.natives.get("main,GamePlan,createSingleRace") {
            nf(&[
                course_code,
                Value::Int(1),       // entry_num
                Value::Int(1),       // arcade_laps
                Value::Int(0),       // ai_skill
                Value::Int(0),       // enemy_lv
                Value::Int(0),       // boost_lv
                Value::Int(0),       // penalty_level
                Value::Array(Rc::new(vec![Value::Int(4), Value::Int(3), Value::Int(2), Value::Int(1)])),
            ]);
        }

        // Execute out of sequence
        if let Some(nf) = vm.natives.get("main,GamePlan,executeOutOfSequence") {
            nf(&[]);
        }

        // End game plan
        if let Some(nf) = vm.natives.get("main,GamePlan,end") {
            nf(&[]);
        }

        Ok(())
    }

    /// Load menu fonts (sanserif, serif families).
    fn load_menu_fonts(&self, vm: &mut Engine) {
        let fonts = [
            "sanserif-r", "sanserif-i", "sanserif-b", "sanserif-bi",
            "serif-r", "serif-i", "serif-b", "serif-bi",
        ];
        for font in &fonts {
            if let Some(nf) = vm.natives.get("main,pdiext,LoadLatinFont") {
                nf(&[Value::String(Rc::new(font.to_string()))]);
            }
        }
    }

    /// Unload menu fonts.
    fn unload_menu_fonts(&self, vm: &mut Engine) {
        if let Some(nf) = vm.natives.get("main,pdiext,UnloadLatinFont") {
            nf(&[]);
        }
    }
}

// ─── Helpers ──────────────────────────────────────────────────

fn capitalize(s: &str) -> String {
    let mut chars = s.chars();
    match chars.next() {
        None => String::new(),
        Some(c) => c.to_uppercase().to_string() + chars.as_str(),
    }
}

/// Try to find and call a global function by name. Returns true if found.
fn try_call_global(vm: &mut crate::vm::engine::Engine, name: &str, args: &[Value]) -> bool {
    if let Some(fv) = vm.global_functions.get(name) {
        eprintln!("[MainLoop] Calling global function: {}", name);
        let _ = vm.call_function_value(Value::Function(fv.clone()), args.to_vec());
        true
    } else {
        false
    }
}

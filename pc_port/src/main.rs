/// GT PSP Native PC Port - with native main loop bypass.

/// The main game loop runs natively in Rust, bypassing the VM.

/// Project scripts (arcade, race, dialog, etc.) still execute through the VM.

use std::env;

use std::fs;

use std::rc::Rc;

use std::cell::RefCell;



mod vm;

mod engine;



use vm::value::Value;

use vm::loader::Loader;

use vm::module::ModuleRegistry;

use vm::native::NativeRegistry;

use vm::engine::Engine;

use engine::main_loop::MainLoopState;

use engine::test_tools;



// Console-only build - graphics not required

// #[cfg(windows)]

// use crate::engine::graphics::get_thread_renderer;



const DEFAULT_ASSETS_PATH: &str = "assets/scripts";



fn resolve_path(input: &str) -> String {

    if std::path::Path::new(input).is_absolute() {

        return input.to_string();

    }

    

    let assets_path = std::path::Path::new(DEFAULT_ASSETS_PATH);

    let full_path = assets_path.join(input);

    

    if full_path.exists() {

        full_path.to_string_lossy().to_string()

    } else {

        input.to_string()

    }

}



fn main() {
    // Initialize the assets root — change this to your extracted GT.VOL directory.
    // The path should point to the root of the extracted GT.VOL archive,
    // e.g., ".../Gran Turismo/PSP_GAME/USRDIR/GT.VOL/"
    let default_gt_vol = if cfg!(target_os = "windows") {
        "D:\\GTPSP-decompile\\files\\decompiled\\Gran Turismo\\PSP_GAME\\USRDIR\\GT.VOL"
    } else {
        "assets"
    };
    engine::init_assets_root(default_gt_vol);

    let args: Vec<String> = env::args().collect();

    if args.len() < 2 {

        println!("Usage: adhoc-vm <file.adc> [args...]");

        println!("  Load and execute an Adhoc bytecode (.adc) file.");

        println!("  Files are resolved relative to: {}", DEFAULT_ASSETS_PATH);

        println!("");

        println!("Commands:");

        println!("  adhoc-vm <file.adc>            Execute the file");

        println!("  adhoc-vm --disassemble <file>  Print disassembly");

        println!("  adhoc-vm --dump <file>         Dump file structure");

        println!("  adhoc-vm --specdb              Test SpecDB loading");

        println!("  adhoc-vm --specdb-dump         Dump column schemas");

        println!("  adhoc-vm --test-all            Test all .adc files load correctly");

        println!("  adhoc-vm --test-3ldm            Test 3LDM model parsing");

        println!("  adhoc-vm --test-window          Open test window");

        println!("  adhoc-vm --boot                 Run boot sequence");

        println!("  adhoc-vm --trace <file>         Execute with instruction trace");

        println!("  adhoc-vm --log-native <file>    Execute with native call logging");

        println!("  adhoc-vm --list-native          List all registered native functions");

        return;

    }



    run_cli(&args);

}



fn run_headless_boot() {

    let specdb_path = format!("{}\\specdb\\GT_PSP_JP2817", engine::assets_root());

    eprintln!("[Headless] Loading Application.adc");

    let bytecode = match std::fs::read(resolve_path("Application.adc")) {

        Ok(d) => d, Err(e) => { eprintln!("Error: {}", e); return; }

    };

    let code_frame = match Loader::load(&bytecode) {

        Ok(cf) => cf, Err(e) => { eprintln!("Load error: {}", e); return; }

    };

    let specdb = Rc::new(RefCell::new(engine::specdb::SpecDB::new()));

    let mut natives = NativeRegistry::new();

    
engine::audio::register_audio(&mut natives);

    engine::pdistd::register_pdistd(&mut natives);

    engine::pdiext::register_pdiext(&mut natives);

    engine::pdiapp::register_pdiapp(&mut natives);

    engine::menu::register_menu(&mut natives);

    engine::gtengine::register_gtengine(&mut natives, specdb.clone());

    if let Err(e) = specdb.borrow_mut().load_all(&specdb_path) { eprintln!("SpecDB error: {}", e); }

    let mut registry = ModuleRegistry::new();

    let frame_idx = registry.add_frame(code_frame);

    let mut vm_engine = Engine::new(registry, natives);

    // vm_engine.trace is false by default

    {

        let fi = vm_engine.modules.get_frame(frame_idx);

        let static_base = vm_engine.current_module_index;

        vm_engine.current_module_index += fi.static_var_count.max(1) as usize;

        let frame = vm::frame::Frame::new(frame_idx, fi.stack_size, fi.local_var_count, static_base);

        vm_engine.call_stack.push(frame);

    }

    eprintln!("[Headless] Starting VM (no SDL)");

    let now = std::time::Instant::now();

    loop {

        match vm_engine.execute_frame(10000) {

            Ok(Some(val)) => { eprintln!("[Headless] Done: {:?} in {:?}, {} insns", val, now.elapsed(), vm_engine.get_insn_count()); break; }

            Ok(None) => {}

            Err(e) => { eprintln!("[Headless] Error: {} in {:?}, {} insns", e, now.elapsed(), vm_engine.get_insn_count()); break; }

        }

    }

}



fn run_cli(args: &[String]) {

    if args[1] == "--specdb" {

        test_specdb();

        return;

    }



    if args[1] == "--specdb-dump" {

        let path = format!("{}\\specdb\\GT_PSP_JP2817", engine::assets_root());

        match engine::specdb::SpecDB::load_directory(&path) {

            Ok(sd) => {

                let mut names: Vec<&String> = sd.tables.keys().collect();

                names.sort();

                for name in names {

                    if let Some(t) = sd.get_table(name) {

                        t.print_summary();

                    }

                }

            }

            Err(e) => eprintln!("SpecDB error: {}", e),

        }

        std::process::exit(0);

    }



    if args[1] == "--test-all" {

        test_all_adc_files();

        return;

    }

    if args[1] == "--test-3ldm" {

        test_tools::test_3ldm_parsing();

        return;

    }



    if args[1] == "--list-native" {

        list_native_functions();

        return;

    }



    if args[1] == "--disassemble" {

        if args.len() < 3 { eprintln!("Usage: adhoc-vm --disassemble <file.adc>"); return; }

        load_and_disassemble(&args[2]);

        return;

    }



    if args[1] == "--dump" {

        if args.len() < 3 { eprintln!("Usage: adhoc-vm --dump <file.adc>"); return; }

        load_and_dump(&args[2]);

        return;

    }



    if args[1] == "--test-window" || args[1] == "--test-texture" || args[1] == "--test-sprite" {

        #[cfg(windows)]

        {

            use crate::engine::graphics::{init_renderer, get_thread_renderer};

            init_renderer();

            let renderer = get_thread_renderer();

            eprintln!("SDL2 window opened: 960x544");

            renderer.borrow_mut().clear();

            renderer.borrow_mut().draw_text(10, 10, "GT PSP - Native PC Port", 200, 200, 200, 1.0);

            renderer.borrow_mut().end_scene();

            std::thread::sleep(std::time::Duration::from_secs(2));

            eprintln!("Render loop test complete");

            std::process::exit(0);

        }

        #[cfg(not(windows))]

        {

            eprintln!("Graphics not supported on this platform.");

            return;

        }

    }



    if args[1] == "--boot" {

        run_game_loop("boot");

        return;

    }



    if args[1] == "--headless-boot" {

        run_headless_boot();

        return;

    }



    if args[1] == "--game" {

        if args.len() < 3 { eprintln!("Usage: adhoc-vm --game <file.adc>"); return; }

        run_game_loop(&args[2]);

        return;

    }



    let trace = args.iter().any(|a| a == "--trace" || a == "-t");

    let log_native = args.iter().any(|a| a == "--log-native" || a == "-n");

    

    let file_path = if args[1] == "--trace" || args[1] == "-t" || args[1] == "--log-native" || args[1] == "-n" {

        if args.len() < 3 { eprintln!("Usage: adhoc-vm <file.adc>"); return; }

        &args[2]

    } else {

        &args[1]

    };



    match execute_file_with_flags(file_path, trace, log_native) {

        Ok(result) => { println!("Result: {:?}", result); std::process::exit(0); }

        Err(e) => { eprintln!("Error: {}", e); std::process::exit(1); }

    }

}



#[cfg(windows)]

fn run_game_loop(_file_path: &str) {

    use crate::engine::graphics::{get_thread_renderer, init_renderer};

    use engine::main_loop::MainLoopState;



    let specdb_path = format!("{}\\specdb\\GT_PSP_JP2817", engine::assets_root());

    let assets_path = "assets";



    // ── Engine setup ───────────────────────────────────────────

    let specdb = Rc::new(RefCell::new(engine::specdb::SpecDB::new()));

    let mut natives = NativeRegistry::new();

    
engine::audio::register_audio(&mut natives);

    engine::pdistd::register_pdistd(&mut natives);

    engine::pdiext::register_pdiext(&mut natives);

    engine::pdiapp::register_pdiapp(&mut natives);

    engine::menu::register_menu(&mut natives);

    engine::gtengine::register_gtengine(&mut natives, specdb.clone());



    // Initialize SpecDB

    eprintln!("[Game] Loading SpecDB from: {}", specdb_path);

    {

        let mut sd = specdb.borrow_mut();

        if let Err(e) = sd.load_all(&specdb_path) {

            eprintln!("[Game] SpecDB load error: {}", e);

        } else {

            eprintln!("[Game] SpecDB loaded successfully");

        }

    }



    let registry = ModuleRegistry::new();

    let mut vm_engine = Engine::new(registry, natives);

    init_renderer();

    // Note: OpenGL requires SDL2 with OpenGL context
    // init_opengl can be called after window is ready
    
    let renderer = get_thread_renderer();



    // ── Phase 1: Run bootstrap.adc (execBoot) ──────────────────

    eprintln!("[Game] Phase 1: Loading bootstrap.adc...");

    match vm_engine.load_module("scripts/bootstrap.adc", assets_path) {

        Ok(idx) => {

            eprintln!("[Game] bootstrap.adc loaded (frame idx={})", idx);

        }

        Err(e) => {

            eprintln!("[Game] Error loading bootstrap: {}", e);

            return;

        }

    }



    eprintln!("[Game] Calling execBoot()...");
    // Execute execBoot() from bootstrap.adc - IteratorNext instruction now handles foreach loops
    if let Some(func) = vm_engine.global_functions.get("execBoot") {
        let func_val = vm::value::Value::Function(func.clone());
        eprintln!("[Game] execBoot() VM execution starting...");
        match vm_engine.call_function_value(func_val, vec![]) {
            Ok(_) => {
                // Execute the function
                match vm_engine.execute_frame(1000000) {
                    Ok(Some(_)) => eprintln!("[Game] execBoot() completed successfully"),
                    Ok(None) => eprintln!("[Game] execBoot() finished (no return value)"),
                    Err(e) => eprintln!("[Game] execBoot() error: {}", e),
                }
            }
            Err(e) => eprintln!("[Game] execBoot() setup error: {}", e),
        }
        vm_engine.call_stack.clear();
    } else {
        eprintln!("[Game] execBoot() not found in global functions, skipping");
    }





    // ── Phase 2: Load packed_main_loop.adc (module definitions) ──

    eprintln!("[Game] Phase 2: Loading packed_main_loop.adc...");

    match vm_engine.load_module("scripts/packed_main_loop.adc", assets_path) {

        Ok(idx) => {

            eprintln!("[Game] packed_main_loop.adc loaded (frame idx={}), {} global functions",

                idx, vm_engine.global_functions.len());

        }

        Err(e) => {

            eprintln!("[Game] Error loading packed_main_loop: {}", e);

            return;

        }

    }



    // ── Phase 3: Run bootstrap_phase2.adc (execBootPhase2) ─────

    eprintln!("[Game] Phase 3: Loading bootstrap_phase2.adc...");

    match vm_engine.load_module("scripts/bootstrap_phase2.adc", assets_path) {

        Ok(idx) => {

            eprintln!("[Game] bootstrap_phase2.adc loaded (frame idx={})", idx);

        }

        Err(e) => {

            eprintln!("[Game] Error loading bootstrap_phase2: {}", e);

            return;

        }

    };


    eprintln!("[Game] Calling execBootPhase2()...");
    // Execute execBootPhase2() from bootstrap_phase2.adc
    if let Some(func) = vm_engine.global_functions.get("execBootPhase2") {
        let func_val = vm::value::Value::Function(func.clone());
        eprintln!("[Game] execBootPhase2() VM execution starting...");
        match vm_engine.call_function_value(func_val, vec![]) {
            Ok(_) => {
                // Execute the function
                match vm_engine.execute_frame(1000000) {
                    Ok(Some(_)) => eprintln!("[Game] execBootPhase2() completed successfully"),
                    Ok(None) => eprintln!("[Game] execBootPhase2() finished (no return value)"),
                    Err(e) => eprintln!("[Game] execBootPhase2() error: {}", e),
                }
            }
            Err(e) => eprintln!("[Game] execBootPhase2() setup error: {}", e),
        }
        vm_engine.call_stack.clear();
    } else {
        eprintln!("[Game] execBootPhase2() not found in global functions, skipping");
    }



    // ── Phase 3b: Load MenuClassDefine (widget class registration) ─

    eprintln!("[Game] Phase 3b: Loading MenuClassDefine.adc...");

    match vm_engine.load_module("products/gt5m/script/MenuClassDefine.adc", assets_path) {

        Ok(idx) => {

            eprintln!("[Game] MenuClassDefine loaded (frame idx={})", idx);

        }

        Err(e) => {

            eprintln!("[Game] Error loading MenuClassDefine: {}", e);

            // Not fatal - UI will just have fewer classes registered

        }

    }



    // ── Phase 3c: Load config module ──────────────────────────────

    eprintln!("[Game] Phase 3c: Loading config/gt5m.adc...");

    match vm_engine.load_module("projects/gt5m/config/gt5m.adc", assets_path) {

        Ok(idx) => {

            eprintln!("[Game] config/gt5m.adc loaded (frame idx={})", idx);

        }

        Err(e) => {

            eprintln!("[Game] Error loading config: {}", e);

        }

    }



    // ── Phase 3d: Load init_sound.adc ────────────────────────────

    eprintln!("[Game] Phase 3d: Loading init_sound.adc...");

    match vm_engine.load_module("scripts/init_sound.adc", assets_path) {

        Ok(idx) => {

            eprintln!("[Game] init_sound.adc loaded (frame idx={})", idx);

        }

        Err(e) => {

            eprintln!("[Game] Error loading init_sound: {}", e);

        }

    }



    // ── Phase 4: Native Main Loop ──────────────────────────────
    eprintln!("[Game] Starting NATIVE main loop (VM bypass)");
    // Start directly in race mode
    engine::menu::set_next_sequence(4); // RACE = 4
    let mut loop_state = MainLoopState::new();

    if let Some(project) = get_appopt_boot_project(&mut vm_engine) {

        loop_state.game_sequence.current_project = project;

    }

    eprintln!("[Game] Initial project: {}", loop_state.game_sequence.current_project);



    let mut frame_count = 0u64;

    let start_time = std::time::Instant::now();



    loop {

        // ── Poll events ────────────────────────────────────────

        let event_opt = renderer.borrow_mut().poll_event();

        if let Some(event_str) = event_opt {

            if event_str == "quit" {

                eprintln!("[Game] Quit requested");

                return;

            }

        }



        // ── Clear screen ───────────────────────────────────────

        renderer.borrow_mut().clear();



        // ── Native Main Loop tick ──────────────────────────────

        match loop_state.tick(&mut vm_engine) {

            Ok(true) => {} // Continue game

            Ok(false) => {

                eprintln!("[Game] Main loop finished");

                break;

            }

            Err(e) => {

                eprintln!("[Game] Main loop error: {}", e);

                break;

            }

        }



        // ── UiManager: process input, update animations, render ─

        let key_state = renderer.borrow().get_key_state();

        engine::menu::update_global_key_state(key_state);

        let mut script_events: Vec<(String, String)> = vec![];

        engine::ui::with_ui(|ui| {

            let prev_finished = ui.is_finished();

            ui.update(key_state, 0.016);

            // Handle page navigation on Cross activation

            if !prev_finished && ui.is_finished() {

                if let Some(focused) = ui.focused_widget() {

                    handle_page_navigation(ui, &focused, &loop_state.game_sequence.current_project);

                }

                ui.reset_finished();

            }

            // Drain script events

            script_events = ui.drain_events();

        });

        // Process script callbacks (widget activate/focus → VM function)

        for (evt_type, widget_name) in script_events {

            let callback_name = format!("{}_{}_onActivate", loop_state.game_sequence.current_project, widget_name);

            if let Some(fv) = vm_engine.global_functions.get(&callback_name) {

                eprintln!("[Script] Calling {}", callback_name);

                if let Err(e) = vm_engine.call_function_value(Value::Function(fv.clone()), vec![]) {

                    eprintln!("[Script] Error in {}: {}", callback_name, e);

                }

            }

            let generic_name = format!("{}_onActivate", widget_name);

            if let Some(fv) = vm_engine.global_functions.get(&generic_name) {

                if let Err(e) = vm_engine.call_function_value(Value::Function(fv.clone()), vec![]) {

                    eprintln!("[Script] Error in {}: {}", generic_name, e);

                }

            }

        }



        // ── HUD overlay ────────────────────────────────────────

        let elapsed = start_time.elapsed().as_secs_f64().max(0.001);

        let fps = frame_count as f64 / elapsed;

        let insn_count = vm_engine.get_insn_count();



        let seq_labels = ["MENU", "RACE", "REPLAY", "LICENSE", "ONLINE"];

        let seq_idx = loop_state.game_sequence.current_sequence as usize;

        let seq_name = seq_labels.get(seq_idx).unwrap_or(&"???");



        renderer.borrow_mut().draw_text(10, 10, "GT PSP PC Port [Native Loop]", 100, 200, 100, 0.7);

        renderer.borrow_mut().draw_text(10, 30, &format!("FPS: {:.0}  Seq: {}  Proj: {}  Insn: {}K  Phase: {:?}",

            fps, seq_name, loop_state.game_sequence.current_project, insn_count / 1000, loop_state.phase), 140, 160, 140, 0.5);

        renderer.borrow_mut().draw_text(10, 530, "[ESC] Quit  [Arrows] Navigate  [S] Select  [D] Back", 100, 100, 120, 0.4);



        // ── Present frame ──────────────────────────────────────

        renderer.borrow_mut().end_scene();



        frame_count += 1;



        if frame_count % 300 == 0 {

            eprintln!("[Game] Frame {} | {:.*} fps | insn: {}K | seq: {} | proj: {} | phase: {:?}",

                frame_count, 1, fps, insn_count / 1000, seq_name,

                loop_state.game_sequence.current_project, loop_state.phase);

        }



        std::thread::sleep(std::time::Duration::from_millis(16));

    }



    eprintln!("[Game] Exited after {} frames ({} instructions)", frame_count, vm_engine.get_insn_count());

    std::process::exit(0);

}



/// Handle page navigation when a focused widget is activated.

fn handle_page_navigation(ui: &mut engine::ui::UiManager, focused: &str, current_project: &str) {

    eprintln!("[Nav] Focused: \"{}\" on page \"{}\"", focused, ui.active_page);



    // Arcade mode navigation based on focused widget

    let next = match ui.active_page.as_str() {

        "BranchRoot" => match focused {

            "ArcadeRace" | "TimeAttack" | "DriftAttack" | "2PBattle" | "Status" | "ReplayTheater" | "TradeAndShare" => "GTTopRoot",

            _ => "GTTopRoot",

        },

        "GTTopRoot" => match focused {

            "CarTown" => "DealerRoot",

            "Course" => "CourseRoot",

            "LicenseCenter" => "LicenseRoot",

            "Status" => "StatusRoot",

            "Options" => "",

            _ => "TopRoot",

        },

        "TopRoot" => match focused {

            "DrivingModeSelect" => "DrivingModeRoot",

            "CarSelect" => "CarRoot",

            "CourseSelect" => "CourseRoot",

            "Start" => {

                eprintln!("[Nav] Start race! Setting sequence to RACE");

                engine::menu::set_next_sequence(4); // RACE = 4

                // Reset the menu finished state so the sync phase can detect the change

                return;

            }

            "Exit" => { ui.go_to_page("BranchRoot"); return; }

            _ => return,

        },

        "DrivingModeRoot" => match focused {

            "SingleRace" | "TimeAttack" | "DriftAttack" => { ui.go_to_page("CarRoot"); return; }

            "AdHocBattle" => { ui.go_to_page("OnlineRoot"); return; }

            _ => { ui.go_to_page("TopRoot"); return; }

        },

        "CarRoot" | "CourseRoot" | "DealerRoot" | "OnlineRoot" | "SelectRoomRoot" |

        "LicenseRoot" | "LicenseCategoryRoot" | "StatusRoot" | "LogsRoot" | "ReplayRoot" => "",

        _ => "TopRoot",

    };



    if !next.is_empty() && next != ui.active_page {

        eprintln!("[Nav] Transition: {} -> {}", ui.active_page, next);

        ui.go_to_page(next);

    }

}



/// Run a global VM function with fallback for long execution.
fn run_vm_function(vm: &mut Engine, name: &str) {
    if let Some(fv) = vm.global_functions.get(name)
        .or_else(|| vm.global_functions.get(&format!("main::{}", name)))
    {
        let code_frame = {
            let mut found = None;
            for frame in &vm.modules.frames {
                if (fv.code_frame as usize) < frame.child_frames.len() {
                    found = Some(frame.child_frames[fv.code_frame as usize].clone());
                    break;
                }
            }
            found
        };
        if let Some(cf) = code_frame {
            let stack = cf.stack_size;
            let locals = cf.local_var_count;
            let cf_idx = vm.modules.add_frame(cf);
            let mut frame = vm::frame::Frame::new(
                cf_idx, stack, locals, fv.static_base
            );
            vm.call_stack.push(frame);
            let mut iters = 0u64;
            loop {
                if vm.call_stack.is_empty() { break; }
                match vm.execute_frame(10000) {
                    Ok(Some(_)) => break,
                    Ok(None) => {}
                    Err(e) => { eprintln!("[VM] {} error: {}", name, e); break; }
                }
                iters += 1;
                if iters > 100000 { eprintln!("[VM] {}: hit iter limit", name); break; }
                std::thread::sleep(std::time::Duration::from_millis(1));
            }
        } else {
            eprintln!("[VM] {}: child frame not found", name);
        }
    } else {
        eprintln!("[VM] {}: not found in global_functions", name);
    }
}

/// Read the initial project from GameSequence shared state.

fn get_appopt_boot_project(vm: &mut Engine) -> Option<String> {

    if let Some(nf) = vm.natives.get("main,GameSequence,getCurrentProject") {

        let val = nf(&[]);

        let s = val.to_string();

        if !s.is_empty() && s != "nil" {

            return Some(s.trim_matches('"').to_string());

        }

    }

    Some("arcade".to_string())

}



#[cfg(not(windows))]

fn run_game_loop(_file_path: &str) {

    eprintln!("Game loop not supported on this platform.");

}



fn test_specdb() {

    let path = format!("{}\\specdb\\GT_PSP_JP2817", engine::assets_root());

    println!("Testing SpecDB loading from path: {}", path);

    let sd = match engine::specdb::SpecDB::load_directory(&path) {

        Ok(sd) => sd,

        Err(e) => { eprintln!("SpecDB error: {}", e); return; }

    };

    println!("SpecDB loaded {} tables:", sd.tables.len());

    for name in sd.tables.keys().take(20) {

        println!("  {}", name);

    }

    if sd.tables.len() > 20 {

        println!("  ... and {} more", sd.tables.len() - 20);

    }

    std::process::exit(0);

}



fn list_native_functions() {

    let mut registry = vm::native::NativeRegistry::new();

    let specdb = Rc::new(RefCell::new(engine::specdb::SpecDB::new()));

    engine::gtengine::register_gtengine(&mut registry, specdb.clone());

    engine::pdiext::register_pdiext(&mut registry);

    engine::pdiapp::register_pdiapp(&mut registry);

    engine::menu::register_menu(&mut registry);

    
engine::audio::register_audio(&mut registry);

    eprintln!("Listing native functions...");

    registry.list();

    std::process::exit(0);

}



fn load_and_disassemble(path: &str) {

    let resolved = resolve_path(path);

    match std::fs::read(&resolved) {

        Ok(data) => {

            match Loader::load(&data) {

                Ok(frame) => {

                    println!("=== Disassembly of {} ===", resolved);

                    println!("Version: {}.{}", frame.adhoc_version / 16, frame.adhoc_version % 16);

                    println!("Stack size: {}", frame.stack_size);

                    println!("Local vars: {}", frame.local_var_count);

                    println!("Static vars: {}", frame.static_var_count);

                    println!("Instructions: {}", frame.instructions.len());

                    for (i, insn) in frame.instructions.iter().enumerate().take(30) {

                        println!("  {:04}: {:?}", i, insn);

                    }

                    if frame.instructions.len() > 30 {

                        println!("  ... and {} more", frame.instructions.len() - 30);

                    }

                }

                Err(e) => eprintln!("Load error: {}", e)

            }

        }

        Err(e) => eprintln!("Read error: {}", e)

    }

}



fn load_and_dump(path: &str) {

    let resolved = resolve_path(path);

    match std::fs::read(&resolved) {

        Ok(data) => {

            match Loader::load(&data) {

                Ok(frame) => {

                    println!("=== Dump of {} ===", resolved);

                    println!("Header: version={}.{}, stack={}, locals={}, statics={}", 

                        frame.adhoc_version / 16, frame.adhoc_version % 16,

                        frame.stack_size, frame.local_var_count, frame.static_var_count);

                    println!("Instructions: {}", frame.instructions.len());

                    println!("Child frames: {}", frame.child_frames.len());

                }

                Err(e) => eprintln!("Load error: {}", e)

            }

        }

        Err(e) => eprintln!("Read error: {}", e)

    }

}



fn execute_file_with_flags(path: &str, trace: bool, log_native: bool) -> Result<Value, String> {

    let resolved = resolve_path(path);

    let data = fs::read(&resolved).map_err(|e| format!("Failed to read {}: {}", resolved, e))?;

    let root_frame = Loader::load(&data)?;

    let mut registry = ModuleRegistry::new();

    let frame_idx = registry.add_frame(root_frame);



    let specdb = Rc::new(RefCell::new(engine::specdb::SpecDB::new()));

    let mut natives = NativeRegistry::new();

    
engine::audio::register_audio(&mut natives);

    engine::pdistd::register_pdistd(&mut natives);

    engine::pdiext::register_pdiext(&mut natives);

    engine::pdiapp::register_pdiapp(&mut natives);

    engine::menu::register_menu(&mut natives);

    engine::gtengine::register_gtengine(&mut natives, specdb);

    

    let mut engine = Engine::new(registry, natives);

    engine.trace = trace;

    engine.log_native = log_native;

    engine.execute(frame_idx, vec![])

}



fn test_all_adc_files() {

    let base = "assets/scripts";

    let mut passed = 0u32;

    let mut failed = 0u32;

    let mut warnings = 0u32;

    

    fn recurse(dir: &std::path::Path, base: &str, passed: &mut u32, failed: &mut u32, warnings: &mut u32) {

        if let Ok(entries) = std::fs::read_dir(dir) {

            for entry in entries.flatten() {

                let path = entry.path();

                if path.is_dir() {

                    recurse(&path, base, passed, failed, warnings);

                } else if path.extension().map(|e| e == "adc").unwrap_or(false) {

                    let name = path.file_name().unwrap().to_string_lossy().to_string();

                    let data = match std::fs::read(&path) {

                        Ok(d) => d,

                        Err(e) => { eprintln!("FAIL {}: read error: {}", name, e); *failed += 1; continue; }

                    };

                    match crate::vm::loader::Loader::load(&data) {

                        Ok(frame) => {

                            let has_warnings = frame.instructions.is_empty();

                            if has_warnings {

                                eprintln!("WARN {}: {} insns, {} child frames", name, frame.instructions.len(), frame.child_frames.len());

                                *warnings += 1;

                            } else {

                                *passed += 1;

                            }

                        }

                        Err(e) => {

                            eprintln!("FAIL {}: {}", name, e);

                            *failed += 1;

                        }

                    }

                }

            }

        }

    }

    

    eprintln!("Testing all .adc files in {}...", base);

    recurse(std::path::Path::new(base), base, &mut passed, &mut failed, &mut warnings);

    eprintln!("Results: {} passed, {} warnings, {} failed", passed, warnings, failed);

    std::process::exit(if failed > 0 { 1 } else { 0 });

}



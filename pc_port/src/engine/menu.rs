use std::rc::Rc;
use std::sync::atomic::{AtomicU32, Ordering};
use crate::vm::value::*;

/// Global key state updated each frame by the render loop.
/// Bits: 0=SELECT 1=START 2=UP 3=RIGHT 4=DOWN 5=LEFT
/// 6=L 7=R 8=TRIANGLE 9=CIRCLE 10=CROSS 11=SQUARE
pub static KEY_STATE: AtomicU32 = AtomicU32::new(0);
pub static PREV_KEY_STATE: AtomicU32 = AtomicU32::new(0);

use std::sync::OnceLock;
use std::sync::Mutex;
struct SeqState {
    current_sequence: i32, current_project: String, finished: bool,
    mode: i32, current_mode: i32,
}
static SEQ: OnceLock<Mutex<SeqState>> = OnceLock::new();
fn seq_state() -> &'static Mutex<SeqState> {
    SEQ.get_or_init(|| Mutex::new(SeqState {
        current_sequence: 1, // MENU
        current_project: "arcade".to_string(),
        finished: false, mode: 0, current_mode: 1,
    }))
}

/// Update the global key state (called from main.rs render loop).
pub fn update_global_key_state(state: u32) {
    PREV_KEY_STATE.store(KEY_STATE.load(Ordering::Relaxed), Ordering::Relaxed);
    KEY_STATE.store(state, Ordering::Relaxed);
}

/// Set the next game sequence (called from navigation in main.rs).
pub fn set_next_sequence(seq: i32) {
    let mut s = seq_state().lock().unwrap();
    s.current_sequence = seq;
    eprintln!("[GameSeq] setNextSequence: {}", seq);
}

/// Get the shared game sequence state.
pub fn get_current_sequence() -> i32 {
    seq_state().lock().unwrap().current_sequence
}

/// Register all menu/UI native API stubs with the VM.
pub fn register_menu(registry: &mut crate::vm::native::NativeRegistry) {

    // ─── Fallback for ALL unmatched menu paths ───
    // Any "main,menu,*,*" call that isn't explicitly registered
    // gets routed here with its full path as arg[0]
    registry.register_fallback("main,menu,", Rc::new(|args: &[Value]| {
        Value::Void
    }));
    registry.register_fallback("main,pdistd,", Rc::new(|args: &[Value]| {
        Value::Void
    }));
    registry.register_fallback("main,pdiext,", Rc::new(|args: &[Value]| {
        Value::Void
    }));
    registry.register_fallback("main,pdiapp,", Rc::new(|args: &[Value]| {
        Value::Void
    }));
    registry.register_fallback("main,gtengine,", Rc::new(|args: &[Value]| {
        Value::Void
    }));

    // ─── MMenuGameObjectManager — root widget manager ──────────
    registry.register("main,menu,MMenuGameObjectManager", Rc::new(|args| {
        Value::Object(Rc::new(ObjectInstance {
            class_path: "MMenuGameObjectManager".to_string(),
            fields: vec![],
        }))
    }));
    registry.register("main,menu,MMenuGameObjectManager,create", Rc::new(|args| {
        Value::Object(Rc::new(ObjectInstance {
            class_path: "MMenuGameObjectManager".to_string(),
            fields: vec![],
        }))
    }));
    registry.register("main,menu,MMenuGameObjectManager,initialize", Rc::new(|_args| Value::Void));
    registry.register("main,menu,MMenuGameObjectManager,setProjectRoot", Rc::new(|args| {
        let root = args.first().map(|a| a.to_string()).unwrap_or_default();
        eprintln!("[MGOM] setProjectRoot: \"{}\"", root);
        Value::Void
    }));
    registry.register("main,menu,MMenuGameObjectManager,addChild", Rc::new(|args| Value::Void));
    registry.register("main,menu,MMenuGameObjectManager,removeChild", Rc::new(|args| Value::Void));
    registry.register("main,menu,MMenuGameObjectManager,setVisible", Rc::new(|args| Value::Void));
    registry.register("main,menu,MMenuGameObjectManager,setAlpha", Rc::new(|args| Value::Void));
    registry.register("main,menu,MMenuGameObjectManager,set_value", Rc::new(|args| Value::Void));
    registry.register("main,menu,MMenuGameObjectManager,get_value", Rc::new(|args| Value::Nil));
    registry.register("main,menu,MMenuGameObjectManager,addWatcher", Rc::new(|args| Value::Void));
    registry.register("main,menu,MMenuGameObjectManager,removeWatcher", Rc::new(|args| Value::Void));
    registry.register("main,menu,MMenuGameObjectManager,getChildCount", Rc::new(|args| Value::Int(0)));
    registry.register("main,menu,MMenuGameObjectManager,getChildAt", Rc::new(|args| Value::Nil));

    // ─── MRootTransition — screen transition manager ──────────
    registry.register("main,menu,MRootTransition", Rc::new(|args| {
        Value::Object(Rc::new(ObjectInstance {
            class_path: "MRootTransition".to_string(),
            fields: vec![],
        }))
    }));
    registry.register("main,menu,MRootTransition,start", Rc::new(|args| Value::Void));
    registry.register("main,menu,MRootTransition,setProject", Rc::new(|args| Value::Void));

    // ─── MFunctionEvent — function event wrapper ──────────────
    registry.register("main,menu,MFunctionEvent", Rc::new(|args| {
        Value::Object(Rc::new(ObjectInstance {
            class_path: "MFunctionEvent".to_string(),
            fields: vec![],
        }))
    }));
    registry.register("main,menu,MFunctionEvent,setFunction", Rc::new(|args| Value::Void));
    registry.register("main,menu,MFunctionEvent,call", Rc::new(|args| Value::Nil));

    // ─── MScriptEvent — script event wrapper ──────────────────
    registry.register("main,menu,MScriptEvent", Rc::new(|args| {
        Value::Object(Rc::new(ObjectInstance {
            class_path: "MScriptEvent".to_string(),
            fields: vec![],
        }))
    }));
    registry.register("main,menu,MScriptEvent,setScript", Rc::new(|args| Value::Void));
    registry.register("main,menu,MScriptEvent,call", Rc::new(|args| Value::Nil));

    // ─── MActivateEvent — activation event ────────────────────
    registry.register("main,menu,MActivateEvent", Rc::new(|args| {
        Value::Object(Rc::new(ObjectInstance {
            class_path: "MActivateEvent".to_string(),
            fields: vec![],
        }))
    }));
    registry.register("main,menu,MActivateEvent,call", Rc::new(|args| Value::Nil));

    // ─── MFocusEnterEvent — focus enter event ─────────────────
    registry.register("main,menu,MFocusEnterEvent", Rc::new(|args| {
        Value::Object(Rc::new(ObjectInstance {
            class_path: "MFocusEnterEvent".to_string(),
            fields: vec![],
        }))
    }));
    registry.register("main,menu,MFocusEnterEvent,call", Rc::new(|args| Value::Nil));

    // ─── MMoveActor — move animation actor ────────────────────
    registry.register("main,menu,MMoveActor", Rc::new(|args| {
        Value::Object(Rc::new(ObjectInstance {
            class_path: "MMoveActor".to_string(),
            fields: vec![],
        }))
    }));
    registry.register("main,menu,MMoveActor,setTarget", Rc::new(|args| Value::Void));
    registry.register("main,menu,MMoveActor,setDuration", Rc::new(|args| Value::Void));
    registry.register("main,menu,MMoveActor,setDelay", Rc::new(|args| Value::Void));
    registry.register("main,menu,MMoveActor,play", Rc::new(|args| Value::Void));
    registry.register("main,menu,MMoveActor,stop", Rc::new(|args| Value::Void));
    registry.register("main,menu,MMoveActor,reset", Rc::new(|args| Value::Void));
    registry.register("main,menu,MMoveActor,isPlaying", Rc::new(|args| Value::Bool(false)));

    // ─── MFadeActor — fade animation actor ────────────────────
    registry.register("main,menu,MFadeActor", Rc::new(|args| {
        Value::Object(Rc::new(ObjectInstance {
            class_path: "MFadeActor".to_string(),
            fields: vec![],
        }))
    }));
    registry.register("main,menu,MFadeActor,setTargetAlpha", Rc::new(|args| Value::Void));
    registry.register("main,menu,MFadeActor,setDuration", Rc::new(|args| Value::Void));
    registry.register("main,menu,MFadeActor,play", Rc::new(|args| Value::Void));
    registry.register("main,menu,MFadeActor,stop", Rc::new(|args| Value::Void));

    // ─── MInterpolator — animation interpolation ──────────────
    registry.register("main,menu,MInterpolator", Rc::new(|args| {
        Value::Object(Rc::new(ObjectInstance {
            class_path: "MInterpolator".to_string(),
            fields: vec![],
        }))
    }));
    registry.register("main,menu,MInterpolator,start", Rc::new(|args| Value::Void));
    registry.register("main,menu,MInterpolator,stop", Rc::new(|args| Value::Void));
    registry.register("main,menu,MInterpolator,reset", Rc::new(|args| Value::Void));
    registry.register("main,menu,MInterpolator,isFinished", Rc::new(|args| Value::Bool(true)));

    // ─── MScriptWatcher — async timer/ticker ──────────────────
    registry.register("main,menu,MScriptWatcher", Rc::new(|args| {
        Value::Object(Rc::new(ObjectInstance {
            class_path: "MScriptWatcher".to_string(),
            fields: vec![],
        }))
    }));
    registry.register("main,menu,MScriptWatcher,create", Rc::new(|args| Value::Void));
    registry.register("main,menu,MScriptWatcher,start", Rc::new(|args| Value::Void));
    registry.register("main,menu,MScriptWatcher,stop", Rc::new(|args| Value::Void));
    registry.register("main,menu,MScriptWatcher,isExpired", Rc::new(|args| Value::Bool(false)));
    registry.register("main,menu,MScriptWatcher,getElapsed", Rc::new(|args| Value::Int(0)));

    // ─── MAdjustment — scrollbar/list adjustment ──────────────
    registry.register("main,menu,MAdjustment", Rc::new(|args| {
        Value::Object(Rc::new(ObjectInstance {
            class_path: "MAdjustment".to_string(),
            fields: vec![],
        }))
    }));
    registry.register("main,menu,MAdjustment,set_value", Rc::new(|args| Value::Void));
    registry.register("main,menu,MAdjustment,get_value", Rc::new(|args| Value::Int(0)));
    registry.register("main,menu,MAdjustment,setRange", Rc::new(|args| Value::Void));

    // ─── MColorObject — color representation ──────────────────
    registry.register("main,menu,MColorObject", Rc::new(|args| {
        Value::Object(Rc::new(ObjectInstance {
            class_path: "MColorObject".to_string(),
            fields: vec![],
        }))
    }));
    registry.register("main,menu,MColorObject,setColor", Rc::new(|args| Value::Void));
    registry.register("main,menu,MColorObject,getColor", Rc::new(|args| Value::Int(0)));

    // ─── MOptionMenu — dropdown/option menu ───────────────────
    registry.register("main,menu,MOptionMenu", Rc::new(|args| {
        Value::Object(Rc::new(ObjectInstance {
            class_path: "MOptionMenu".to_string(),
            fields: vec![],
        }))
    }));
    registry.register("main,menu,MOptionMenu,addItem", Rc::new(|args| Value::Void));
    registry.register("main,menu,MOptionMenu,setActiveItem", Rc::new(|args| Value::Void));
    registry.register("main,menu,MOptionMenu,getActiveItem", Rc::new(|args| Value::Int(0)));

    // ─── EventLoop — core event loop ──────────────────────────
    registry.register("main,menu,EventLoop", Rc::new(|args| {
        Value::Object(Rc::new(ObjectInstance {
            class_path: "EventLoop".to_string(),
            fields: vec![],
        }))
    }));
    registry.register("main,menu,EventLoop,run", Rc::new(|args| Value::Void));
    registry.register("main,menu,EventLoop,stop", Rc::new(|args| Value::Void));
    registry.register("main,menu,EventLoop,isRunning", Rc::new(|args| Value::Bool(false)));

    // ─── MUpdateContext::Sync — frame synchronization ─────────
    registry.register("main,menu,MUpdateContext,Sync", Rc::new(|args| Value::Void));

    // ─── ScreenWidth / ScreenHeight constants ─────────────────
    registry.register("main,menu,ScreenWidth", Rc::new(|args| Value::Int(480)));
    registry.register("main,menu,ScreenHeight", Rc::new(|args| Value::Int(272)));

    // ─── MActor base class ────────────────────────────────────
    registry.register("main,menu,MActor", Rc::new(|args| {
        Value::Object(Rc::new(ObjectInstance {
            class_path: "MActor".to_string(),
            fields: vec![],
        }))
    }));
    registry.register("main,menu,MActor,setPosition", Rc::new(|args| Value::Void));
    registry.register("main,menu,MActor,setSize", Rc::new(|args| Value::Void));
    registry.register("main,menu,MActor,setVisible", Rc::new(|args| Value::Void));
    registry.register("main,menu,MActor,getPosition", Rc::new(|args| Value::Int(0)));
    registry.register("main,menu,MActor,getSize", Rc::new(|args| Value::Int(0)));

    // ─── MCrossfadeGroup ──────────────────────────────────────
    registry.register("main,menu,MCrossfadeGroup", Rc::new(|args| {
        Value::Object(Rc::new(ObjectInstance {
            class_path: "MCrossfadeGroup".to_string(),
            fields: vec![],
        }))
    }));
    registry.register("main,menu,MCrossfadeGroup,start", Rc::new(|args| Value::Void));
    registry.register("main,menu,MCrossfadeGroup,stop", Rc::new(|args| Value::Void));

    // ─── MManager ─────────────────────────────────────────────
    registry.register("main,menu,MManager", Rc::new(|args| {
        Value::Object(Rc::new(ObjectInstance {
            class_path: "MManager".to_string(),
            fields: vec![],
        }))
    }));
    registry.register("main,menu,MManager,add", Rc::new(|args| Value::Void));
    registry.register("main,menu,MManager,remove", Rc::new(|args| Value::Void));
    registry.register("main,menu,MManager,clear", Rc::new(|args| Value::Void));
    registry.register("main,menu,MManager,getCount", Rc::new(|args| Value::Int(0)));

    // ─── SuperPortButtonBit — controller button input ─────────
    foreach_button_input(registry);

    // ─── SuperPortAnalogChannel — analog stick input ──────────
    foreach_analog_input(registry);

    // ─── ActorUtil helpers ────────────────────────────────────
    registry.register("main,menu,ActorUtil", Rc::new(|args| {
        Value::Object(Rc::new(ObjectInstance {
            class_path: "ActorUtil".to_string(),
            fields: vec![],
        }))
    }));
    registry.register("main,menu,ActorUtil,add", Rc::new(|args| Value::Void));
    registry.register("main,menu,ActorUtil,remove", Rc::new(|args| Value::Void));
    registry.register("main,menu,ActorUtil,clear", Rc::new(|args| Value::Void));
    registry.register("main,menu,ActorUtil,find", Rc::new(|args| Value::Nil));

    // ─── DialogUtil — confirm/error dialogs ───────────────────
    registry.register("main,pdistd,DialogUtil,openConfirmDialog", Rc::new(|args| Value::Int(1)));
    registry.register("main,pdistd,DialogUtil,openErrorDialog", Rc::new(|args| Value::Int(0)));
    registry.register("main,pdistd,DialogUtil,showMessage", Rc::new(|args| Value::Void));

    // ─── Key bindings / super port ────────────────────────────
    registry.register("main,menu,SuperPortGetButtonBit", Rc::new(|args| Value::Int(0)));
    registry.register("main,menu,SuperPortGetAnalogChannel", Rc::new(|args| Value::Int(0)));

    // ─── GameSequence — state machine with shared state ──────
    // Shared state so both native stubs and native main loop stay in sync
    registry.register("main,GameSequence,getCurrentSequence", Rc::new(|_| {
        Value::Int(seq_state().lock().unwrap().current_sequence)
    }));
    registry.register("main,GameSequence,setMode", Rc::new(|args| {
        let m = args.get(0).and_then(|v| v.as_i32()).unwrap_or(1);
        seq_state().lock().unwrap().current_mode = m;
        Value::Void
    }));
    registry.register("main,GameSequence,getCurrentProject", Rc::new(|_| {
        let proj = seq_state().lock().unwrap().current_project.clone();
        Value::String(Rc::new(proj))
    }));
    registry.register("main,GameSequence,setNextProject", Rc::new(|args| {
        let proj = args.get(0).map(|v| v.to_string()).unwrap_or_default();
        seq_state().lock().unwrap().current_project = proj;
        Value::Void
    }));
    registry.register("main,GameSequence,allocateReplayBuffer", Rc::new(|_| Value::Void));
    registry.register("main,GameSequence,freeUnusedReplayBuffer", Rc::new(|_| Value::Void));
    registry.register("main,GameSequence,endUsedReplayBuffer", Rc::new(|_| Value::Void));
    registry.register("main,GameSequence,executeRace", Rc::new(|_| {
        eprintln!("[GameSequence] executeRace — starting race project");
        // Start the race project — this loads race.mproject + race.adc
        crate::engine::ui::with_ui(|ui| ui.start_project("race"));
        Value::Void
    }));
    registry.register("main,GameSequence,setType", Rc::new(|_| Value::Void));
    registry.register("main,GameSequence,getRaceCount", Rc::new(|_| Value::Int(1)));
    registry.register("main,GameSequence,finish", Rc::new(|_| {
        seq_state().lock().unwrap().finished = true;
        Value::Void
    }));

    // GameSequence context — returns a map with mutable state keys
    registry.register("main,GameSequence,context", Rc::new(|_| {
        let state = seq_state().lock().unwrap();
        let data = vec![
            (Value::String(Rc::new("finished".to_string())), Value::Bool(state.finished)),
            (Value::String(Rc::new("current_sequence".to_string())), Value::Int(state.current_sequence)),
            (Value::String(Rc::new("current_project".to_string())), Value::String(Rc::new(state.current_project.clone()))),
        ];
        Value::Map(Rc::new(data))
    }));

    // ─── GameSequence enums (match GameSequence.ad source) ──
    // GameSequence.ad: UNDEFINED=0, MENU=1, SINGLE_RACE=2, ONLINE_BATTLE=3, RACE=4
    registry.register("main,GameSequence,UNDEFINED", Rc::new(|_| Value::Int(0)));
    registry.register("main,GameSequence,MENU", Rc::new(|_| Value::Int(1)));
    registry.register("main,GameSequence,SINGLE_RACE", Rc::new(|_| Value::Int(2)));
    registry.register("main,GameSequence,ONLINE_BATTLE", Rc::new(|_| Value::Int(3)));
    registry.register("main,GameSequence,RACE", Rc::new(|_| Value::Int(4)));
    registry.register("main,GameSequence,REPLAY_THEATER", Rc::new(|_| Value::Int(5)));
    registry.register("main,GameSequence,LEAVE_DEMO", Rc::new(|_| Value::Int(6)));

    // ─── MGOM (MMenuGameObjectManager) — project loading ────
    registry.register("main,MGOM", Rc::new(|_| Value::Object(Rc::new(ObjectInstance {
        class_path: "MMenuGameObjectManager".to_string(), fields: vec![],
    }))));
    registry.register("main,MGOM,start", Rc::new(|args: &[Value]| {
        let project = args.first().map(|a| a.to_string()).unwrap_or_default();
        eprintln!("[MGOM] start project: \"{}\"", project);
        crate::engine::ui::with_ui(|ui| ui.start_project(&project));
        // Also try to load the project's .adc script so its classes/functions register
        let search_paths = [
            format!("assets/projects/gt5m/{}/{}.adc", project, project),
            format!("assets/projects/gt5m/{}.adc", project),
        ];
        for adc_path in &search_paths {
            if std::path::Path::new(adc_path).exists() {
                match crate::vm::loader::load_adc_file(adc_path) {
                    Ok(_) => eprintln!("[MGOM] Loaded project script: {}", adc_path),
                    Err(e) => eprintln!("[MGOM] Error loading {}: {}", adc_path, e),
                }
                break;
            }
        }
        Value::Void
    }));
    registry.register("main,MGOM,sync", Rc::new(|_| {
        let finished = crate::engine::ui::with_ui(|ui| ui.is_finished());
        Value::Bool(finished)
    }));
    // Expose project state accessor for the game loop
    registry.register("main,menu,_get_active_project", Rc::new(|_| {
        let name = crate::engine::ui::with_ui(|ui| ui.active_project.clone());
        match name {
            Some(n) => Value::String(Rc::new(n)),
            None => Value::Nil,
        }
    }));

    // ─── MOrganizer ─────────────────────────────────────────
    // Organizer system for game modes (arcade, campaign, etc.)
    registry.register("main,menu,MOrganizer", Rc::new(|_| Value::Object(Rc::new(ObjectInstance {
        class_path: "MOrganizer".to_string(), fields: vec![],
    }))));
    registry.register("main,menu,MOrganizer,initialize", Rc::new(|_| Value::Void));
    registry.register("main,menu,MOrganizer,finalize", Rc::new(|_| Value::Void));
    registry.register("main,menu,MOrganizer,start", Rc::new(|_| Value::Void));
    registry.register("main,menu,MOrganizer,stop", Rc::new(|_| Value::Void));
    registry.register("main,menu,MOrganizer,isRunning", Rc::new(|_| Value::Bool(false)));
    registry.register("main,menu,MOrganizer,getCurrent", Rc::new(|_| Value::Int(0)));
    registry.register("main,menu,MOrganizer,setCurrent", Rc::new(|_| Value::Void));

    // ─── MRaceOperator ─────────────────────────────────────
    // Race/arcade mode operator
    registry.register("main,menu,MRaceOperator", Rc::new(|_| Value::Object(Rc::new(ObjectInstance {
        class_path: "MRaceOperator".to_string(), fields: vec![],
    }))));
    registry.register("main,menu,MRaceOperator,initialize", Rc::new(|_| Value::Void));
    registry.register("main,menu,MRaceOperator,finalize", Rc::new(|_| Value::Void));
    registry.register("main,menu,MRaceOperator,start", Rc::new(|_| Value::Void));
    registry.register("main,menu,MRaceOperator,stop", Rc::new(|_| Value::Void));
    registry.register("main,menu,MRaceOperator,isRunning", Rc::new(|_| Value::Bool(false)));

    // ─── MSound ─────────────────────────────────────────────
    // Sound system for menus and BGM
    registry.register("main,menu,MSound", Rc::new(|_| Value::Object(Rc::new(ObjectInstance {
        class_path: "MSound".to_string(), fields: vec![],
    }))));
    registry.register("main,menu,MSound,initialize", Rc::new(|_| Value::Void));
    registry.register("main,menu,MSound,finalize", Rc::new(|_| Value::Void));
    registry.register("main,menu,MSound,start", Rc::new(|_| Value::Void));
    registry.register("main,menu,MSound,stop", Rc::new(|_| Value::Void));
    registry.register("main,menu,MSound,playBGM", Rc::new(|_| Value::Void));
    registry.register("main,menu,MSound,stopBGM", Rc::new(|_| Value::Void));
    registry.register("main,menu,MSound,playSE", Rc::new(|_| Value::Void));

    // ─── DebugTool ──────────────────────────────────────────
    registry.register("main,DebugTool,printHeapStatus", Rc::new(|_| Value::Void));

    // ─── GamePlan ─────────────────────────────────────────--
    registry.register("main,GamePlan", Rc::new(|_| Value::Object(Rc::new(ObjectInstance {
        class_path: "GamePlan".to_string(), fields: vec![],
    }))));
    registry.register("main,GamePlan,begin", Rc::new(|_| Value::Void));
    registry.register("main,GamePlan,end", Rc::new(|_| Value::Void));
    registry.register("main,GamePlan,executeNext", Rc::new(|_| Value::Void));

    // ─── SoundUtil ─────────────────────────────────────────
    registry.register("main,SoundUtil,BGMPlayGroup", Rc::new(|_| Value::Void));
}

fn foreach_button_input(registry: &mut crate::vm::native::NativeRegistry) {
    let buttons = ["cross", "circle", "triangle", "square", "start", "select",
                   "L", "R", "up", "down", "left", "right", "home", "hold"];
    // Bit positions matching the renderer's key_state:
    // 0=SELECT 1=START 2=UP 3=RIGHT 4=DOWN 5=LEFT
    // 6=L 7=R 8=TRIANGLE 9=CIRCLE 10=CROSS 11=SQUARE
    let bit_positions: [(&str, u32); 14] = [
        ("cross", 10), ("circle", 9), ("triangle", 8), ("square", 11),
        ("start", 1), ("select", 0),
        ("L", 6), ("R", 7),
        ("up", 2), ("down", 4), ("left", 5), ("right", 3),
        ("home", 31), ("hold", 30),
    ];
    for port in &[0i32, 1] {
        for &(btn, bit) in &bit_positions {
            let path = format!("main,menu,SuperPortButtonBit,{},{}", port, btn);
            registry.register(&path, Rc::new(move |_| {
                // Edge-detected: only true on press (prev=0, cur=1)
                let cur = (KEY_STATE.load(Ordering::Relaxed) >> bit) & 1;
                let prev = (PREV_KEY_STATE.load(Ordering::Relaxed) >> bit) & 1;
                Value::Bool(prev == 0 && cur == 1)
            }));
        }
    }
}

fn foreach_analog_input(registry: &mut crate::vm::native::NativeRegistry) {
    let channels = ["X", "Y"];
    for port in &[0i32, 1] {
        for ch in &channels {
            let path = format!("main,menu,SuperPortAnalogChannel,{},{}", port, ch);
            registry.register(&path, Rc::new(|_| Value::Int(0)));
        }
    }
}

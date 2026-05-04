use std::rc::Rc;
use crate::vm::value::*;

/// Register pdiext (extended library) native APIs.
pub fn register_pdiext(registry: &mut crate::vm::native::NativeRegistry) {
    // MProductInformation
    registry.register("main,pdiext,MProductInformation", Rc::new(|_args: &[Value]| {
        Value::Object(Rc::new(ObjectInstance {
            class_path: "MProductInformation".to_string(),
            fields: vec![],
        }))
    }));

    registry.register("main,pdiext,MProductInformation,GetProductInformationString", Rc::new(|_args: &[Value]| {
        Value::String(Rc::new("Gran Turismo PSP".to_string()))
    }));

    registry.register("main,pdiext,MProductInformation,GetPDIVersion", Rc::new(|_args: &[Value]| {
        Value::String(Rc::new("JP2817".to_string()))
    }));

    registry.register("main,pdiext,MProductInformation,GetVersionBranch", Rc::new(|_args: &[Value]| {
        Value::String(Rc::new("release".to_string()))
    }));

    registry.register("main,pdiext,MProductInformation,GetProductVersion", Rc::new(|_args: &[Value]| {
        Value::Int(2) // version 2.00
    }));

    registry.register("main,pdiext,MProductInformation,GetVersionEnvironment", Rc::new(|_args: &[Value]| {
        Value::Int(0) // 0 = release, 1 = development
    }));

    // LoadLatinFont
    registry.register("main,pdiext,LoadLatinFont", Rc::new(|_args: &[Value]| {
        Value::Int(1)
    }));

    registry.register("main,pdiext,UnloadLatinFont", Rc::new(|_args: &[Value]| {
        eprintln!("[Font] UnloadLatinFont");
        Value::Void
    }));

    // BGM/sound registrations are in audio.rs (with actual ffmpeg-based implementations).
    // pdiext.rs keeps only the non-audio registrations.

    // ClearFontCache
    registry.register("main,pdiext,ClearFontCache", Rc::new(|_args: &[Value]| {
        eprintln!("[Font] ClearFontCache");
        Value::Void
    }));

    registry.register("main,pdiext,MSoundContext", Rc::new(|_args: &[Value]| {
        Value::Object(Rc::new(ObjectInstance {
            class_path: "MSoundContext".to_string(),
            fields: vec![],
        }))
    }));

    // CarSound
    registry.register("main,pdiext,CarSound,road_attribute_sound_parameter", Rc::new(|_args: &[Value]| {
        Value::Object(Rc::new(ObjectInstance {
            class_path: "MRaceSound".to_string(),
            fields: vec![],
        }))
    }));

    registry.register("main,pdiext,CarSound,getContext", Rc::new(|_args: &[Value]| {
        Value::Object(Rc::new(ObjectInstance {
            class_path: "CarSoundContext".to_string(),
            fields: vec![],
        }))
    }));

    // Save data - proper implementation
    use std::path::Path;

    registry.register("main,pdiext,MSaveDataUtilPSP", Rc::new(|_args: &[Value]| {
        Value::Object(Rc::new(ObjectInstance {
            class_path: "MSaveDataUtilPSP".to_string(),
            fields: vec![],
        }))
    }));

    registry.register("main,pdiext,MSaveDataUtilPSP,save", Rc::new(|args: &[Value]| {
        let slot = args.first().and_then(|v| v.as_i32()).unwrap_or(0);
        let path = format!("save_data_{}.bin", slot);
        eprintln!("[SaveData] save slot={} path=\"{}\"", slot, path);
        
        // Collect all arguments as the save data blob
        let mut data = Vec::new();
        data.extend_from_slice(b"GTPSP_SAVE_V1"); // 12-byte magic header
        for arg in args.iter().skip(1) {
            match arg {
                Value::Int(i) => data.extend_from_slice(&i.to_le_bytes()),
                Value::UInt(u) => data.extend_from_slice(&u.to_le_bytes()),
                Value::Long(l) => data.extend_from_slice(&l.to_le_bytes()),
                Value::Float(f) => data.extend_from_slice(&f.to_le_bytes()),
                Value::String(s) => {
                    let bytes = s.as_bytes();
                    data.extend_from_slice(&(bytes.len() as u32).to_le_bytes());
                    data.extend_from_slice(bytes);
                }
                Value::Bool(b) => data.push(if *b { 1 } else { 0 }),
                _ => {}
            }
        }
        
        match std::fs::write(&path, &data) {
            Ok(_) => { eprintln!("[SaveData] saved {} bytes to {}", data.len(), path); Value::Int(1) }
            Err(e) => { eprintln!("[SaveData] save error: {}", e); Value::Int(0) }
        }
    }));

    registry.register("main,pdiext,MSaveDataUtilPSP,load", Rc::new(|args: &[Value]| {
        let slot = args.first().and_then(|v| v.as_i32()).unwrap_or(0);
        let path = format!("save_data_{}.bin", slot);
        eprintln!("[SaveData] load slot={} path=\"{}\"", slot, path);
        
        if Path::new(&path).exists() {
            match std::fs::read(&path) {
                Ok(data) => {
                    if data.len() >= 12 && &data[0..12] == b"GTPSP_SAVE_V1" {
                        eprintln!("[SaveData] loaded {} bytes", data.len());
                        Value::Int(1) // success
                    } else {
                        eprintln!("[SaveData] invalid magic");
                        Value::Int(0)
                    }
                }
                Err(e) => { eprintln!("[SaveData] load error: {}", e); Value::Int(0) }
            }
        } else {
            eprintln!("[SaveData] slot {} not found", slot);
            Value::Int(0)
        }
    }));

    registry.register("main,pdiext,MSaveDataUtilPSP,exists", Rc::new(|args: &[Value]| {
        let slot = args.first().and_then(|v| v.as_i32()).unwrap_or(0);
        let path = format!("save_data_{}.bin", slot);
        Value::Bool(Path::new(&path).exists())
    }));

    // MSystemUtility
    registry.register("main,pdiext,MSystemUtility,print", Rc::new(|args: &[Value]| {
        let msg = args.first().map(|a| a.to_string()).unwrap_or_default();
        println!("[Adhoc] {}", msg);
        Value::Void
    }));

    registry.register("main,pdiext,MSystemUtility,sprintf", Rc::new(|args: &[Value]| {
        let fmt = args.first().map(|a| a.to_string()).unwrap_or_default();
        // Simple %{expr} interpolation: evaluate native calls within %{...}
        let mut result = String::new();
        let mut i = 0;
        let bytes = fmt.as_bytes();
        while i < bytes.len() {
            if i + 2 < bytes.len() && bytes[i] == b'%' && bytes[i+1] == b'{' {
                let mut end = i + 2;
                while end < bytes.len() && bytes[end] != b'}' { end += 1; }
                end += 1; // skip '}'
                // Try to evaluate by calling through the native registry
                // For now, insert placeholder since we can't call arbitrary natives from here
                result.push('?');
                i = end;
            } else {
                result.push(bytes[i] as char);
                i += 1;
            }
        }
        Value::String(Rc::new(result))
    }));

    registry.register("main,pdiext,MSystemUtility,sleep", Rc::new(|_args: &[Value]| {
        std::thread::sleep(std::time::Duration::from_millis(16));
        Value::Void
    }));

    // MMisc utilities
    registry.register("main,pdiext,MMisc", Rc::new(|_args: &[Value]| {
        Value::Object(Rc::new(ObjectInstance {
            class_path: "MMisc".to_string(),
            fields: vec![],
        }))
    }));

    registry.register("main,pdiext,MMisc,GetMoneyString", Rc::new(|args: &[Value]| {
        let amount = args.first().and_then(|a| a.as_i32()).unwrap_or(0);
        Value::String(Rc::new(format!("{} Cr.", amount)))
    }));

    registry.register("main,pdiext,MMisc,GetTimeString", Rc::new(|args: &[Value]| {
        let secs = args.first().and_then(|a| a.as_i32()).unwrap_or(0);
        let h = secs / 3600;
        let m = (secs % 3600) / 60;
        let s = secs % 60;
        Value::String(Rc::new(format!("{:02}:{:02}:{:02}", h, m, s)))
    }));

    registry.register("main,pdiext,MMisc,GetDistanceString", Rc::new(|args: &[Value]| {
        let meters = args.first().and_then(|a| a.as_i32()).unwrap_or(0);
        Value::String(Rc::new(format!("{}m", meters)))
    }));

    registry.register("main,pdiext,MMisc,GetSpeedString", Rc::new(|args: &[Value]| {
        let speed = args.first().and_then(|a| a.as_i32()).unwrap_or(0);
        Value::String(Rc::new(format!("{} km/h", speed)))
    }));

    // Unit conversion
    registry.register("main,pdiext,MUnit", Rc::new(|_args: &[Value]| {
        Value::Object(Rc::new(ObjectInstance {
            class_path: "MUnit".to_string(),
            fields: vec![],
        }))
    }));

    registry.register("main,pdiext,MUnit,ConvertSpeed", Rc::new(|args: &[Value]| {
        let value = args.first().and_then(|a| a.as_i32()).unwrap_or(0);
        let to_mph = args.get(1).and_then(|a| Some(a.truthy())).unwrap_or(false);
        if to_mph {
            Value::Int((value as f32 * 0.621371) as i32)
        } else {
            Value::Int(value)
        }
    }));

    registry.register("main,pdiext,MUnit,ConvertPower", Rc::new(|args: &[Value]| {
        let value = args.first().and_then(|a| a.as_i32()).unwrap_or(0);
        let to_kw = args.get(1).and_then(|a| Some(a.truthy())).unwrap_or(false);
        if to_kw {
            Value::Int((value as f32 * 0.735499) as i32)
        } else {
            Value::Int(value)
        }
    }));

    // Input handling - controller (wired to global KEY_STATE from menu.rs)
    use std::sync::atomic::Ordering;
    let bit_positions = [
        ("cross", 10), ("circle", 9), ("triangle", 8), ("square", 11),
        ("start", 1), ("select", 0),
        ("L", 6), ("R", 7),
        ("up", 2), ("down", 4), ("left", 5), ("right", 3),
    ];

    registry.register("main,pdiext,SuperPortButtonBit", Rc::new(|_args: &[Value]| {
        Value::Bool(false)
    }));

    for port in 0..2 {
        for &(btn, bit) in &bit_positions {
            let path = format!("main,pdiext,SuperPortButtonBit,{},{}", port, btn);
            registry.register(&path, Rc::new(move |_args: &[Value]| {
                let cur = (crate::engine::menu::KEY_STATE.load(Ordering::Relaxed) >> bit) & 1;
                let prev = (crate::engine::menu::PREV_KEY_STATE.load(Ordering::Relaxed) >> bit) & 1;
                Value::Bool(prev == 0 && cur == 1)
            }));
        }
    }

    registry.register("main,pdiext,SuperPortAnalogChannel", Rc::new(|_args: &[Value]| {
        let ks = crate::engine::menu::KEY_STATE.load(Ordering::Relaxed);
        let _x_analog = if (ks >> 3) & 1 == 1 { 255 } else if (ks >> 5) & 1 == 1 { 0 } else { 128 };
        let _y_analog = if (ks >> 4) & 1 == 1 { 255 } else if (ks >> 2) & 1 == 1 { 0 } else { 128 };
        // TODO: Return Object with x,y fields once ObjectInstance supports key-value fields
        Value::Int(0)
    }));
    // Per-channel variants
    for port in 0..2 {
        for chan in &["X", "Y"] {
            let c = *chan;
            let path = format!("main,pdiext,SuperPortAnalogChannel,{},{}", port, chan);
            registry.register(&path, Rc::new(move |_args: &[Value]| {
                let ks = crate::engine::menu::KEY_STATE.load(Ordering::Relaxed);
                match c {
                    "X" => {
                        let v = if (ks >> 3) & 1 == 1 { 255 } else if (ks >> 5) & 1 == 1 { 0 } else { 128 };
                        Value::Int(v)
                    }
                    _ => {
                        let v = if (ks >> 4) & 1 == 1 { 255 } else if (ks >> 2) & 1 == 1 { 0 } else { 128 };
                        Value::Int(v)
                    }
                }
            }));
        }
    }

    // GlobalStatus - game state persistence
    registry.register("main,pdiext,GlobalStatus", Rc::new(|_args: &[Value]| {
        Value::Object(Rc::new(ObjectInstance {
            class_path: "GlobalStatus".to_string(),
            fields: vec![],
        }))
    }));
    registry.register("main,pdiext,GlobalStatus,initialize", Rc::new(|_args: &[Value]| {
        eprintln!("[GlobalStatus] initialize");
        Value::Void
    }));

    // manager - project manager
    registry.register("main,manager", Rc::new(|_args: &[Value]| {
        Value::Object(Rc::new(ObjectInstance {
            class_path: "manager".to_string(),
            fields: vec![],
        }))
    }));
    registry.register("main,manager,loadProject", Rc::new(|args: &[Value]| {
        let path = args.first().map(|a| a.to_string()).unwrap_or_default();
        eprintln!("[Manager] loadProject: \"{}\"", path);
        Value::Object(Rc::new(ObjectInstance {
            class_path: "MProject".to_string(),
            fields: vec![],
        }))
    }));

    // MPDINetwork methods
    registry.register("main,pdistd,MPDINetwork,initialize", Rc::new(|args: &[Value]| {
        let name = args.first().map(|a| a.to_string()).unwrap_or_default();
        eprintln!("[MPDINetwork] initialize: \"{}\"", name);
        Value::Void
    }));
    registry.register("main,pdistd,MPDINetwork,startUpdate", Rc::new(|_args: &[Value]| {
        eprintln!("[MPDINetwork] startUpdate");
        Value::Void
    }));

    // ─── Structure types (STStructure, STInt, STString) ─────
    registry.register("main,pdiext,STStructure", Rc::new(|_args: &[Value]| {
        Value::Object(Rc::new(ObjectInstance {
            class_path: "STStructureInstance".to_string(),
            fields: vec![],
        }))
    }));
    registry.register("main,pdiext,STStructureInstance,define", Rc::new(|args: &[Value]| {
        let name = args.first().map(|a| a.to_string()).unwrap_or_default();
        eprintln!("[STStructure] define field: \"{}\"", name);
        Value::Void
    }));
    registry.register("main,pdiext,STStructureInstance,call", Rc::new(|_args: &[Value]| {
        // Creating an instance: st() → returns a data map
        fn kv(k: &str, v: Value) -> (Value, Value) {
            (Value::String(Rc::new(k.to_string())), v)
        }
        let data = vec![
            kv("finished", Value::Bool(false)),
            kv("current_sequence", Value::Int(1)),
            kv("prev_sequence", Value::Int(0)),
            kv("current_mode", Value::Int(1)),
            kv("prev_mode", Value::Int(0)),
            kv("current_project", Value::String(Rc::new("boot".to_string()))),
            kv("prev_project", Value::String(Rc::new("undefined".to_string()))),
            kv("arg", Value::String(Rc::new(String::new()))),
            kv("result", Value::String(Rc::new(String::new()))),
            kv("prev_page", Value::String(Rc::new("undefined".to_string()))),
            kv("prev_page_arg", Value::String(Rc::new(String::new()))),
        ];
        Value::Map(Rc::new(data))
    }));

    registry.register("main,pdiext,STInt", Rc::new(|_args: &[Value]| {
        Value::Int(0)
    }));
    registry.register("main,pdiext,STString", Rc::new(|args: &[Value]| {
        let _len = args.first().and_then(|a| a.as_i32()).unwrap_or(16);
        Value::String(Rc::new(String::new()))
    }));

    // ByteData - binary data buffer
    registry.register("main,pdiext,ByteData", Rc::new(|args: &[Value]| {
        let _alignment = args.first().and_then(|a| a.as_i32()).unwrap_or(16);
        let _size = args.get(1).and_then(|a| a.as_i32()).unwrap_or(0x400);
        Value::Object(Rc::new(ObjectInstance {
            class_path: "ByteDataInstance".to_string(),
            fields: vec![],
        }))
    }));
    registry.register("main,pdiext,ByteDataInstance,setByteData", Rc::new(|_args: &[Value]| {
        Value::Void
    }));
    registry.register("main,pdiext,ByteDataInstance,getByteData", Rc::new(|_args: &[Value]| {
        Value::Object(Rc::new(ObjectInstance {
            class_path: "ByteDataInstance".to_string(),
            fields: vec![],
        }))
    }));

    // MSystemCondition - game state
    registry.register("main,pdiext,MSystemCondition", Rc::new(|_args: &[Value]| {
        Value::Object(Rc::new(ObjectInstance {
            class_path: "MSystemCondition".to_string(),
            fields: vec![],
        }))
    }));

    registry.register("main,pdiext,MSystemCondition,IsExitGame", Rc::new(|_args: &[Value]| {
        Value::Bool(false)
    }));

    registry.register("main,pdiext,MSystemCondition,IsPaused", Rc::new(|_args: &[Value]| {
        Value::Bool(false)
    }));

    // Voucher/DLC
    registry.register("main,pdiext,MVoucher", Rc::new(|_args: &[Value]| {
        Value::Object(Rc::new(ObjectInstance {
            class_path: "MVoucher".to_string(),
            fields: vec![],
        }))
    }));

    registry.register("main,pdiext,MVoucher,processCode", Rc::new(|_args: &[Value]| {
        Value::Bool(true)
    }));

    // USB communication (GT5 link - stub)
    registry.register("main,pdiext,MUsbPspCommPSP", Rc::new(|_args: &[Value]| {
        Value::Object(Rc::new(ObjectInstance {
            class_path: "MUsbPspCommPSP".to_string(),
            fields: vec![],
        }))
    }));

    registry.register("main,pdiext,MUsbPspCommPSP,send", Rc::new(|_args: &[Value]| {
        Value::Bool(true)
    }));

    registry.register("main,pdiext,MUsbPspCommPSP,receive", Rc::new(|_args: &[Value]| {
        Value::Bool(true)
    }));

    // Network / Ad-hoc (placeholder stubs)
    registry.register("main,pdiext,MNetwork", Rc::new(|_args: &[Value]| {
        Value::Object(Rc::new(ObjectInstance {
            class_path: "MNetwork".to_string(),
            fields: vec![],
        }))
    }));

    registry.register("main,pdiext,MNetwork,getAdhocPeerList", Rc::new(|_args: &[Value]| {
        Value::Object(Rc::new(ObjectInstance {
            class_path: "MAdhocPeerList".to_string(),
            fields: vec![],
        }))
    }));

    registry.register("main,pdiext,MNetwork,getState", Rc::new(|_args: &[Value]| {
        Value::Int(0) // DISCONNECTED
    }));

    registry.register("main,pdiext,MNetwork,connect", Rc::new(|_args: &[Value]| {
        eprintln!("[MNetwork] connect (stub - no network)");
        Value::Int(0)
    }));

    registry.register("main,pdiext,MNetwork,disconnect", Rc::new(|_args: &[Value]| {
        Value::Void
    }));

    eprintln!("Registered additional pdiext stubs");
}
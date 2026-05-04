use std::rc::Rc;
use crate::vm::value::*;

/// Register pdistd (standard library) native APIs.
pub fn register_pdistd(registry: &mut crate::vm::native::NativeRegistry) {
    // MRandom
    registry.register("main,pdistd,MRandom", Rc::new(|_args: &[Value]| {
        Value::Object(Rc::new(ObjectInstance {
            class_path: "MRandom".to_string(),
            fields: vec![],
        }))
    }));

    registry.register("main,pdistd,MRandom,GetValue", Rc::new(|_args: &[Value]| {
        Value::Float(rand::random::<f32>())
    }));

    registry.register("main,pdistd,MRandom,GetValueRange", Rc::new(|args: &[Value]| {
        let min = args.first().and_then(|a| a.as_i32()).unwrap_or(0) as f32;
        let max = args.get(1).and_then(|a| a.as_i32()).unwrap_or(100) as f32;
        Value::Int((rand::random::<f32>() * (max - min) + min) as i32)
    }));

    // MTime
    registry.register("main,pdistd,MTime", Rc::new(|_args: &[Value]| {
        Value::Object(Rc::new(ObjectInstance {
            class_path: "MTime".to_string(),
            fields: vec![],
        }))
    }));

    registry.register("main,pdistd,MTime,getCurrentTime", Rc::new(|_args: &[Value]| {
        use std::time::{SystemTime, UNIX_EPOCH};
        let secs = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_secs();
        Value::Long(secs as i64)
    }));

    registry.register("main,pdistd,MTime,getSystemTimeLow", Rc::new(|_args: &[Value]| {
        use std::time::Instant;
        let now = Instant::now();
        Value::UInt(now.elapsed().subsec_millis() as u32)
    }));

    // MLocale
    registry.register("main,pdistd,MLocale", Rc::new(|_args: &[Value]| {
        Value::Object(Rc::new(ObjectInstance {
            class_path: "MLocale".to_string(),
            fields: vec![],
        }))
    }));

    registry.register("main,pdistd,MLocale,getLanguage", Rc::new(|_args: &[Value]| {
        Value::Int(1) // English
    }));

    registry.register("main,pdistd,MLocale,getCountry", Rc::new(|_args: &[Value]| {
        Value::Int(1) // US
    }));

    // MFile
    registry.register("main,pdistd,MFile", Rc::new(|_args: &[Value]| {
        Value::Object(Rc::new(ObjectInstance {
            class_path: "MFile".to_string(),
            fields: vec![],
        }))
    }));

    registry.register("main,pdistd,MFile,_exists", Rc::new(|args: &[Value]| {
        let path = args.first().map(|a| a.to_string()).unwrap_or_default();
        Value::Bool(std::path::Path::new(&path).exists())
    }));

    registry.register("main,pdistd,MFile,_stat", Rc::new(|args: &[Value]| {
        let path = args.first().map(|a| a.to_string()).unwrap_or_default();
        if let Ok(meta) = std::fs::metadata(&path) {
            Value::Int(meta.len() as i32)
        } else {
            Value::Int(-1)
        }
    }));

    // MXml - used for save data
    registry.register("main,pdistd,MXml,_new", Rc::new(|_args: &[Value]| {
        Value::Object(Rc::new(ObjectInstance {
            class_path: "MXml".to_string(),
            fields: vec![],
        }))
    }));

    registry.register("main,pdistd,MXml,load", Rc::new(|args: &[Value]| {
        let path = args.first().map(|a| a.to_string()).unwrap_or_default();
        Value::Bool(std::path::Path::new(&path).exists())
    }));

    registry.register("main,pdistd,MXml,save", Rc::new(|_args: &[Value]| {
        Value::Bool(true)
    }));

    registry.register("main,pdistd,MXml,parse", Rc::new(|_args: &[Value]| {
        Value::Bool(true)
    }));

    registry.register("main,pdistd,MXml,createRoot", Rc::new(|_args: &[Value]| {
        Value::Bool(true)
    }));

    registry.register("main,pdistd,MXml,createElement", Rc::new(|_args: &[Value]| {
        Value::Bool(true)
    }));

    registry.register("main,pdistd,MXml,addChild", Rc::new(|_args: &[Value]| {
        Value::Bool(true)
    }));

    // MDynRes (dynamic resources)
    registry.register("main,pdistd,MDynRes", Rc::new(|_args: &[Value]| {
        Value::Object(Rc::new(ObjectInstance {
            class_path: "MDynRes".to_string(),
            fields: vec![],
        }))
    }));

    // Deflate/Inflate (compression)
    registry.register("main,pdistd,Deflate", Rc::new(|_args: &[Value]| {
        Value::Object(Rc::new(ObjectInstance {
            class_path: "Deflate".to_string(),
            fields: vec![],
        }))
    }));

    registry.register("main,pdistd,Inflate", Rc::new(|_args: &[Value]| {
        Value::Object(Rc::new(ObjectInstance {
            class_path: "Inflate".to_string(),
            fields: vec![],
        }))
    }));

    // MEncryption
    registry.register("main,pdistd,MEncryption", Rc::new(|_args: &[Value]| {
        Value::Object(Rc::new(ObjectInstance {
            class_path: "MEncryption".to_string(),
            fields: vec![],
        }))
    }));

    // MCipher
    registry.register("main,pdistd,MCipher", Rc::new(|_args: &[Value]| {
        Value::Object(Rc::new(ObjectInstance {
            class_path: "MCipher".to_string(),
            fields: vec![],
        }))
    }));

    // MFloat (math)
    registry.register("main,pdistd,MFloat", Rc::new(|_args: &[Value]| {
        Value::Object(Rc::new(ObjectInstance {
            class_path: "MFloat".to_string(),
            fields: vec![],
        }))
    }));

    // Vector operations
    registry.register("main,pdistd,MVector3", Rc::new(|_args: &[Value]| {
        Value::Object(Rc::new(ObjectInstance {
            class_path: "MVector3".to_string(),
            fields: vec![],
        }))
    }));

    // MColor
    registry.register("main,pdistd,MColor", Rc::new(|_args: &[Value]| {
        Value::Object(Rc::new(ObjectInstance {
            class_path: "MColor".to_string(),
            fields: vec![],
        }))
    }));

    // MMatrix44
    registry.register("main,pdistd,MMatrix44", Rc::new(|_args: &[Value]| {
        Value::Object(Rc::new(ObjectInstance {
            class_path: "MMatrix44".to_string(),
            fields: vec![],
        }))
    }));

    // DialogUtil stubs
    registry.register("main,pdistd,DialogUtil,openConfirmDialog", Rc::new(|_args: &[Value]| {
        Value::Int(1) // success
    }));

    registry.register("main,pdistd,DialogUtil,openErrorDialog", Rc::new(|_args: &[Value]| {
        Value::Int(1)
    }));

    registry.register("main,pdistd,DialogUtil,showMessage", Rc::new(|_args: &[Value]| {
        Value::Int(1)
    }));

    // ReadFile
    registry.register("main,pdistd,ReadFile", Rc::new(|args: &[Value]| {
        let path = args.first().map(|a| a.to_string()).unwrap_or_default();
        match std::fs::read(&path) {
            Ok(data) => Value::Array(Rc::new(data.into_iter().map(|b| Value::Int(b as i32)).collect())),
            Err(_) => Value::Nil
        }
    }));

    // WriteFile  
    registry.register("main,pdistd,WriteFile", Rc::new(|args: &[Value]| {
        let path = args.first().map(|a| a.to_string()).unwrap_or_default();
        let data = args.get(1).map(|a| a.to_string()).unwrap_or_default();
        match std::fs::write(&path, data.as_bytes()) {
            Ok(_) => Value::Int(0),
            Err(_) => Value::Int(-1)
        }
    }));

    // AsciiStringHash - heavily used!
    registry.register("main,pdistd,AsciiStringHash", Rc::new(|args: &[Value]| {
        let s = args.first().map(|a| a.to_string()).unwrap_or_default();
        let mut hash: u32 = 0;
        for b in s.bytes() {
            hash = hash.wrapping_mul(31).wrapping_add(b as u32);
        }
        Value::Int(hash as i32)
    }));

    // MPDINetwork (network - stub)
    registry.register("main,pdistd,MPDINetwork", Rc::new(|_args: &[Value]| {
        Value::Object(Rc::new(ObjectInstance {
            class_path: "MPDINetwork".to_string(),
            fields: vec![],
        }))
    }));

    // MProductNetwork (another network reference)
    registry.register("main,pdistd,MProductNetwork", Rc::new(|_args: &[Value]| {
        Value::Object(Rc::new(ObjectInstance {
            class_path: "MProductNetwork".to_string(),
            fields: vec![],
        }))
    }));

    // GetArgs - return command-line args (empty for PC port)
    registry.register("main,pdistd,GetArgs", Rc::new(|_args: &[Value]| {
        Value::Array(Rc::new(vec![]))
    }));

    // MSound - sound system constructor
    registry.register("main,pdistd,MSound", Rc::new(|_args: &[Value]| {
        Value::Object(Rc::new(ObjectInstance {
            class_path: "MSound".to_string(),
            fields: vec![],
        }))
    }));

    // Module loading - search multiple asset dirs
    registry.register("main,pdistd,Module,load", Rc::new(|args: &[Value]| {
            let path = args.first().map(|a| a.to_string()).unwrap_or_default();
            eprintln!("[Module::load] CALLED path=\"{}\"", path);
            // Strip leading '/' or '//' from paths like "/scripts/gt5m/util/Foo"
            let clean = path.trim_start_matches('/').trim_start_matches('\\');
            let search_paths = [
                format!("assets/scripts/{}.adc", clean),
                format!("assets/projects/gt5m/{}.adc", path),
                format!("assets/projects/gt5m/{}/{}.adc", path, path),
                format!("assets/products/gt5m/{}.adc", path),
                format!("assets/products/gt5m/script/{}.adc", path),
            ];
            let mut loaded = false;
            for adc_path in &search_paths {
                if std::path::Path::new(adc_path).exists() {
                    match crate::vm::loader::load_adc_file(adc_path) {
                        Ok(idx) => {
                            eprintln!("[Module::load] Loaded {} (idx={})", adc_path, idx);
                            loaded = true;
                            break;
                        }
                        Err(e) => {
                            eprintln!("[Module::load] Error loading {}: {}", adc_path, e);
                        }
                    }
                }
            }
            if !loaded {
                eprintln!("[Module::load] NOT FOUND: \"{}\" (tried {:?})", path, search_paths);
            }
            Value::Int(if loaded { 1 } else { -1 })
        }));

    eprintln!("Registered {} pdistd stubs", 30);
}
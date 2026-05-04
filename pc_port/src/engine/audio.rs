/// GT PSP Audio — ffmpeg decodes ATRAC3 → PCM → SDL2_mixer plays it

use std::process::{Command, Stdio};
use std::sync::Mutex;
use std::rc::Rc;
use std::sync::OnceLock;

static MIXER_INIT: OnceLock<bool> = OnceLock::new();
static BGM_CHANNEL: Mutex<Option<i32>> = Mutex::new(None);

fn ensure_mixer() -> bool {
    #[cfg(windows)]
    {
        *MIXER_INIT.get_or_init(|| {
            if let Ok(()) = sdl2::mixer::open_audio(44100, sdl2::mixer::AUDIO_S16LSB, sdl2::mixer::DEFAULT_CHANNELS, 4096) {
                sdl2::mixer::allocate_channels(16);
                eprintln!("[Audio] SDL2_mixer ready");
                true
            } else {
                eprintln!("[Audio] SDL2_mixer unavailable");
                false
            }
        })
    }
    #[cfg(not(windows))] { false }
}

fn decode_at3(path: &str) -> Result<Vec<u8>, String> {
    let output = Command::new("ffmpeg")
        .args(&["-i", path, "-f", "wav", "-ac", "2", "-ar", "44100",
                "-acodec", "pcm_s16le", "-y", "pipe:1"])
        .stdout(Stdio::piped()).stderr(Stdio::null())
        .output().map_err(|e| format!("ffmpeg: {}", e))?;
    if !output.status.success() {
        return Err(format!("ffmpeg exit: {:?}", output.status.code()));
    }
    Ok(output.stdout)
}

pub fn register_audio(registry: &mut crate::vm::native::NativeRegistry) {
    use crate::vm::value::*;
    eprintln!("[Audio] registered (lazy SDL2_mixer + ffmpeg)");

    registry.register("main,pdiext,MSystemBGM", Rc::new(|_| {
        Value::Object(Rc::new(ObjectInstance { class_path: "MSystemBGM".to_string(), fields: vec![] }))
    }));

    registry.register("main,pdiext,MSystemBGM,play", Rc::new(|args| {
        let path = args.first().map(|a| a.to_string()).unwrap_or_default();
        let full_path = format!("assets/sound_gt/track/{}.at3",
            path.trim_end_matches(".at3").trim_end_matches(".wav"));

        if !ensure_mixer() {
            eprintln!("[BGM] play \"{}\" (headless, no audio)", full_path);
            return Value::Int(0);
        }

        eprintln!("[BGM] play \"{}\"", full_path);
        match decode_at3(&full_path) {
            Ok(wav) => {
                let samples: Box<[i16]> = if wav.len() > 44 {
                    let raw = &wav[44..];
                    raw.chunks(2).filter(|c| c.len() == 2)
                        .map(|c| i16::from_le_bytes([c[0], c[1]]))
                        .collect()
                } else { Box::new([]) };
                if let Ok(chunk) = sdl2::mixer::Chunk::from_raw_buffer(samples) {
                    match sdl2::mixer::Channel::all().play(&chunk, 0) {
                        Ok(ch) => { *BGM_CHANNEL.lock().unwrap() = Some(ch.0); eprintln!("[BGM] ch={}", ch.0); }
                        Err(e) => eprintln!("[BGM] {}", e),
                    }
                    Value::Int(0)
                } else { eprintln!("[BGM] bad chunk"); Value::Int(-1) }
            }
            Err(e) => { eprintln!("[BGM] {}", e); Value::Int(-1) }
        }
    }));

    registry.register("main,pdiext,MSystemBGM,stop", Rc::new(|_| {
        if let Some(ch) = *BGM_CHANNEL.lock().unwrap() {
            sdl2::mixer::Channel(ch).halt();
        }
        Value::Void
    }));

    registry.register("main,pdiext,MSystemBGM,pause", Rc::new(|_| {
        if let Some(ch) = *BGM_CHANNEL.lock().unwrap() { sdl2::mixer::Channel(ch).pause(); }
        Value::Void
    }));

    registry.register("main,pdiext,MSystemBGM,resume", Rc::new(|_| {
        if let Some(ch) = *BGM_CHANNEL.lock().unwrap() { sdl2::mixer::Channel(ch).resume(); }
        Value::Void
    }));

    registry.register("main,pdiext,MSystemBGM,setVolume", Rc::new(|args| {
        let vol = args.first().and_then(|v| v.as_i32()).unwrap_or(100).clamp(0, 128);
        sdl2::mixer::Channel::all().set_volume(vol);
        Value::Void
    }));

    registry.register("main,pdiext,MSystemBGM,setCue", Rc::new(|_| Value::Void));
    registry.register("main,pdiext,MSystemBGM,getContextSize", Rc::new(|_| Value::Int(44100)));
    registry.register("main,pdiext,MSystemBGM,fadeout", Rc::new(|_| Value::Void));
    registry.register("main,pdiext,MSystemBGM,getContext", Rc::new(|_| Value::Int(0)));
    registry.register("main,pdiext,MSystemBGM,setContext", Rc::new(|_| Value::Void));
    registry.register("main,pdiext,MSystemBGM,openDirectory", Rc::new(|_| Value::Void));

    registry.register("main,pdiapp,MEngineSound", Rc::new(|_| {
        Value::Object(Rc::new(ObjectInstance { class_path: "MEngineSound".to_string(), fields: vec![] }))
    }));
    registry.register("main,pdiapp,MEngineSound,loadPreset", Rc::new(|args| {
        eprintln!("[Engine] loadPreset: \"{}\"", args.first().map(|a| a.to_string()).unwrap_or_default());
        Value::Int(0)
    }));
    registry.register("main,pdiapp,MEngineSound,getContext", Rc::new(|_| Value::Int(0)));

    registry.register("main,pdiext,CarSound", Rc::new(|_| {
        Value::Object(Rc::new(ObjectInstance { class_path: "CarSound".to_string(), fields: vec![] }))
    }));
    registry.register("main,pdiext,CarSound,getContext", Rc::new(|_| {
        Value::Object(Rc::new(ObjectInstance { class_path: "CarSoundContext".to_string(), fields: vec![] }))
    }));
    registry.register("main,pdiext,CarSound,road_attribute_sound_parameter", Rc::new(|_| {
        Value::Object(Rc::new(ObjectInstance { class_path: "MRaceSound".to_string(), fields: vec![] }))
    }));
}

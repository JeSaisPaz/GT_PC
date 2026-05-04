use std::rc::Rc;
use std::cell::RefCell;
use std::fs::{self, File};
use std::path::Path;
use crate::vm::value::*;
use crate::engine::graphics::get_thread_renderer;
use crate::engine::graphics::LoadedTexture;
use crate::engine::graphics::ProjectorRef;
use crate::engine::GT_VOL_PATH;

const WINDOW_W: u32 = 960;
const WINDOW_H: u32 = 544;

#[derive(Clone, Copy)]
pub struct Mat4 {
    pub m: [f32; 16],
}

impl Mat4 {
    pub fn identity() -> Self {
        Mat4 { m: [1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0] }
    }

    pub fn perspective(fov: f32, aspect: f32, near: f32, far: f32) -> Self {
        let f = 1.0 / (fov / 2.0).tan();
        let nf = 1.0 / (near - far);
        // Row-major storage of standard OpenGL perspective matrix (transposed)
        Mat4 { m: [
            f / aspect, 0.0, 0.0, 0.0,
            0.0, f, 0.0, 0.0,
            0.0, 0.0, (far + near) * nf, 2.0 * far * near * nf,
            0.0, 0.0, -1.0, 0.0,
        ] }
    }

    pub fn look_at(eye: (f32, f32, f32), center: (f32, f32, f32), up: (f32, f32, f32)) -> Self {
        let (ex, ey, ez) = eye;
        let (cx, cy, cz) = center;
        let (ux, uy, uz) = up;
        let zx = ex - cx; let zy = ey - cy; let zz = ez - cz;
        let zlen = (zx*zx + zy*zy + zz*zz).sqrt();
        let zx = zx/zlen; let zy = zy/zlen; let zz = zz/zlen;
        let xx = uy*zz - uz*zy;
        let xy = uz*zx - ux*zz;
        let xz = ux*zy - uy*zx;
        let xlen = (xx*xx + xy*xy + xz*xz).sqrt();
        let xx = xx/xlen; let xy = xy/xlen; let xz = xz/xlen;
        let yx = zy*xz - zz*xy;
        let yy = zz*xx - zx*xz;
        let yz = zx*xy - zy*xx;
        Mat4 { m: [
            xx, yx, zx, 0.0,
            xy, yy, zy, 0.0,
            xz, yz, zz, 0.0,
            -(xx*ex + xy*ey + xz*ez), -(yx*ex + yy*ey + yz*ez), -(zx*ex + zy*ey + zz*ez), 1.0
        ] }
    }

    pub fn project_point(&self, x: f32, y: f32, z: f32) -> Option<(f32, f32, f32)> {
        // Row-major matrix-vector multiplication
        let w = self.m[12]*x + self.m[13]*y + self.m[14]*z + self.m[15];
        if w.abs() < 0.0001 { return None; }
        let px = (self.m[0]*x + self.m[1]*y + self.m[2]*z + self.m[3]) / w;
        let py = (self.m[4]*x + self.m[5]*y + self.m[6]*z + self.m[7]) / w;
        let pz = (self.m[8]*x + self.m[9]*y + self.m[10]*z + self.m[11]) / w;
        Some((px, py, pz))
    }
}

#[derive(Clone)]
pub struct Projector {
    pub view: Mat4,
    pub proj: Mat4,
    pub viewport: (u32, u32),
}

impl Projector {
    pub fn new() -> Self {
        Projector {
            view: Mat4::identity(),
            proj: Mat4::perspective(1.0, WINDOW_W as f32 / WINDOW_H as f32, 0.1, 1000.0),
            viewport: (WINDOW_W, WINDOW_H),
        }
    }

    pub fn set_camera(&mut self, eye: (f32, f32, f32), center: (f32, f32, f32), up: (f32, f32, f32)) {
        self.view = Mat4::look_at(eye, center, up);
    }

    pub fn project(&self, x: f32, y: f32, z: f32) -> Option<(i32, i32)> {
        if let Some((px, py, pz)) = self.proj.project_point(x, y, z) {
            let sx = ((px + 1.0) * 0.5 * self.viewport.0 as f32) as i32;
            let sy = ((1.0 - py) * 0.5 * self.viewport.1 as f32) as i32;
            if pz >= 0.0 && pz <= 1.0 { Some((sx, sy)) } else { None }
        } else { None }
    }
}

pub fn get_track_state() -> Rc<RefCell<TrackState>> {
    Rc::new(RefCell::new(TrackState::new()))
}

pub fn get_projector() -> Rc<RefCell<Projector>> {
    Rc::new(RefCell::new(Projector::new()))
}

fn read_u32(data: &[u8], off: usize) -> u32 {
    if off + 4 > data.len() { return 0; }
    u32::from_le_bytes([data[off], data[off + 1], data[off + 2], data[off + 3]])
}

fn read_u16(data: &[u8], off: usize) -> u16 {
    if off + 2 > data.len() { return 0; }
    u16::from_le_bytes([data[off], data[off + 1]])
}

fn load_img_texture(path: &str) -> Result<LoadedTexture, String> {
    let data = fs::read(path).map_err(|e| format!("Read {}: {}", path, e))?;
    crate::engine::sprite::parse_txs3_texture(&data)
}

/// Register all pdiapp native API stubs with the VM.
pub fn register_pdiapp(registry: &mut crate::vm::native::NativeRegistry) {
    // MTexture - local loader
    registry.register("main,pdiapp,MTexture", Rc::new(|_args: &[Value]| {
        Value::Object(Rc::new(ObjectInstance {
            class_path: "MTexture".to_string(),
            fields: vec![],
        }))
    }));

    registry.register("main,pdiapp,MTexture,load", Rc::new(move |args: &[Value]| {
        let path = args.first().map(|a| a.to_string()).unwrap_or_default();
        eprintln!("[MTexture] load path=\"{}\"", path);
        
        if Path::new(&path).exists() {
            match load_img_texture(&path) {
                Ok(tex) => {
                    eprintln!("[MTexture] loaded {}x{}", tex.width, tex.height);
                    let r_tex = get_thread_renderer();
                    let cached = r_tex.borrow_mut().cache_texture(&path, &tex);
                    eprintln!("[MTexture] cached={}", cached);
                    Value::Bool(cached)
                }
                Err(e) => {
                    eprintln!("[MTexture] load error: {}", e);
                    Value::Bool(false)
                }
            }
        } else {
            eprintln!("[MTexture] file not found: {}", path);
            Value::Bool(false)
        }
    }));

    // MGraphics - wired to thread-local renderer
    let r = get_thread_renderer();
    
    registry.register("main,pdiapp,MGraphics", Rc::new(move |_args: &[Value]| {
        Value::Object(Rc::new(ObjectInstance {
            class_path: "MGraphics".to_string(),
            fields: vec![],
        }))
    }));

    let r2 = r.clone();
    registry.register("main,pdiapp,MGraphics,beginScene", Rc::new(move |_args: &[Value]| {
        r2.borrow_mut().begin_scene();
        Value::Void
    }));

    let r3 = r.clone();
    registry.register("main,pdiapp,MGraphics,endScene", Rc::new(move |_args: &[Value]| {
        r3.borrow_mut().end_scene();
        Value::Void
    }));

    let r4 = r.clone();
    registry.register("main,pdiapp,MGraphics,clear", Rc::new(move |_args: &[Value]| {
        r4.borrow_mut().clear();
        Value::Void
    }));

    // MRender - wired to thread-local renderer
    let r_render = get_thread_renderer();
    
    registry.register("main,pdiapp,MRender", Rc::new(|_args: &[Value]| {
        Value::Object(Rc::new(ObjectInstance {
            class_path: "MRender".to_string(),
            fields: vec![],
        }))
    }));

    let r7 = r_render.clone();
    let proj = get_projector();
    
    registry.register("main,pdiapp,MRender,drawPolygon", Rc::new(move |args: &[Value]| {
        let vert_count = args.len() / 4;
        let mut vertices = Vec::with_capacity(vert_count);
        for i in 0..vert_count {
            let x = args.get(i * 4).and_then(|v| v.as_f32()).unwrap_or(0.0);
            let y = args.get(i * 4 + 1).and_then(|v| v.as_f32()).unwrap_or(0.0);
            let z = args.get(i * 4 + 2).and_then(|v| v.as_f32()).unwrap_or(0.0);
            vertices.push((x, y, z));
        }
        
        let tri_count = if vert_count >= 3 { (vert_count - 2) as u32 } else { 0 };
        let mut triangles = Vec::new();
        for i in 0..tri_count {
            if i == 0 {
                triangles.push((0u32, 1, 2));
            } else {
                triangles.push((0u32, i as u32 + 1, i as u32 + 2));
            }
        }
        
let _proj_ref = proj.borrow();
        
        let proj_simple = ProjectorRef;
        
        if !triangles.is_empty() {
            let drawn = r7.borrow_mut().project_and_fill_triangles(&vertices, &triangles, &proj_simple, sdl2::pixels::Color::RGB(100, 150, 200));
            eprintln!("[MRender] drawPolygon v={} tri={} filled={}", vert_count, tri_count, drawn);
        } else {
            let drawn = r7.borrow_mut().draw_polyline(&vertices, &proj_simple, sdl2::pixels::Color::RGB(200, 200, 200));
            eprintln!("[MRender] drawPolygon vertices={} wireframe={}", vert_count, drawn);
        }
        
        Value::Void
    }));

    let r8 = r_render.clone();
    registry.register("main,pdiapp,MRender,drawText", Rc::new(move |args: &[Value]| {
        let text = args.first().map(|a| a.to_string()).unwrap_or_default();
        let x = args.get(1).and_then(|v| v.as_i32()).unwrap_or(10);
        let y = args.get(2).and_then(|v| v.as_i32()).unwrap_or(10);
        let scale = args.get(3).and_then(|v| v.as_f32()).unwrap_or(1.0);
        r8.borrow_mut().draw_text(x, y, &text, 200, 200, 200, scale);
        Value::Void
    }));

    // MRender 3D matrix helpers
    let r_setview = r_render.clone();
    registry.register("main,pdiapp,MRender,setViewMatrix", Rc::new(move |args: &[Value]| {
        if args.len() >= 16 {
            let mut m = [0f32; 16];
            for i in 0..16 {
                m[i] = args.get(i).and_then(|v| v.as_f32()).unwrap_or(0.0);
            }
            r_setview.borrow_mut().view_matrix = m;
        }
        Value::Void
    }));

    let r_setproj = r_render.clone();
    registry.register("main,pdiapp,MRender,setProjectionMatrix", Rc::new(move |args: &[Value]| {
        if args.len() >= 16 {
            let mut m = [0f32; 16];
            for i in 0..16 {
                m[i] = args.get(i).and_then(|v| v.as_f32()).unwrap_or(0.0);
            }
            r_setproj.borrow_mut().proj_matrix = m;
        }
        Value::Void
    }));

    // MInput - wired to thread-local renderer for keyboard state
    let r_input = get_thread_renderer();
    
    registry.register("main,pdiapp,MInput", Rc::new(move |_args: &[Value]| {
        Value::Object(Rc::new(ObjectInstance {
            class_path: "MInput".to_string(),
            fields: vec![],
        }))
    }));

    let r10 = r_input.clone();
    registry.register("main,pdiapp,MInput,poll", Rc::new(move |_args: &[Value]| {
        let quit = r10.borrow_mut().is_quit();
        Value::Bool(quit)
    }));

    let r11 = r_input.clone();
    registry.register("main,pdiapp,MInput,isKeyDown", Rc::new(move |args: &[Value]| {
        let key = args.first().and_then(|a| a.as_i32()).unwrap_or(0) as usize;
        let keys = r11.borrow().get_key_state();
        // Simple bit check - key state is a u32 bitmask
        let down = (keys >> key) & 1 != 0;
        Value::Bool(down)
    }));

    registry.register("main,pdiapp,MInput,getAxis", Rc::new(|_args: &[Value]| {
        Value::Int(0)
    }));

    // MDEBUG - debugging APIs
    registry.register("main,pdiapp,MDEBUG", Rc::new(|_args: &[Value]| {
        Value::Object(Rc::new(ObjectInstance {
            class_path: "MDEBUG".to_string(),
            fields: vec![],
        }))
    }));

    registry.register("main,pdiapp,MDEBUG,print", Rc::new(|args: &[Value]| {
        let msg = args.iter().map(|a| a.to_string()).collect::<Vec<_>>().join(" ");
        eprintln!("[DEBUG] {}", msg);
        Value::Void
    }));

    registry.register("main,pdiapp,MDEBUG,drawText", Rc::new(|args: &[Value]| {
        let text = args.first().map(|a| a.to_string()).unwrap_or_default();
        eprintln!("[DEBUG] drawText: \"{}\"", text);
        Value::Void
    }));

    registry.register("main,pdiapp,MDEBUG,drawPolygon", Rc::new(|args: &[Value]| {
        let vert_count = args.len() / 4;
        eprintln!("[DEBUG] drawPolygon: {} vertices", vert_count);
        Value::Void
    }));

    registry.register("main,pdiapp,MDEBUG,drawLine", Rc::new(|args: &[Value]| {
        eprintln!("[DEBUG] drawLine");
        Value::Void
    }));

    registry.register("main,pdiapp,MDEBUG,setDebugLevel", Rc::new(|args: &[Value]| {
        let level = args.first().and_then(|v| v.as_i32()).unwrap_or(0);
        eprintln!("[DEBUG] setDebugLevel: {}", level);
        Value::Void
    }));

    registry.register("main,pdiapp,MDEBUG,getDebugLevel", Rc::new(|_args: &[Value]| {
        Value::Int(0)
    }));

    // MVideo
    registry.register("main,pdiapp,MVideo", Rc::new(|_args: &[Value]| {
        Value::Object(Rc::new(ObjectInstance {
            class_path: "MVideo".to_string(),
            fields: vec![],
        }))
    }));

    registry.register("main,pdiapp,MVideo,play", Rc::new(|_args: &[Value]| Value::Bool(true)));
    registry.register("main,pdiapp,MVideo,stop", Rc::new(|_args: &[Value]| Value::Void));

    // MMotion - animation
    registry.register("main,pdiapp,MMotion", Rc::new(|_args: &[Value]| {
        Value::Object(Rc::new(ObjectInstance {
            class_path: "MMotion".to_string(),
            fields: vec![],
        }))
    }));

    registry.register("main,pdiapp,MMotion,play", Rc::new(|_args: &[Value]| Value::Void));

    // MSceneGraph
    registry.register("main,pdiapp,MSceneGraph", Rc::new(|_args: &[Value]| {
        Value::Object(Rc::new(ObjectInstance {
            class_path: "MSceneGraph".to_string(),
            fields: vec![],
        }))
    }));

    // MCourse - track model loader
    registry.register("main,pdiapp,MCourse", Rc::new(|_args: &[Value]| {
        Value::Object(Rc::new(ObjectInstance {
            class_path: "MCourse".to_string(),
            fields: vec![],
        }))
    }));

    registry.register("main,pdiapp,MCourse,getCoursePath", Rc::new(|args: &[Value]| {
        let course_id = args.first().map(|a| a.to_string()).unwrap_or_default();
        let path = format!("{}/crs/{:03}.mdl", GT_VOL_PATH, course_id.parse::<u32>().unwrap_or(0));
        Value::String(Rc::new(path))
    }));

    registry.register("main,pdiapp,MCourse,loadCourse", Rc::new(|args: &[Value]| {
        let path = args.first().map(|a| a.to_string()).unwrap_or_default();
        eprintln!("[MCourse] loadCourse path=\"{}\"", path);
        Value::Bool(true)
    }));

    eprintln!("Registered {} pdiapp stubs", 40);
}

// Re-export model types and functions for convenience
pub use crate::engine::model::{CarModel, TrackState, CameraData, load_car_model, load_course, load_camera};
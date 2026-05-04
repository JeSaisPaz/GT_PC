// ─── SDL2 + ab_glyph rendering backend ──────────────────────────
// Thread-local renderer via Rc<RefCell<GraphicsRenderer>>,
// software-rasterized lines/triangles/text, and texture cache.

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;
use sdl2::pixels::Color;
use sdl2::render::{Canvas, Texture, TextureCreator};
use sdl2::video::Window;
use sdl2::event::Event;
use sdl2::keyboard::Scancode;
use sdl2::EventPump;

#[cfg(windows)]
use gl::types::{GLuint, GLenum, GLint};

// ─── Thread-local renderer ──────────────────────────────────────

thread_local! {
    static RENDERER: RefCell<Option<Rc<RefCell<GraphicsRenderer>>>> = RefCell::new(None);
    static OPENGL_RENDERER: RefCell<Option<OpenGLRenderer>> = RefCell::new(None);
}

pub fn init_renderer_win(headless: bool, w: u32, h: u32) -> Option<Rc<RefCell<GraphicsRenderer>>> {
    let gr = GraphicsRenderer::new(headless, w, h);
    let rc = Rc::new(RefCell::new(gr));
    let clone = rc.clone();
    RENDERER.with(|r| { *r.borrow_mut() = Some(rc); });
    Some(clone)
}

pub fn init_renderer() -> Option<Rc<RefCell<GraphicsRenderer>>> {
    // Don't override if already initialized with a real window
    let needs_init = RENDERER.with(|r| r.borrow().is_none());
    if needs_init {
        init_renderer_win(false, 960, 544)
    } else {
        // Already have a headless renderer — upgrade to windowed
        init_renderer_win(false, 960, 544)
    }
}

pub fn get_thread_renderer() -> Rc<RefCell<GraphicsRenderer>> {
    RENDERER.with(|r| {
        if r.borrow().is_none() {
            // Auto-init headless (window will be created later by init_renderer)
            let gr = GraphicsRenderer::new_headless();
            *r.borrow_mut() = Some(Rc::new(RefCell::new(gr)));
        }
        r.borrow().clone().expect("GraphicsRenderer not initialized")
    })
}

pub fn has_renderer() -> bool {
    RENDERER.with(|r| r.borrow().is_some())
}

#[cfg(windows)]
pub fn init_opengl(w: i32, h: i32) -> Option<OpenGLRenderer> {
    init_opengl_internal(w, h)
}

#[cfg(windows)]
fn init_opengl_internal(w: i32, h: i32) -> Option<OpenGLRenderer> {
    unsafe {
        let result = std::panic::catch_unwind(|| {
            gl::ClearColor(0.1, 0.1, 0.2, 1.0);
            gl::Enable(gl::DEPTH_TEST);
            gl::Enable(gl::CULL_FACE);
            gl::CullFace(gl::BACK);
        });
        
        if result.is_err() {
            return None;
        }

            let vao = create_vao();
            let vbo = create_vbo();
            let ebo = create_ebo();
            let shader = match compile_shader_program() {
                Ok(s) => s,
                Err(e) => {
                    eprintln!("Warning: shader compile failed: {}", e);
                    return None;
                }
            };
            let textured_shader = compile_textured_shader_program().unwrap_or(shader);
            
            let renderer = OpenGLRenderer {
                width: w,
                height: h,
                vao,
                vbo,
                ebo,
                shader_program: shader,
                textured_shader_program: textured_shader,
                view_matrix: identity_matrix(),
                proj_matrix: default_projection(w as f32, h as f32),
                textures: std::collections::HashMap::new(),
            };
        
        OPENGL_RENDERER.with(|r| { *r.borrow_mut() = Some(renderer.clone()); });
        
        Some(renderer)
    }
}

#[cfg(windows)]
pub fn get_opengl_renderer() -> Option<OpenGLRenderer> {
    OPENGL_RENDERER.with(|r| {
        let opt = r.borrow();
        opt.as_ref().map(|r| r.clone())
    })
}

// ─── ProjectorRef (placeholder for 3D projection) ────────────────

pub struct ProjectorRef;
impl ProjectorRef {
    pub fn project(&self, x: f32, y: f32, _z: f32) -> (i32, i32) {
        (x as i32, y as i32)
    }
}

// ─── GraphicsRenderer ───────────────────────────────────────────

pub struct GraphicsRenderer {
    pub canvas: Option<Canvas<Window>>,
    event_pump: Option<EventPump>,
    pub width: i32,
    pub height: i32,
    key_state: u32,
    pub proj_matrix: [f32; 16],
    pub view_matrix: [f32; 16],
    sdl_ctx: Option<sdl2::Sdl>,
    pub tex_cache: HashMap<String, LoadedTexture>,
}

fn default_projection(w: f32, h: f32) -> [f32; 16] {
    let aspect = w / h;
    let f = 1.0 / (1.2f32 / 2.0).tan();
    let nf = 1.0 / (0.1 - 500.0);
    [f/aspect, 0.0, 0.0, 0.0, 0.0, f, 0.0, 0.0, 0.0, 0.0, (500.0+0.1)*nf, 2.0*500.0*0.1*nf, 0.0, 0.0, -1.0, 0.0]
}

fn identity_matrix() -> [f32; 16] {
    [1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0]
}

impl GraphicsRenderer {
    pub fn new(headless: bool, w: u32, h: u32) -> Self {
        if headless {
            GraphicsRenderer {
                canvas: None, event_pump: None,
                width: w as i32, height: h as i32,
                key_state: 0,
                proj_matrix: default_projection(w as f32, h as f32),
                view_matrix: identity_matrix(),
                sdl_ctx: None, tex_cache: HashMap::new(),
            }
        } else {
            let sdl = sdl2::init().expect("SDL2 init failed");
            let video = sdl.video().expect("SDL2 video init failed");
            
            let window = video.window("GT PSP PC Port", w, h)
                .position_centered()
                .resizable()
                .build()
                .expect("SDL2 window creation failed");
            
            // Try to get GL context
            let gl_context = window.gl_create_context();
            
            // Note: GL init attempted after window is ready
            // For now, use SDL2 canvas
            let canvas = window.into_canvas()
                .accelerated()
                .present_vsync()
                .build()
                .expect("SDL2 canvas creation failed");
            
            let pump = sdl.event_pump().expect("SDL2 event pump creation failed");
            
            eprintln!("[Game] Window ready (SDL2 canvas)");
            
            GraphicsRenderer {
                canvas: Some(canvas), event_pump: Some(pump),
                width: w as i32, height: h as i32, key_state: 0,
                proj_matrix: default_projection(w as f32, h as f32),
                view_matrix: identity_matrix(),
                sdl_ctx: Some(sdl), tex_cache: HashMap::new(),
            }
        }
    }

    pub fn new_headless() -> Self {
        GraphicsRenderer {
            canvas: None, event_pump: None,
            width: 960, height: 544, key_state: 0,
            proj_matrix: default_projection(960.0, 544.0),
            view_matrix: identity_matrix(),
            sdl_ctx: None, tex_cache: HashMap::new(),
        }
    }

    pub fn clear(&mut self) {
        if let Some(ref mut c) = self.canvas {
            c.set_draw_color(Color::RGB(0, 0, 0));
            c.clear();
        }
        self.update_key_state();
    }

    pub fn begin_scene(&mut self) {}
    pub fn end_scene(&mut self) {
        if let Some(ref mut c) = self.canvas { c.present(); }
    }
    pub fn present(&mut self) {
        if let Some(ref mut c) = self.canvas { c.present(); }
    }

    fn update_key_state(&mut self) {
        if let Some(ref pump) = self.event_pump {
            let keys = pump.keyboard_state();
            let mut state: u32 = 0;
            let mappings: [(Scancode, u32); 12] = [
                (Scancode::Backspace, 0), (Scancode::Return, 1),
                (Scancode::Up, 2), (Scancode::Right, 3),
                (Scancode::Down, 4), (Scancode::Left, 5),
                (Scancode::LShift, 6), (Scancode::RShift, 7),
                (Scancode::W, 8), (Scancode::D, 9),
                (Scancode::S, 10), (Scancode::A, 11),
            ];
            for (sc, bit) in &mappings {
                if keys.is_scancode_pressed(*sc) { state |= 1 << bit; }
            }
            self.key_state = state;
        }
    }

    pub fn poll_event(&mut self) -> Option<String> {
        if let Some(ref mut pump) = self.event_pump {
            for event in pump.poll_iter() {
                match event {
                    Event::Quit { .. } => return Some("quit".to_string()),
                    Event::KeyDown { scancode: Some(code), .. } => {
                        if code == Scancode::Escape { return Some("quit".to_string()); }
                    }
                    _ => {}
                }
            }
        }
        None
    }

    pub fn get_key_state(&self) -> u32 { self.key_state }
    pub fn screen_width(&self) -> i32 { self.width }
    pub fn screen_height(&self) -> i32 { self.height }
    pub fn set_viewport(&mut self, _x: i32, _y: i32, _w: i32, _h: i32) {}

    pub fn set_color(&mut self, r: u8, g: u8, b: u8, _a: u8) {
        if let Some(ref mut c) = self.canvas {
            c.set_draw_color(Color::RGB(r, g, b));
        }
    }

    pub fn draw_line(&mut self, x1: i32, y1: i32, x2: i32, y2: i32, r: u8, g: u8, b: u8) -> i32 {
        if let Some(ref mut c) = self.canvas {
            c.set_draw_color(Color::RGB(r, g, b));
            if c.draw_line((x1, y1), (x2, y2)).is_ok() { 1 } else { 0 }
        } else { 0 }
    }

    pub fn draw_rect(&mut self, x: i32, y: i32, w: i32, h: i32, r: u8, g: u8, b: u8) {
        if let Some(ref mut c) = self.canvas {
            c.set_draw_color(Color::RGB(r, g, b));
            let _ = c.draw_rect(sdl2::rect::Rect::new(x, y, w as u32, h as u32));
        }
    }

    pub fn fill_rect(&mut self, x: i32, y: i32, w: i32, h: i32, r: u8, g: u8, b: u8) {
        if let Some(ref mut c) = self.canvas {
            c.set_draw_color(Color::RGB(r, g, b));
            let _ = c.fill_rect(sdl2::rect::Rect::new(x, y, w as u32, h as u32));
        }
    }

    pub fn draw_text(&mut self, x: i32, y: i32, text: &str, r: u8, g: u8, b: u8, scale: f32) {
        self.draw_text_align(x, y, text, r, g, b, scale, 3);
    }

    pub fn draw_text_align(&mut self, x: i32, y: i32, text: &str, r: u8, g: u8, b: u8, scale: f32, align: i32) {
        if text.is_empty() || self.canvas.is_none() { return; }
        use ab_glyph::{Font, PxScale, point};
        let font_data = include_bytes!("../../assets/arial.ttf");
        if let Ok(font) = ab_glyph::FontRef::try_from_slice(font_data) {
            let c = self.canvas.as_mut().unwrap();
            let size = (18.0 * scale) as f32;
            let glyph_step = size * 0.55;
            let text_w = text.chars().count() as f32 * glyph_step + 4.0;
            let text_h = size * 1.3;
            let x_cursor = match align {
                2 | 4 => (x as f32 - text_w).max(0.0),
                1 | 3 => (x as f32 - text_w * 0.5).max(0.0),
                _ => x as f32,
            };

            // Render entire string into a single bitmap
            let bw = text_w.ceil() as u32;
            let bh = text_h.ceil() as u32;
            if bw == 0 || bh == 0 { return; }
            let mut pixels = vec![0u8; (bw * bh * 4) as usize];
            let mut cursor = 0.0f32;
            for ch in text.chars() {
                let glyph = font.glyph_id(ch).with_scale_and_position(PxScale::from(size), point(cursor, size));
                if let Some(outlined) = font.outline_glyph(glyph) {
                    outlined.draw(|px, py, v| {
                        let gx = px as u32;
                        let gy = py as u32;
                        if gx < bw && gy < bh {
                            let idx = ((gy * bw + gx) * 4) as usize;
                            if idx + 3 < pixels.len() {
                                let a = (v * 255.0).min(255.0) as u8;
                                pixels[idx] = r;
                                pixels[idx+1] = g;
                                pixels[idx+2] = b;
                                pixels[idx+3] = pixels[idx+3].saturating_add(a);
                            }
                        }
                    });
                }
                cursor += glyph_step;
            }
            if let Ok(surf) = sdl2::surface::Surface::from_data(&mut pixels, bw, bh, bw*4, sdl2::pixels::PixelFormatEnum::ARGB8888) {
                if let Ok(tex) = c.texture_creator().create_texture_from_surface(&surf) {
                    let _ = c.copy(&tex, None, sdl2::rect::Rect::new(x_cursor as i32, y, bw, bh));
                }
            }
        }
    }

    pub fn draw_polyline(&mut self, verts: &[(f32, f32, f32)], _proj: &ProjectorRef, color: sdl2::pixels::Color) -> i32 {
        if verts.len() < 2 || self.canvas.is_none() { return 0; }
        let c = self.canvas.as_mut().unwrap();
        let half_w = self.width as f32 * 0.5;
        let half_h = self.height as f32 * 0.5;
        c.set_draw_color(color);
        let mut count = 0;
        for i in 0..verts.len() - 1 {
            let (x1, y1, _z1) = verts[i];
            let (x2, y2, _) = verts[i + 1];
            let sx1 = (x1 + 1.0) * half_w;
            let sy1 = self.height as f32 - (y1 + 1.0) * half_h;
            let sx2 = (x2 + 1.0) * half_w;
            let sy2 = self.height as f32 - (y2 + 1.0) * half_h;
            if c.draw_line((sx1 as i32, sy1 as i32), (sx2 as i32, sy2 as i32)).is_ok() {
                count += 1;
            }
        }
        count
    }

    pub fn project_and_fill_triangles(&mut self, verts: &[(f32, f32, f32)], tris: &[(u32, u32, u32)], _proj: &ProjectorRef, color: sdl2::pixels::Color) -> i32 {
        if tris.is_empty() || self.canvas.is_none() { return 0; }
        let c = self.canvas.as_mut().unwrap();
        let half_w = self.width as f32 * 0.5;
        let half_h = self.height as f32 * 0.5;
        c.set_draw_color(color);
        let mut count = 0;
        for (a, b, cc) in tris {
            let i_a = *a as usize;
            let i_b = *b as usize;
            let i_c = *cc as usize;
            if i_a >= verts.len() || i_b >= verts.len() || i_c >= verts.len() { continue; }
            let (ax, ay, _) = verts[i_a];
            let (bx, by, _) = verts[i_b];
            let (cx, cy, _) = verts[i_c];
            let sax = (ax + 1.0) * half_w;
            let say = self.height as f32 - (ay + 1.0) * half_h;
            let sbx = (bx + 1.0) * half_w;
            let sby = self.height as f32 - (by + 1.0) * half_h;
            let scx = (cx + 1.0) * half_w;
            let scy = self.height as f32 - (cy + 1.0) * half_h;
            for dy in -1..=1 {
                let _ = c.draw_line((sax as i32, say as i32 + dy), (sbx as i32, sby as i32 + dy));
                let _ = c.draw_line((sbx as i32, sby as i32 + dy), (scx as i32, scy as i32 + dy));
                let _ = c.draw_line((scx as i32, scy as i32 + dy), (sax as i32, say as i32 + dy));
            }
            count += 1;
        }
        count
    }

    pub fn draw_screen_line(&mut self, x0: f32, y0: f32, x1: f32, y1: f32, r: u8, g: u8, b: u8) {
        if let Some(ref mut c) = self.canvas {
            let half_w = self.width as f32 * 0.5;
            let half_h = self.height as f32 * 0.5;
            let sx0 = ((x0 + 1.0) * half_w) as i32;
            let sy0 = (self.height as f32 - (y0 + 1.0) * half_h) as i32;
            let sx1 = ((x1 + 1.0) * half_w) as i32;
            let sy1 = (self.height as f32 - (y1 + 1.0) * half_h) as i32;
            c.set_draw_color(Color::RGB(r, g, b));
            let _ = c.draw_line((sx0, sy0), (sx1, sy1));
        }
    }

    pub fn canvas_set_color(&mut self, color: sdl2::pixels::Color) {
        if let Some(ref mut c) = self.canvas { c.set_draw_color(color); }
    }

    pub fn canvas_draw_line(&mut self, x0: i32, y0: i32, x1: i32, y1: i32) -> bool {
        if let Some(ref mut c) = self.canvas { c.draw_line((x0, y0), (x1, y1)).is_ok() } else { false }
    }

    pub fn cache_texture(&mut self, path: &str, tex: &LoadedTexture) -> bool {
        if tex.width == 0 || tex.height == 0 || tex.rgba.is_empty() { return false; }
        self.tex_cache.insert(path.to_string(), tex.clone());
        true
    }

    pub fn draw_texture(&mut self, path: &str, x: i32, y: i32, w: u32, h: u32) {
        // Draw as colored rectangle using the texture's dominant color
        // to avoid expensive SDL texture creation every frame
        if let Some(tex) = self.tex_cache.get(path) {
            let (r, g, b, a) = dominant_rgba(&tex.rgba, tex.width, tex.height);
            if a > 0 {
                self.fill_rect(x, y, w as i32, h as i32, r, g, b);
            }
        }
    }

    pub fn draw_texture_raw(&mut self, path: &str, x: i32, y: i32, w: u32, h: u32) {
        // Original SDL2 texture path (expensive, use sparingly)
        self.draw_texture_region(path, x, y, w, h, 0, 0, 0, 0);
    }

    pub fn draw_texture_region(&mut self, path: &str, x: i32, y: i32, w: u32, h: u32, _sx: u32, _sy: u32, _sw: u32, _sh: u32) {
        if let Some(tex) = self.tex_cache.get(path) {
            if let Some(ref mut c) = self.canvas {
                if tex.width > 0 && tex.height > 0 && !tex.rgba.is_empty() {
                    let bw = tex.width as usize;
                    let sw = _sw.max(tex.width) as usize;
                    let sx = _sx as usize;
                    let sy = _sy as usize;
                    let mut pixels = vec![0u8; sw * tex.height as usize * 4];
                    let mut idx = 0;
                    for py in 0..tex.height as usize {
                        for px in 0..sw {
                            let src = ((sy + py) * bw + (sx + px)) * 4;
                            if src + 3 < tex.rgba.len() {
                                pixels[idx] = tex.rgba[src];
                                pixels[idx+1] = tex.rgba[src+1];
                                pixels[idx+2] = tex.rgba[src+2];
                                pixels[idx+3] = tex.rgba[src+3];
                            }
                            idx += 4;
                        }
                    }
                    if let Ok(surf) = sdl2::surface::Surface::from_data(&mut pixels, sw as u32, tex.height, (sw*4) as u32, sdl2::pixels::PixelFormatEnum::ARGB8888) {
                        if let Ok(sdl_tex) = c.texture_creator().create_texture_from_surface(&surf) {
                            let _ = c.copy(&sdl_tex, None, sdl2::rect::Rect::new(x, y, w, h));
                        }
                    }
                }
            }
        }
    }

    pub fn get_texture(&self, path: &str) -> Option<&LoadedTexture> {
        self.tex_cache.get(path)
    }

    pub fn has_texture(&self, path: &str) -> bool {
        self.tex_cache.contains_key(path)
    }

    pub fn is_quit(&mut self) -> bool {
        false
    }
}

// ─── LoadedTexture ───────────────────────────────────────────────

#[derive(Clone)]
pub struct LoadedTexture {
    pub width: u32,
    pub height: u32,
    pub rgba: Vec<u8>,
}

impl LoadedTexture {
    pub fn new() -> Self {
        LoadedTexture { width: 0, height: 0, rgba: vec![] }
    }
    pub fn len(&self) -> usize { self.rgba.len() }
}

/// Compute the dominant RGBA color from texture pixel data.
fn dominant_rgba(rgba: &[u8], _w: u32, _h: u32) -> (u8, u8, u8, u8) {
    if rgba.len() < 4 { return (0, 0, 0, 0); }
    let count = rgba.len() / 4;
    let mut sum_r: u64 = 0;
    let mut sum_g: u64 = 0;
    let mut sum_b: u64 = 0;
    let mut sum_a: u64 = 0;
    let mut opaque_count: u64 = 0;
    for i in 0..count {
        let off = i * 4;
        let a = rgba[off + 3] as u64;
        if a > 128 {
            sum_r += rgba[off] as u64;
            sum_g += rgba[off + 1] as u64;
            sum_b += rgba[off + 2] as u64;
            sum_a += a;
            opaque_count += 1;
        }
    }
    if opaque_count == 0 {
        for i in 0..count {
            let off = i * 4;
            sum_r += rgba[off] as u64;
            sum_g += rgba[off + 1] as u64;
            sum_b += rgba[off + 2] as u64;
            sum_a += rgba[off + 3] as u64;
            opaque_count += 1;
        }
    }
    if opaque_count == 0 { return (0, 0, 0, 0); }
    let r = (sum_r / opaque_count).min(255) as u8;
    let g = (sum_g / opaque_count).min(255) as u8;
    let b = (sum_b / opaque_count).min(255) as u8;
    let a = (sum_a / opaque_count).min(255) as u8;
    (r, g, b, a)
}

// ─── OpenGL Renderer ────────────────────────────────────────────────

#[cfg(windows)]
#[derive(Clone)]
pub struct OpenGLTexture {
    pub id: GLuint,
    pub width: i32,
    pub height: i32,
}

#[cfg(windows)]
impl OpenGLTexture {
    pub fn from_loaded_texture(tex: &LoadedTexture) -> Option<Self> {
        unsafe {
            let mut id: GLuint = 0;
            gl::GenTextures(1, &mut id);
            if id == 0 { return None; }
            
            gl::BindTexture(gl::TEXTURE_2D, id);
            
            // Set texture parameters
            gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_WRAP_S, gl::REPEAT as i32);
            gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_WRAP_T, gl::REPEAT as i32);
            gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MIN_FILTER, gl::LINEAR as i32);
            gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MAG_FILTER, gl::LINEAR as i32);
            
            // Upload pixel data
            gl::TexImage2D(
                gl::TEXTURE_2D,
                0,
                gl::RGBA as i32,
                tex.width as i32,
                tex.height as i32,
                0,
                gl::RGBA,
                gl::UNSIGNED_BYTE,
                tex.rgba.as_ptr() as *const std::ffi::c_void
            );
            
            gl::BindTexture(gl::TEXTURE_2D, 0);
            
            Some(OpenGLTexture {
                id,
                width: tex.width as i32,
                height: tex.height as i32,
            })
        }
    }
    
    pub fn bind(&self, slot: u32) {
        unsafe {
            gl::ActiveTexture(gl::TEXTURE0 + slot);
            gl::BindTexture(gl::TEXTURE_2D, self.id);
        }
    }
    
    pub fn unbind(&self) {
        unsafe {
            gl::BindTexture(gl::TEXTURE_2D, 0);
        }
    }
}

#[cfg(windows)]
#[derive(Clone)]
pub struct OpenGLRenderer {
    pub width: i32,
    pub height: i32,
    pub vao: GLuint,
    pub vbo: GLuint,
    pub ebo: GLuint,
    pub shader_program: GLuint,
    pub textured_shader_program: GLuint,
    pub view_matrix: [f32; 16],
    pub proj_matrix: [f32; 16],
    pub textures: std::collections::HashMap<String, OpenGLTexture>,
}

#[cfg(windows)]
impl OpenGLRenderer {
    pub fn new(w: i32, h: i32) -> Option<Self> {
        // For lazy init - actual init happens in init_opengl_internal
        // with context already available
        unsafe {
            let vao = create_vao();
            let vbo = create_vbo();
            let ebo = create_ebo();
            let shader = compile_shader_program().ok()?;
            let textured_shader = compile_textured_shader_program().ok()?;
            
            Some(OpenGLRenderer {
                width: w,
                height: h,
                vao,
                vbo,
                ebo,
                shader_program: shader,
                textured_shader_program: textured_shader,
                view_matrix: identity_matrix(),
                proj_matrix: default_projection(w as f32, h as f32),
                textures: std::collections::HashMap::new(),
            })
        }
    }
    
    pub fn clear(&mut self) {
        unsafe {
            gl::Clear(gl::COLOR_BUFFER_BIT | gl::DEPTH_BUFFER_BIT);
        }
    }
    
    pub fn set_view(&mut self, view: [f32; 16]) {
        self.view_matrix = view;
    }
    
    pub fn set_projection(&mut self, proj: [f32; 16]) {
        self.proj_matrix = proj;
    }
    
    /// Upload and cache a texture from LoadedTexture
    pub fn upload_texture(&mut self, name: &str, tex: &LoadedTexture) -> bool {
        if let Some(gl_tex) = OpenGLTexture::from_loaded_texture(tex) {
            self.textures.insert(name.to_string(), gl_tex);
            return true;
        }
        false
    }
    
    /// Check if a texture is cached
    pub fn has_texture(&self, name: &str) -> bool {
        self.textures.contains_key(name)
    }
    
    /// Draw mesh with solid color (no texture)
    pub fn draw_mesh(&mut self, vertices: &[f32], indices: &[u32], color: (f32, f32, f32)) {
        self.draw_mesh_internal(vertices, indices, color, false);
    }
    
    /// Draw mesh with texture and UV coordinates
    /// vertices format: [x,y,z, u,v, x,y,z, u,v, ...] - 5 floats per vertex
    pub fn draw_mesh_textured(&mut self, vertices: &[f32], indices: &[u32], texture_name: &str) {
        // Get texture ID first to avoid borrow issues
        let tex_id = self.textures.get(texture_name).map(|t| t.id);
        if let Some(id) = tex_id {
            unsafe {
                gl::ActiveTexture(gl::TEXTURE0);
                gl::BindTexture(gl::TEXTURE_2D, id);
            }
            self.draw_mesh_internal(vertices, indices, (1.0, 1.0, 1.0), true);
            unsafe {
                gl::BindTexture(gl::TEXTURE_2D, 0);
            }
        }
    }
    
    fn draw_mesh_internal(&mut self, vertices: &[f32], indices: &[u32], 
                          color: (f32, f32, f32), 
                          use_textured_shader: bool) {
        unsafe {
            let program = if use_textured_shader { 
                self.textured_shader_program 
            } else { 
                self.shader_program 
            };
            gl::UseProgram(program);
            
            // Set uniforms
            let v_loc = gl::GetUniformLocation(program, b"view\0".as_ptr() as *const i8);
            let p_loc = gl::GetUniformLocation(program, b"proj\0".as_ptr() as *const i8);
            gl::UniformMatrix4fv(v_loc, 1, gl::FALSE, self.view_matrix.as_ptr());
            gl::UniformMatrix4fv(p_loc, 1, gl::FALSE, self.proj_matrix.as_ptr());
            
            if use_textured_shader {
                // Set texture sampler
                let t_loc = gl::GetUniformLocation(program, b"texSampler\0".as_ptr() as *const i8);
                gl::Uniform1i(t_loc, 0);  // Texture unit 0
            } else {
                let c_loc = gl::GetUniformLocation(program, b"fragColor\0".as_ptr() as *const i8);
                gl::Uniform3f(c_loc, color.0, color.1, color.2);
            }
            
            // Bind VAO and upload data
            gl::BindVertexArray(self.vao);
            gl::BindBuffer(gl::ARRAY_BUFFER, self.vbo);
            gl::BufferData(gl::ARRAY_BUFFER, 
                (vertices.len() * std::mem::size_of::<f32>()) as isize,
                vertices.as_ptr() as *const std::ffi::c_void,
                gl::DYNAMIC_DRAW);
            
            if use_textured_shader {
                // Position attribute (location 0) - 3 floats, stride 5 floats
                gl::VertexAttribPointer(0, 3, gl::FLOAT, gl::FALSE, 
                    (5 * std::mem::size_of::<f32>()) as i32, std::ptr::null());
                gl::EnableVertexAttribArray(0);
                // UV attribute (location 1) - 2 floats, stride 5 floats, offset 3 floats
                gl::VertexAttribPointer(1, 2, gl::FLOAT, gl::FALSE,
                    (5 * std::mem::size_of::<f32>()) as i32, 
                    (3 * std::mem::size_of::<f32>()) as *const std::ffi::c_void);
                gl::EnableVertexAttribArray(1);
            } else {
                // Position attribute (location 0) - 3 floats
                gl::VertexAttribPointer(0, 3, gl::FLOAT, gl::FALSE, 
                    (3 * std::mem::size_of::<f32>()) as i32, std::ptr::null());
                gl::EnableVertexAttribArray(0);
            }
            
            // Upload indices
            gl::BindBuffer(gl::ELEMENT_ARRAY_BUFFER, self.ebo);
            gl::BufferData(gl::ELEMENT_ARRAY_BUFFER,
                (indices.len() * std::mem::size_of::<u32>()) as isize,
                indices.as_ptr() as *const std::ffi::c_void,
                gl::DYNAMIC_DRAW);
            
            // Draw
            gl::DrawElements(gl::TRIANGLES, indices.len() as i32, gl::UNSIGNED_INT, std::ptr::null());
            
            gl::BindVertexArray(0);
        }
    }
    
    pub fn draw_wireframe(&mut self, vertices: &[f32], indices: &[u32], color: (f32, f32, f32)) {
        unsafe {
            gl::PolygonMode(gl::FRONT_AND_BACK, gl::LINE);
            self.draw_mesh(vertices, indices, color);
            gl::PolygonMode(gl::FRONT_AND_BACK, gl::FILL);
        }
    }
}

#[cfg(windows)]
unsafe fn create_vao() -> GLuint {
    let mut vao: GLuint = 0;
    gl::GenVertexArrays(1, &mut vao);
    vao
}

#[cfg(windows)]
unsafe fn create_vbo() -> GLuint {
    let mut vbo: GLuint = 0;
    gl::GenBuffers(1, &mut vbo);
    vbo
}

#[cfg(windows)]
unsafe fn create_ebo() -> GLuint {
    let mut ebo: GLuint = 0;
    gl::GenBuffers(1, &mut ebo);
    ebo
}

#[cfg(windows)]
unsafe fn compile_shader_program() -> Result<GLuint, String> {
    let vert_src = r#"
        #version 330 core
        layout(location = 0) in vec3 position;
        uniform mat4 view;
        uniform mat4 proj;
        void main() {
            gl_Position = proj * view * vec4(position, 1.0);
        }
    "#;
    
    let frag_src = r#"
        #version 330 core
        precision mediump float;
        uniform vec3 fragColor;
        out vec4 fragColorOut;
        void main() {
            fragColorOut = vec4(fragColor, 1.0);
        }
    "#;
    
    let vert = compile_shader(gl::VERTEX_SHADER, vert_src)?;
    let frag = compile_shader(gl::FRAGMENT_SHADER, frag_src)?;
    
    let program = gl::CreateProgram();
    gl::AttachShader(program, vert);
    gl::AttachShader(program, frag);
    gl::LinkProgram(program);
    
    // Check link status
    let mut status: i32 = 0;
    gl::GetProgramiv(program, gl::LINK_STATUS, &mut status);
    if status == 0 {
        let mut len: i32 = 0;
        gl::GetProgramiv(program, gl::INFO_LOG_LENGTH, &mut len);
        let log = vec![0u8; len as usize];
        gl::GetProgramInfoLog(program, len, std::ptr::null_mut(), log.as_ptr() as *mut i8);
        return Err(String::from_utf8_lossy(&log).to_string());
    }
    
    gl::DeleteShader(vert);
    gl::DeleteShader(frag);
    
    Ok(program)
}

#[cfg(windows)]
unsafe fn compile_shader(ty: gl::types::GLenum, src: &str) -> Result<GLuint, String> {
    let shader = gl::CreateShader(ty);
    let ptr = src.as_ptr() as *const i8;
    let len = src.len() as i32;
    gl::ShaderSource(shader, 1, &ptr, &len);
    gl::CompileShader(shader);
    
    // Check compile status
    let mut status: i32 = 0;
    gl::GetShaderiv(shader, gl::COMPILE_STATUS, &mut status);
    if status == 0 {
        let mut len: i32 = 0;
        gl::GetShaderiv(shader, gl::INFO_LOG_LENGTH, &mut len);
        let log = vec![0u8; len as usize];
        gl::GetShaderInfoLog(shader, len, std::ptr::null_mut(), log.as_ptr() as *mut i8);
        return Err(String::from_utf8_lossy(&log).to_string());
    }
    
    Ok(shader)
}

#[cfg(windows)]
unsafe fn compile_textured_shader_program() -> Result<GLuint, String> {
    let vert_src = r#"
        #version 330 core
        layout(location = 0) in vec3 position;
        layout(location = 1) in vec2 texCoord;
        uniform mat4 view;
        uniform mat4 proj;
        out vec2 vTexCoord;
        void main() {
            gl_Position = proj * view * vec4(position, 1.0);
            vTexCoord = texCoord;
        }
    "#;
    
    let frag_src = r#"
        #version 330 core
        precision mediump float;
        in vec2 vTexCoord;
        uniform sampler2D texSampler;
        out vec4 fragColorOut;
        void main() {
            fragColorOut = texture(texSampler, vTexCoord);
        }
    "#;
    
    let vert = compile_shader(gl::VERTEX_SHADER, vert_src)?;
    let frag = compile_shader(gl::FRAGMENT_SHADER, frag_src)?;
    
    let program = gl::CreateProgram();
    gl::AttachShader(program, vert);
    gl::AttachShader(program, frag);
    gl::LinkProgram(program);
    
    // Check link status
    let mut status: i32 = 0;
    gl::GetProgramiv(program, gl::LINK_STATUS, &mut status);
    if status == 0 {
        let mut len: i32 = 0;
        gl::GetProgramiv(program, gl::INFO_LOG_LENGTH, &mut len);
        let log = vec![0u8; len as usize];
        gl::GetProgramInfoLog(program, len, std::ptr::null_mut(), log.as_ptr() as *mut i8);
        return Err(String::from_utf8_lossy(&log).to_string());
    }
    
    gl::DeleteShader(vert);
    gl::DeleteShader(frag);
    
    Ok(program)
}

// ─── Matrix Utilities for OpenGL ─────────────────────────────

#[cfg(windows)]
pub fn perspective_matrix(fov: f32, aspect: f32, near: f32, far: f32) -> [f32; 16] {
    let f = 1.0 / (fov / 2.0).tan();
    let nf = 1.0 / (near - far);
    [
        f / aspect, 0.0, 0.0, 0.0,
        0.0, f, 0.0, 0.0,
        0.0, 0.0, (far + near) * nf, -1.0,
        0.0, 0.0, 2.0 * far * near * nf, 0.0,
    ]
}

#[cfg(windows)]
pub fn look_at_matrix(eye: (f32, f32, f32), center: (f32, f32, f32), up: (f32, f32, f32)) -> [f32; 16] {
    let zx = eye.0 - center.0;
    let zy = eye.1 - center.1;
    let zz = eye.2 - center.2;
    let mut z_len = (zx*zx + zy*zy + zz*zz).sqrt();
    if z_len == 0.0 { z_len = 1.0; }
    let z = (zx/z_len, zy/z_len, zz/z_len);
    
    let xx = up.1 * z.2 - up.2 * z.1;
    let xy = up.2 * z.0 - up.0 * z.2;
    let xz = up.0 * z.1 - up.1 * z.0;
    let mut x_len = (xx*xx + xy*xy + xz*xz).sqrt();
    if x_len == 0.0 { x_len = 1.0; }
    let x = (xx/x_len, xy/x_len, xz/x_len);
    
    let y = (z.1 * x.2 - z.2 * x.1, z.2 * x.0 - z.0 * x.2, z.0 * x.1 - z.1 * x.0);
    
    [
        x.0, y.0, z.0, 0.0,
        x.1, y.1, z.1, 0.0,
        x.2, y.2, z.2, 0.0,
        -(x.0*eye.0 + x.1*eye.1 + x.2*eye.2),
        -(y.0*eye.0 + y.1*eye.1 + y.2*eye.2),
        -(z.0*eye.0 + z.1*eye.1 + z.2*eye.2),
        1.0,
    ]
}

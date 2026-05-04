---
tags: [pc-port, rust, graphics, sdl2]
type: documentation
project: GT PSP PC Port
section: Core Engine
---

# Graphics Renderer — PC Port

> SDL2 rendering backend (`pc_port/src/engine/graphics.rs`).

## Overview

Thread-local renderer with software rasterization for triangles/lines/text.

## Thread-Local Renderer

```rust
thread_local! {
    static RENDERER: RefCell<Option<Rc<RefCell<GraphicsRenderer>>>> = RefCell::new(None);
}

pub fn get_thread_renderer() -> Rc<RefCell<GraphicsRenderer>> {
    // Auto-init headless if not initialized
}
```

## GraphicsRenderer Structure

```rust
pub struct GraphicsRenderer {
    pub canvas: Option<Canvas<Window>>,
    event_pump: Option<EventPump>,
    pub width: i32,
    pub height: i32,
    key_state: u32,
    pub proj_matrix: [f32; 16],  // 4x4 projection
    pub view_matrix: [f32; 16], // 4x4 view
    sdl_ctx: Option<sdl2::Sdl>,
    pub tex_cache: HashMap<String, LoadedTexture>,
}
```

## Projection Matrix

```rust
fn default_projection(w: f32, h: f32) -> [f32; 16] {
    let aspect = w / h;
    let f = 1.0 / (1.2f32 / 2.0).tan();
    let nf = 1.0 / (0.1 - 500.0);
    // Perspective projection
    [f/aspect, 0.0, 0.0, 0.0,
     0.0, f, 0.0, 0.0,
     0.0, 0.0, (500.0+0.1)*nf, 2.0*500.0*0.1*nf,
     0.0, 0.0, -1.0, 0.0]
}
```

## Window Setup

```rust
impl GraphicsRenderer {
    pub fn new(headless: bool, w: u32, h: u32) -> Self {
        if headless {
            // No window
            GraphicsRenderer { canvas: None, ... }
        } else {
            let sdl = sdl2::init().unwrap();
            let video = sdl.video().unwrap();
            let window = video.window("GT PSP PC Port", w, h)
                .position_centered()
                .resizable()
                .build()
                .unwrap();
            let canvas = window.into_canvas()
                .accelerated()
                .present_vsync()
                .build()
                .unwrap();
            // ...
        }
    }
}
```

## Rendering Methods

| Method | Purpose |
|--------|---------|
| `clear()` | Clear framebuffer |
| `begin_scene()` | Scene begin |
| `end_scene()` | Scene end + present |
| `fill_rect()` | 2D rectangle |
| `draw_line()` | Line segment |
| `draw_triangle()` | Wireframe triangle |
| `project_and_fill_triangles()` | Wireframe only (no fill) |
| `fill_triangle()` | Scanline fill (per-triangle, slow) |
| `draw_text()` | Text rendering (ab_glyph) |

## Current Rendering Pipeline

### Wireframe (working)

```rust
// graphics.rs:291 - project_and_fill_triangles
// Draws triangle OUTLINES only (3 lines per triangle)
// Used by UI and some render passes
for (a, b, c) in tris {
    c.draw_line((sax, say), (sbx, sby));
    c.draw_line((sbx, sby), (scx, scy));
    c.draw_line((scx, scy), (sax, say));
}
```

### Solid Fill (broken/slow)

```rust
// race.rs:516 - fill_triangle
// Scanline algorithm:
// 1. Sort vertices by Y
// 2. Split into top/bottom halves
// 3. Interpolate X at each scanline
// 4. Fill between edges

// Called from:
// - race.rs:385 (track triangles)
// - race.rs:414 (car triangles)
```

## Issue: Triangle Fill

**Current state**: Scanline fill exists but may have bugs:

1. **Sorting** - Vertices sorted by Y, but edge cases for flat triangles
2. **Interpolation** - Linear interp may produce gaps
3. **Performance** - Per-triangle scanline is slow (~1000 triangles = lag)
4. **Depth** - No Z-buffer, painter's algorithm needed

### Potential Fixes

| Approach | Complexity | Speed |
|----------|------------|-------|
| Fix scanline bug | Low | Slow |
| Batch scanline | Medium | Medium |
| Software rasterizer | High | Fast |
| Use GPU (OpenGL) | High | Fastest |

## Current Bug: Scanline Fill

**Location**: `race.rs:516-566`

**Issue**: Interpolation formula error in scanline algorithm

```rust
// Current (buggy):
for y in v0.1..=v1.1 {
    let t = (y - v0.1) as f32 / total_height as f32;  // WRONG: uses total height
    let t_seg = if v1.1 != v0.1 { (y - v0.1) as f32 / (v1.1 - v0.1) as f32 } else { 0.0 };
    let mut xa = v0.0 + (t * (v2.0 - v0.0) as f32) as i32;
    let mut xb = v0.0 + (t_seg * (v1.0 - v0.0) as f32) as i32;
    // ...
}

// Should be:
// Top half: interpolate between left edge (v0→v2) and right edge (v0→v1)
// Bottom half: interpolate between left edge (v0→v2) and right edge (v1→v2)
```

**Fix needed**:

## OpenGL Backend Added ✅

```rust
// graphics.rs - OpenGLRenderer (lines 475-716)
pub struct OpenGLRenderer {
    pub vao: GLuint,
    pub vbo: GLuint,
    pub ebo: GLuint,
    pub shader_program: GLuint,
    pub textured_shader_program: GLuint,  // For textured rendering
    pub textures: HashMap<String, OpenGLTexture>,
}

impl OpenGLRenderer {
    pub fn new(w, h) -> Self
    pub fn clear(&mut self)
    pub fn draw_mesh(&mut self, vertices, indices, color)           // Solid color
    pub fn draw_mesh_textured(&mut self, vertices, indices, texture) // Textured
    pub fn draw_wireframe(&mut self, vertices, indices, color)
    pub fn upload_texture(&mut self, name, &LoadedTexture) -> bool
}

// Matrix utilities
pub fn perspective_matrix(fov, aspect, near, far) -> [f32; 16]
pub fn look_at_matrix(eye, center, up) -> [f32; 16]
```

## Texture Pipeline ✅ (2026-04-29)

### Texture Upload and Binding

```rust
// OpenGLTexture - GPU texture wrapper
pub struct OpenGLTexture {
    pub id: GLuint,
    pub width: i32,
    pub height: i32,
}

impl OpenGLTexture {
    pub fn from_loaded_texture(tex: &LoadedTexture) -> Option<Self>
    pub fn bind(&self, slot: u32)
    pub fn unbind(&self)
}
```

### Shader Programs

Two shader programs are now compiled:

```rust
// 1. Solid color shader (position only)
let shader_program = compile_shader_program()?;  // Existing

// 2. Textured shader (position + UV)
let textured_shader_program = compile_textured_shader_program()?;  // NEW
```

**Textured Vertex Shader:**
```glsl
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
```

**Textured Fragment Shader:**
```glsl
#version 330 core
in vec2 vTexCoord;
uniform sampler2D texSampler;
out vec4 fragColorOut;
void main() {
    fragColorOut = texture(texSampler, vTexCoord);
}
```

### Texture Rendering in Race

```rust
// race.rs: render_opengl()

// Upload textures during initialization
if let Some(ref car_tex) = self.car_texture {
    glr.upload_texture("car_tex", car_tex);
}
if let Some(ref course_tex) = self.course_texture {
    glr.upload_texture("course_tex", course_tex);
}

// Render with texture if UVs available
if track.has_uvs && glr.has_texture("course_tex") {
    // Textured path: vertices = [x,y,z,u,v, ...] (5 floats per vertex)
    glr.draw_mesh_textured(&vertices, &indices, "course_tex");
} else {
    // Solid color fallback
    glr.draw_mesh(&vertices, &indices, color);
}
```

## Current State

- **Active**: SDL2 canvas + GL context
- **OpenGL**: Ready (context created, shaders work)

## SDL2 + GL Context

Window now requests GL context on init:

```rust
let window = video.window("GT PSP PC Port", w, h)
    .gl_create_context();
// Canvas still used for 2D (falls back to software)
```

## Current Rendering

SDL2 wireframe (GL functions available but need full context setup)

## Known Issues

- [ ] [[10_PC_Port/12_Render_Issue|Render Issue]] — Red screen (course texture 8x512, OpenGL not initialized)
- [ ] Full OpenGL rendering (needs renderer integration)

## See Also

- [[10_PC_Port/00_Index|PC Port Index]]
- [[10_PC_Port/10_OpenGL_Backend|OpenGL Backend]]
- [[30_Technical/02_Textures|Textures]]
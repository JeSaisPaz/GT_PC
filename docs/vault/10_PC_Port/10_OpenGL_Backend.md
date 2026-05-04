---
tags: [graphics, opengl, sdl2, rendering]
type: documentation
project: GT PSP PC Port
section: Technical
---

# OpenGL Rendering Backend

> Adding GPU-accelerated rendering via SDL2 + OpenGL.

## Current State

- `gl` crate already in `Cargo.toml` (unused)
- SDL2 canvas currently does software rendering
- Need to add OpenGL context and GPU rendering

## Implementation Plan

### 1. OpenGL Context (SDL2)

```rust
// Instead of Canvas<Window>, use OpenGL context
let gl_attr = [
    (sdl2::video::GLAttr::RedSize, 5),
    (sdl2::video::GLAttr::GreenSize, 5),
    (sdl2::video::GLAttr::BlueSize, 5),
    (sdl2::video::GLAttr::AlphaSize, 0),
    (sdl2::video::GLAttr::DepthSize, 16),
    (sdl2::video::GLAttr::DoubleBuffer, 1),
];

// Create OpenGL window
let window = video.window("GT PSP PC Port", 960, 544)
    .gl_attributes()
    .resizable()
    .build()
    .expect("GL window");

let _gl_context = window.gl_create_context().expect("GL context");
```

### 2. Basic OpenGL Setup

```rust
// graphics.rs - OpenGLRenderer struct
pub struct OpenGLRenderer {
    pub width: i32,
    pub height: i32,
    pub vao: u32,           // Vertex array object
    pub vbo: u32,          // Vertex buffer
    pub shader_program: u32,  // Shader program
}

// Vertex format: x, y, z, r, g, b (6 floats per vertex)
struct Vertex {
    position: [f32; 3],
    color: [f32; 3],
}
```

### 3. Shaders

#### Vertex Shader

```glsl
#version 330 core
layout(location = 0) in vec3 position;
layout(location = 1) in vec3 color;

uniform mat4 view;
uniform mat4 proj;

out vec3 fragColor;

void main() {
    gl_Position = proj * view * vec4(position, 1.0);
    fragColor = color;
}
```

#### Fragment Shader

```glsl
#version 330 core
in vec3 fragColor;
out vec4 fragColorOut;

void main() {
    fragColorOut = vec4(fragColor, 1.0);
}
```

### 4. Rendering Methods

```rust
impl OpenGLRenderer {
    pub fn new(w: i32, h: i32) -> Self {
        // Compile shaders
        // Create VAO/VBO
        // Enable depth test
        gl::Enable(gl::DEPTH_TEST);
    }
    
    pub fn clear(&mut self) {
        unsafe {
            gl::ClearColor(0.1, 0.1, 0.2, 1.0);
            gl::Clear(gl::COLOR_BUFFER_BIT | gl::DEPTH_BUFFER_BIT);
        }
    }
    
    pub fn draw_triangles(&mut self, vertices: &[f32], colors: &[f32]) {
        // Bind VAO/VBO
        // glDrawArrays(gl::TRIANGLES, 0, count)
    }
    
    pub fn present(&mut self) {
        // glSwapBuffers handled by SDL
        self.window.gl_swap_window();
    }
}
```

### 5. Matrix Functions

```rust
fn perspective_matrix(fov: f32, aspect: f32, near: f32, far: f32) -> [f32; 16] {
    let f = 1.0 / (fov / 2.0).tan();
    let nf = 1.0 / (near - far);
    [
        f / aspect, 0.0, 0.0, 0.0,
        0.0, f, 0.0, 0.0,
        0.0, 0.0, (far + near) * nf, -1.0,
        0.0, 0.0, 2.0 * far * near * nf, 0.0,
    ]
}

fn look_at_matrix(eye: [f32; 3], center: [f32; 3], up: [f32; 3]) -> [f32; 16] {
    // Standard lookAt implementation
}
```

### 6. Triang [[3D model]]

```rust
pub fn upload_model(&mut self, vertices: &[f32], triangles: &[(u32, u32, u32)]) {
    // Convert to indexed vertices
    // Upload to VBO
}
```

### 7. Integration

```rust
// In main loop:
// 1. Clear (glClear)
// 2. Set matrices (glUniformMatrix4fv)
// 3. Draw track (glDrawElements)
// 4. Draw car (transform + draw)
// 5. Swap buffers
```

## Key Differences: SDL2 Canvas vs OpenGL

| Feature | SDL2 Canvas | OpenGL |
|---------|-------------|-------|
| Triangle fill | Software scanline | GPU |
| Depth buffer | Manual | Automatic |
| Performance | Slow | Fast |
| Code complexity | Simple | Moderate |
| Text | ab_glyph | ab_glyph → texture |

## Files to Modify

1. `graphics.rs` — Add OpenGLRenderer
2. `race.rs` — Use OpenGL for rendering
3. `Cargo.toml` — Already has `gl` crate

## Progress

- [ ] Add OpenGL window context
- [ ] Compile vertex/fragment shaders
- [ ] Create VAO/VBO helpers
- [ ] Upload triangle geometry
- [ ] Draw with depth test
- [ ] Wireframe mode (debug)
- [ ] Solid fill mode
- [ ] Text via texture

## See Also

- [[10_PC_Port/05_Graphics|Graphics]]
- [[10_PC_Port/02_Race_Engine|Race Engine]]
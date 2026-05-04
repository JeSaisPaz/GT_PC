// Race gameplay engine — full physics, collision, lap detection.
// Drives the RaceRunning phase in the native main loop.

use crate::engine::model::{load_course, load_car_model, load_car_texture, load_course_texture, TrackState};
use crate::engine::pdiapp::Mat4;
#[cfg(windows)]
use crate::engine::graphics::{get_thread_renderer, get_opengl_renderer, look_at_matrix, perspective_matrix, LoadedTexture};
use sdl2::pixels::Color;

// ─── Physics Constants ────────────────────────────────────────
const ACCEL_RATE: f32 = 35.0;
const BRAKE_RATE: f32 = 60.0;
const STEER_RATE: f32 = 3.0;
const DRAG_COEFF: f32 = 0.35;
const OFF_TRACK_DRAG: f32 = 5.0;
const MAX_SPEED: f32 = 90.0;
const GRAVITY: f32 = -20.0;
const TRACK_BOUNDARY_DIST: f32 = 8.0; // max distance to nearest triangle to be "on track"
const LAP_CROSS_DIST: f32 = 15.0;     // distance from start point to trigger lap crossing

#[derive(Clone)]
pub struct CarState {
    pub x: f32, pub y: f32, pub z: f32,
    pub heading: f32,
    pub speed: f32,
    pub steer_angle: f32,
    pub throttle: f32,
    pub brake: f32,
    pub car_id: u32,
    pub on_track: bool,
}

impl Default for CarState {
    fn default() -> Self {
        CarState {
            x: 0.0, y: 0.5, z: 0.0,
            heading: 0.0, speed: 0.0,
            steer_angle: 0.0, throttle: 0.0, brake: 0.0,
            car_id: 1, on_track: true,
        }
    }
}

pub struct RaceState {
    pub car: CarState,
    pub camera: ChaseCamera,
    pub track: Option<TrackState>,
    pub car_model: Option<crate::engine::model::CarModel>,
    pub car_texture: Option<LoadedTexture>,
    pub course_texture: Option<LoadedTexture>,
    pub course_id: u32,
    pub current_lap: i32,
    pub total_laps: i32,
    pub elapsed: f32,
    pub finished: bool,
    pub started: bool,
    pub initialized: bool,
    // Lap timing
    pub best_lap: f32,
    pub last_lap_time: f32,
    // Checkpoints (for sector timing)
    pub checkpoints_passed: Vec<bool>,
    pub sector_times: Vec<f32>,
    pub last_checkpoint_time: f32,
    // Lap detection state
    start_z: f32,
    start_x: f32,
    in_start_zone: bool,
    last_lap_z: f32,
    // Track spatial index for fast lookup
    track_grid: Option<TrackGrid>,
}

pub struct ChaseCamera {
    pub distance: f32,
    pub height: f32,
    pub fov: f32,
}

/// Spatial grid for fast triangle lookups (height, off-track check).
#[derive(Clone)]
struct TrackGrid {
    min_x: f32, max_x: f32, min_z: f32, max_z: f32,
    cells_x: usize, cells_z: usize,
    cell_w: f32, cell_h: f32,
    /// Each cell holds indices into track.triangles
    cell_tris: Vec<Vec<usize>>,
}

impl Default for RaceState {
    fn default() -> Self {
        RaceState {
            car: CarState::default(),
            camera: ChaseCamera { distance: 15.0, height: 6.0, fov: 1.2 },
            track: None, car_model: None,
            car_texture: None, course_texture: None,
            course_id: 1, current_lap: 0, total_laps: 3,
            elapsed: 0.0, finished: false, started: false, initialized: false,
            best_lap: 999.0, last_lap_time: 0.0,
            checkpoints_passed: Vec::new(),
            sector_times: Vec::new(),
            last_checkpoint_time: 0.0,
            start_z: 0.0, start_x: 0.0,
            in_start_zone: true, last_lap_z: 0.0,
            track_grid: None,
        }
    }
}

impl RaceState {
    pub fn new() -> Self { Self::default() }

    pub fn initialize(&mut self, course_id: u32, car_code: u32) {
        eprintln!("[Race] Loading course c{:03}.ad...", course_id);
        match load_course(course_id) {
            Ok(track) => {
                self.track_grid = build_track_grid(&track);
                self.track = Some(track);
            }
            Err(e) => { eprintln!("[Race] Track load error: {}", e); }
        }

        eprintln!("[Race] Loading car model 0x{:08X}...", car_code);
        match load_car_model(car_code) {
            Ok(model) => { self.car_model = Some(model); }
            Err(e) => { eprintln!("[Race] Car model error: {}", e); }
        }

        eprintln!("[Race] Loading car texture 0x{:08X}...", car_code);
        self.car_texture = load_car_texture(car_code);
        if self.car_texture.is_some() {
            eprintln!("[Race] Car texture loaded ({}x{})",
                self.car_texture.as_ref().unwrap().width,
                self.car_texture.as_ref().unwrap().height);
        }

        eprintln!("[Race] Loading course texture...");
        self.course_texture = load_course_texture();
        if self.course_texture.is_some() {
            eprintln!("[Race] Course texture loaded ({}x{})",
                self.course_texture.as_ref().unwrap().width,
                self.course_texture.as_ref().unwrap().height);
        }

        // Upload textures to OpenGL if available
        #[cfg(windows)]
        if let Some(ref mut glr) = get_opengl_renderer() {
            if let Some(ref car_tex) = self.car_texture {
                if glr.upload_texture("car_tex", car_tex) {
                    eprintln!("[Race] Car texture uploaded to OpenGL");
                }
            }
            if let Some(ref course_tex) = self.course_texture {
                if glr.upload_texture("course_tex", course_tex) {
                    eprintln!("[Race] Course texture uploaded to OpenGL");
                }
            }
        }

        self.car.car_id = car_code;
        
        // Setup checkpoints based on track position (3 checkpoints = 4 sectors)
        let num_checkpoints = 3;
        self.checkpoints_passed = vec![false; num_checkpoints];
        self.sector_times = vec![0.0; num_checkpoints];
        
        if let Some(ref track) = self.track {
            // Place car near track center, guard against NaN/Inf
            let cx = track.center.0;
            let cz = track.center.2;
            self.car.x = if cx.is_finite() { cx } else { 0.0 };
            self.car.z = if cz.is_finite() { cz } else { 0.0 };
            if let Some(h) = self.find_track_height(self.car.x, self.car.z) {
                self.car.y = if h.is_finite() { h } else { 0.5 };
            }
            self.start_x = self.car.x;
            self.start_z = self.car.z;
        }
        self.course_id = course_id;
        self.current_lap = 1;
        self.elapsed = 0.0;
        self.finished = false;
        self.started = false;
        self.in_start_zone = true;
        self.last_lap_z = self.car.z;
        self.initialized = true;
        eprintln!("[Race] Init: car ({:.1},{:.1},{:.1}) heading {:.1}° lap {}/{}",
            self.car.x, self.car.y, self.car.z,
            self.car.heading.to_degrees(), self.current_lap, self.total_laps);
    }

    /// Per-frame update. Called from main loop's RaceRunning phase.
    pub fn update(&mut self, dt: f32, throttle: bool, brake: bool, steer_left: bool, steer_right: bool) {
        if !self.initialized || self.finished { return; }
        let dt = dt.min(0.05); // cap to prevent physics blowup
        self.elapsed += dt;

        // ── Input ──────────────────────────────────────────────
        self.car.throttle = if throttle { 1.0 } else { 0.0 };
        self.car.brake = if brake { 1.0 } else { 0.0 };
        self.car.steer_angle = 0.0;
        if steer_left { self.car.steer_angle -= 1.0; }
        if steer_right { self.car.steer_angle += 1.0; }

        // ── Acceleration / Braking ─────────────────────────────
        if self.car.throttle > 0.0 {
            let accel = self.car.throttle * ACCEL_RATE;
            self.car.speed += accel * dt;
        }
        if self.car.brake > 0.0 && self.car.speed > 0.0 {
            self.car.speed -= self.car.brake * BRAKE_RATE * dt;
        }

        // Drag — much higher when off-track
        let drag = if self.car.on_track { DRAG_COEFF } else { OFF_TRACK_DRAG };
        self.car.speed -= self.car.speed * drag * dt;
        self.car.speed = self.car.speed.clamp(0.0, MAX_SPEED);

        // ── Steering (speed-dependent) ─────────────────────────
        if self.car.speed > 0.5 {
            let steer_factor = self.car.speed / (self.car.speed + 8.0); // more stable at high speed
            let turn_rate = self.car.steer_angle * STEER_RATE * steer_factor;
            self.car.heading += turn_rate * dt;
        } else {
            self.car.heading += self.car.steer_angle * 1.2 * dt; // pivot in place
        }

        // ── Position Update ────────────────────────────────────
        let dx = self.car.speed * self.car.heading.sin() * dt;
        let dz = self.car.speed * self.car.heading.cos() * dt;
        let new_x = self.car.x + dx;
        let new_z = self.car.z + dz;

        // ── Track Surface Following ────────────────────────────
        let min_dist = self.closest_triangle_distance(new_x, new_z);
        self.car.on_track = min_dist < TRACK_BOUNDARY_DIST;

        if self.car.on_track {
            // On track — follow surface height
            if let Some(h) = self.find_track_height(new_x, new_z) {
                self.car.x = new_x;
                self.car.z = new_z;
                self.car.y = h;
            }
        } else {
            // Off track — keep moving but with heavy drag
            self.car.x = new_x;
            self.car.z = new_z;
            self.car.y = self.car.y - 0.5 * dt; // gravity while airborne
            if self.car.y < -5.0 { self.car.y = -5.0; }
        }

        // ── Lap Detection ──────────────────────────────────────
        // State machine: car must ENTER → EXIT → RE-ENTER start zone
        // to count a lap. Prevents false counting when spawning at start.
        let dist_from_start = ((self.car.x - self.start_x).powi(2) + (self.car.z - self.start_z).powi(2)).sqrt();

        if !self.in_start_zone && dist_from_start < LAP_CROSS_DIST {
            // Re-entered the start zone after having left it
            self.in_start_zone = true;
            if self.started {
                if self.current_lap < self.total_laps {
                    // Calculate lap time
                    let lap_time = self.elapsed - self.last_lap_time;
                    self.last_lap_time = self.elapsed;
                    
                    // Update best lap
                    if lap_time < self.best_lap && self.current_lap > 0 {
                        self.best_lap = lap_time;
                        eprintln!("[Race] NEW BEST LAP: {:.1}s!", lap_time);
                    }
                    
                    self.current_lap += 1;
                    eprintln!("[Race] LAP {} / {} ({:.1}s)", self.current_lap, self.total_laps, lap_time);
                    self.last_lap_z = self.car.z;
                } else {
                    let final_time = self.elapsed;
                    eprintln!("[Race] FINISHED! Time: {:.1}s (Best: {:.1}s)", final_time, self.best_lap);
                    self.finished = true;
                    return;
                }
            } else {
                eprintln!("[Race] Race started!");
                self.started = true;
                self.last_lap_z = self.car.z;
                self.last_lap_time = 0.0;
            }
        } else if self.in_start_zone && dist_from_start > LAP_CROSS_DIST * 2.0 {
            // Exited the start zone — next re-entry will count
self.in_start_zone = false;
        }
        
        // ── Checkpoint Detection ──────────────────────────────
        // 3 checkpoints = 4 sectors for lap timing
        if self.checkpoints_passed.len() == 3 && self.started && !self.finished {
            let cp_idx = self.checkpoints_passed.iter().position(|&p| !p);
            if let Some(idx) = cp_idx {
                let z_threshold = self.start_z + 30.0 + (idx as f32 * 40.0);
                if self.car.z > z_threshold {
                    self.checkpoints_passed[idx] = true;
                    self.sector_times[idx] = self.elapsed - self.last_checkpoint_time;
                    self.last_checkpoint_time = self.elapsed;
                    eprintln!("[Race] Checkpoint {}: {:.1}s", idx + 1, self.sector_times[idx]);
                }
            }
        }
        
        // ── Update Camera ──────────────────────────────────────
        self.update_camera();
    }

    fn update_camera(&mut self) {
        let behind_x = self.car.x - self.camera.distance * self.car.heading.sin();
        let behind_z = self.car.z - self.camera.distance * self.car.heading.cos();
        let cam_y = self.car.y + self.camera.height;
        let look_x = self.car.x + 3.0 * self.car.heading.sin();
        let look_z = self.car.z + 3.0 * self.car.heading.cos();

        let view = Mat4::look_at(
            (behind_x, cam_y, behind_z),
            (look_x, self.car.y + 1.0, look_z),
            (0.0, 1.0, 0.0),
        );
        get_thread_renderer().borrow_mut().view_matrix = view.m;
    }

    // ─── Track Query Methods ───────────────────────────────────

    /// Find height of track surface at (x,z) using grid-accelerated triangle lookup.
    fn find_track_height(&self, x: f32, z: f32) -> Option<f32> {
        let tris = self.get_nearby_triangles(x, z);
        let mut best_dist = f32::MAX;
        let mut best_y = None;
        for &ti in &tris {
            if let Some(ref track) = self.track {
                if ti >= track.triangles.len() { continue; }
                let t = track.triangles[ti];
                let c = centroid(&track.vertices, t.0 as usize, t.1 as usize, t.2 as usize);
                let d = ((c.0 - x).powi(2) + (c.2 - z).powi(2)).sqrt();
                if d < best_dist {
                    best_dist = d;
                    best_y = Some(c.1);
                    if d < 2.0 { break; }
                }
            }
        }
        best_y
    }

    /// Find closest distance from (x,z) to any track triangle.
    fn closest_triangle_distance(&self, x: f32, z: f32) -> f32 {
        let tris = self.get_nearby_triangles(x, z);
        let mut best = f32::MAX;
        for &ti in &tris {
            if let Some(ref track) = self.track {
                if ti >= track.triangles.len() { continue; }
                let t = track.triangles[ti];
                let c = centroid(&track.vertices, t.0 as usize, t.1 as usize, t.2 as usize);
                let d = ((c.0 - x).powi(2) + (c.2 - z).powi(2)).sqrt();
                if d < best { best = d; }
                if best < 1.0 { return best; }
            }
        }
        best
    }

    /// Get nearby triangle indices using the spatial grid.
    fn get_nearby_triangles(&self, x: f32, z: f32) -> Vec<usize> {
        if let Some(ref grid) = self.track_grid {
            let cx = ((x - grid.min_x) / grid.cell_w) as isize;
            let cz = ((z - grid.min_z) / grid.cell_h) as isize;
            // Check 3x3 neighborhood
            let mut result = Vec::new();
            for dx in -1..=1 {
                for dz in -1..=1 {
                    let gx = cx + dx;
                    let gz = cz + dz;
                    if gx >= 0 && gx < grid.cells_x as isize && gz >= 0 && gz < grid.cells_z as isize {
                        result.extend_from_slice(&grid.cell_tris[gz as usize * grid.cells_x + gx as usize]);
                    }
                }
            }
            result
        } else if let Some(ref track) = self.track {
            // Fallback: return all triangles
            (0..track.triangles.len()).collect()
        } else {
            vec![]
        }
    }

    /// Project a world coordinate to screen pixel position.
    fn world_to_screen(&self, view_m: [f32; 16], proj_m: [f32; 16], pos: (f32, f32, f32)) -> (i32, i32) {
        let (px, py, _) = project_vertex_static(view_m, proj_m, pos);
        let half_w = 960.0 * 0.5;
        let half_h = 544.0 * 0.5;
        let sx = ((px + 1.0) * half_w) as i32;
        let sy = (544.0 - (py + 1.0) * half_h) as i32;
        (sx, sy)
    }

    // ─── Rendering ─────────────────────────────────────────────

    pub fn render(&self) {
        let r = get_thread_renderer();
        let aspect = 960.0 / 544.0;
        
        // Try OpenGL first if available, fall back to SDL2
        if let Some(ref mut glr) = get_opengl_renderer() {
            self.render_opengl(glr, aspect);
            return;
        }
        
        // SDL2 rendering path
        let proj = Mat4::perspective(self.camera.fov, aspect, 0.1, 500.0);
        
        // Get view matrix from renderer (set by update_camera)
        let view_m = {
            let b = r.borrow();
            b.view_matrix
        };
        
        // Use the NEW projection matrix, not the old one from renderer
        let proj_m = proj.m;
        
        // Get mutable reference and update renderer's proj_matrix
        let mut rb = r.borrow_mut();
        rb.proj_matrix = proj_m;
        
        // Now call render_sdl2 with correct matrices
        self.render_sdl2(&mut rb, view_m, proj_m);
    }
    
    fn render_opengl(&self, glr: &mut crate::engine::graphics::OpenGLRenderer, aspect: f32) {
        let proj = perspective_matrix(self.camera.fov, aspect, 0.1, 500.0);
        let eye = (
            self.car.x - self.car.heading.sin() * self.camera.distance,
            self.car.y + self.camera.height,
            self.car.z - self.car.heading.cos() * self.camera.distance,
        );
        let view = look_at_matrix(eye, (self.car.x, self.car.y, self.car.z), (0.0, 1.0, 0.0));
        
        glr.set_projection(proj);
        glr.set_view(view);
        glr.clear();
        
        // Draw track - use texture if available with UVs, otherwise solid color
        if let Some(ref track) = self.track {
            let use_texture = track.has_uvs && glr.has_texture("course_tex");
            
            if use_texture {
                // Textured rendering with UVs: [x,y,z,u,v] per vertex
                let mut vertices = Vec::new();
                let mut indices = Vec::new();
                
                for &tri in &track.triangles {
                    let i0 = tri.0 as usize;
                    let i1 = tri.1 as usize;
                    let i2 = tri.2 as usize;
                    if i0 >= track.vertices.len() || i1 >= track.vertices.len() || i2 >= track.vertices.len() { continue; }
                    
                    let base = vertices.len() as u32 / 5;
                    // Vertex 0: position + UV
                    vertices.push(track.vertices[i0].0);
                    vertices.push(track.vertices[i0].1);
                    vertices.push(track.vertices[i0].2);
                    vertices.push(track.uvs.get(i0).map(|u| u.0).unwrap_or(0.0));
                    vertices.push(track.uvs.get(i0).map(|u| u.1).unwrap_or(0.0));
                    // Vertex 1: position + UV
                    vertices.push(track.vertices[i1].0);
                    vertices.push(track.vertices[i1].1);
                    vertices.push(track.vertices[i1].2);
                    vertices.push(track.uvs.get(i1).map(|u| u.0).unwrap_or(0.0));
                    vertices.push(track.uvs.get(i1).map(|u| u.1).unwrap_or(0.0));
                    // Vertex 2: position + UV
                    vertices.push(track.vertices[i2].0);
                    vertices.push(track.vertices[i2].1);
                    vertices.push(track.vertices[i2].2);
                    vertices.push(track.uvs.get(i2).map(|u| u.0).unwrap_or(0.0));
                    vertices.push(track.uvs.get(i2).map(|u| u.1).unwrap_or(0.0));
                    
                    indices.push(base);
                    indices.push(base + 1);
                    indices.push(base + 2);
                }
                
                if !vertices.is_empty() {
                    glr.draw_mesh_textured(&vertices, &indices, "course_tex");
                }
            } else {
                // Solid color rendering (no texture)
                let color = if self.car.on_track { (0.1, 0.6, 0.1) } else { (0.3, 0.15, 0.05) };
                let mut vertices = Vec::new();
                let mut indices = Vec::new();
                
                for &tri in &track.triangles {
                    let i0 = tri.0 as usize;
                    let i1 = tri.1 as usize;
                    let i2 = tri.2 as usize;
                    if i0 >= track.vertices.len() || i1 >= track.vertices.len() || i2 >= track.vertices.len() { continue; }
                    
                    let base = vertices.len() as u32;
                    vertices.push(track.vertices[i0].0);
                    vertices.push(track.vertices[i0].1);
                    vertices.push(track.vertices[i0].2);
                    vertices.push(track.vertices[i1].0);
                    vertices.push(track.vertices[i1].1);
                    vertices.push(track.vertices[i1].2);
                    vertices.push(track.vertices[i2].0);
                    vertices.push(track.vertices[i2].1);
                    vertices.push(track.vertices[i2].2);
                    indices.push(base);
                    indices.push(base + 1);
                    indices.push(base + 2);
                }
                
                if !vertices.is_empty() {
                    glr.draw_mesh(&vertices, &indices, color);
                }
            }
        }
        
        // Draw car - use texture if available with UVs, otherwise solid color
        if let Some(ref car) = self.car_model {
            let use_texture = car.has_uvs && glr.has_texture("car_tex");
            
            if use_texture {
                // Textured rendering with UVs: [x,y,z,u,v] per vertex
                let mut vertices = Vec::new();
                let mut indices = Vec::new();
                
                for &(a, b, c) in &car.triangles {
                    let ia = a as usize;
                    let ib = b as usize;
                    let ic = c as usize;
                    if ia >= car.vertices.len() || ib >= car.vertices.len() || ic >= car.vertices.len() { continue; }
                    
                    let rotate_vertex = |v: (f32,f32,f32)| -> (f32, f32, f32) {
                        let rx = v.0 * self.car.heading.cos() - v.2 * self.car.heading.sin();
                        let rz = v.0 * self.car.heading.sin() + v.2 * self.car.heading.cos();
                        (self.car.x + rx, self.car.y + 0.5 + v.1, self.car.z + rz)
                    };
                    
                    let v0 = rotate_vertex(car.vertices[ia]);
                    let v1 = rotate_vertex(car.vertices[ib]);
                    let v2 = rotate_vertex(car.vertices[ic]);
                    
                    let base = vertices.len() as u32 / 5;
                    // Vertex 0: position + UV
                    vertices.push(v0.0); vertices.push(v0.1); vertices.push(v0.2);
                    vertices.push(car.uvs.get(ia).map(|u| u.0).unwrap_or(0.0));
                    vertices.push(car.uvs.get(ia).map(|u| u.1).unwrap_or(0.0));
                    // Vertex 1: position + UV
                    vertices.push(v1.0); vertices.push(v1.1); vertices.push(v1.2);
                    vertices.push(car.uvs.get(ib).map(|u| u.0).unwrap_or(0.0));
                    vertices.push(car.uvs.get(ib).map(|u| u.1).unwrap_or(0.0));
                    // Vertex 2: position + UV
                    vertices.push(v2.0); vertices.push(v2.1); vertices.push(v2.2);
                    vertices.push(car.uvs.get(ic).map(|u| u.0).unwrap_or(0.0));
                    vertices.push(car.uvs.get(ic).map(|u| u.1).unwrap_or(0.0));
                    
                    indices.push(base);
                    indices.push(base + 1);
                    indices.push(base + 2);
                }
                
                if !vertices.is_empty() {
                    glr.draw_mesh_textured(&vertices, &indices, "car_tex");
                }
            } else {
                // Solid color rendering (no texture)
                let mut vertices = Vec::new();
                let mut indices = Vec::new();
                
                for &(a, b, c) in &car.triangles {
                    let ia = a as usize;
                    let ib = b as usize;
                    let ic = c as usize;
                    if ia >= car.vertices.len() || ib >= car.vertices.len() || ic >= car.vertices.len() { continue; }
                    
                    let rotate_vertex = |v: (f32,f32,f32)| -> [f32; 3] {
                        let rx = v.0 * self.car.heading.cos() - v.2 * self.car.heading.sin();
                        let rz = v.0 * self.car.heading.sin() + v.2 * self.car.heading.cos();
                        [self.car.x + rx, self.car.y + 0.5 + v.1, self.car.z + rz]
                    };
                    
                    let v0 = rotate_vertex(car.vertices[ia]);
                    let v1 = rotate_vertex(car.vertices[ib]);
                    let v2 = rotate_vertex(car.vertices[ic]);
                    
                    let base = vertices.len() as u32;
                    vertices.extend_from_slice(&v0);
                    vertices.extend_from_slice(&v1);
                    vertices.extend_from_slice(&v2);
                    indices.push(base);
                    indices.push(base + 1);
                    indices.push(base + 2);
                }
                
                if !vertices.is_empty() {
                    glr.draw_mesh(&vertices, &indices, (0.7, 0.2, 0.2));
                }
            }
        }
        
        // HUD via SDL2
        let rb = get_thread_renderer();
        let mut rbb = rb.borrow_mut();
        let speed_kmh = (self.car.speed * 3.6) as i32;
        let lap_str = if self.finished { format!("FINISHED! {:.1}s", self.elapsed) }
                      else { format!("LAP {}/{}", self.current_lap, self.total_laps) };
        rbb.draw_text(10, 10, &format!("{} km/h", speed_kmh), 255, 255, 255, 1.0);
        rbb.draw_text(10, 30, &lap_str, 255, 255, 255, 1.0);
    }
    
    fn render_sdl2(&self, rb: &mut crate::engine::graphics::GraphicsRenderer, view_m: [f32; 16], proj_m: [f32; 16]) {
        // Dark blue background (sky)
        rb.fill_rect(0, 0, 960, 544, 10, 10, 40);

        // ── Cache course texture once, draw as tint strip ──
        if let Some(ref tex) = self.course_texture {
            if !rb.has_texture("course_tex") {
                rb.cache_texture("course_tex", tex);
            }
            // Draw as a thin left-side strip, not full-screen
            rb.draw_texture("course_tex", 0, 0, 4, 544);
        }

        // ── 3D Track Rendering (solid fill) ──────────────────────
        // Use depth-tested projection: skip triangles behind camera or outside frustum
        let track_color = if self.car.on_track { Color::RGB(30, 160, 30) } else { Color::RGB(100, 60, 20) };
        let road_color = Color::RGB(80, 80, 80);
        
        if let Some(ref track) = self.track {
            for &tri in &track.triangles {
                let i0 = tri.0 as usize;
                let i1 = tri.1 as usize;
                let i2 = tri.2 as usize;
                if i0 >= track.vertices.len() || i1 >= track.vertices.len() || i2 >= track.vertices.len() { continue; }
                
                let v0 = project_vertex_safe(view_m, proj_m, track.vertices[i0]);
                let v1 = project_vertex_safe(view_m, proj_m, track.vertices[i1]);
                let v2 = project_vertex_safe(view_m, proj_m, track.vertices[i2]);
                
                // Only draw if all 3 vertices are in front of camera
                if let (Some(a), Some(b), Some(c)) = (v0, v1, v2) {
                    // Back-face culling: skip triangles facing away from camera
                    let area = (b.0 - a.0) * (c.1 - a.1) - (b.1 - a.1) * (c.0 - a.0);
                    if area <= 0.0 { continue; }
                    
                    // Alternate color based on height to show terrain variation
                    let avg_y = (track.vertices[i0].1 + track.vertices[i1].1 + track.vertices[i2].1) / 3.0;
                    let color = if avg_y < 2.0 { road_color } else { track_color };
                    fill_triangle(rb, (a.0, a.1), (b.0, b.1), (c.0, c.1), color);
                }
            }
        }

        // ── Draw start/finish line ───────────────────────────────
        let sp0 = project_vertex_safe(view_m, proj_m, (self.start_x, 0.0, self.start_z));
        let sp1 = project_vertex_safe(view_m, proj_m, (self.start_x, 4.0, self.start_z));
        if let (Some(a), Some(b)) = (sp0, sp1) {
            let half_w = 960.0 * 0.5;
            let half_h = 544.0 * 0.5;
            let ax = ((a.0 + 1.0) * half_w) as i32;
            let ay = (544.0 - (a.1 + 1.0) * half_h) as i32;
            let bx = ((b.0 + 1.0) * half_w) as i32;
            let by = (544.0 - (b.1 + 1.0) * half_h) as i32;
            rb.draw_screen_line(ax as f32, ay as f32, bx as f32, by as f32, 255, 255, 80);
        }

        // ── 3D Car Rendering (solid fill) ────────────────────────
        let car_color = Color::RGB(180, 50, 50);
        if let Some(ref car) = self.car_model {
            for &(a, b, c) in &car.triangles {
                let ia = a as usize;
                let ib = b as usize;
                let ic = c as usize;
                if ia >= car.vertices.len() || ib >= car.vertices.len() || ic >= car.vertices.len() { continue; }
                
                let rotate_vertex = |v: (f32,f32,f32)| {
                    let rx = v.0 * self.car.heading.cos() - v.2 * self.car.heading.sin();
                    let rz = v.0 * self.car.heading.sin() + v.2 * self.car.heading.cos();
                    (self.car.x + rx, self.car.y + 0.5 + v.1, self.car.z + rz)
                };
                
                let v0 = project_vertex_safe(view_m, proj_m, rotate_vertex(car.vertices[ia]));
                let v1 = project_vertex_safe(view_m, proj_m, rotate_vertex(car.vertices[ib]));
                let v2 = project_vertex_safe(view_m, proj_m, rotate_vertex(car.vertices[ic]));
                
                if let (Some(a), Some(b), Some(c)) = (v0, v1, v2) {
                    // Back-face culling
                    let area = (b.0 - a.0) * (c.1 - a.1) - (b.1 - a.1) * (c.0 - a.0);
                    if area <= 0.0 { continue; }
                    
                    fill_triangle(rb, (a.0, a.1), (b.0, b.1), (c.0, c.1), car_color);
                }
            }
        }

        // ── HUD ────────────────────────────────────────────────
        let speed_kmh = (self.car.speed * 3.6) as i32;
        let lap_str = if self.finished { format!("FINISHED! {:.1}s", self.elapsed) }
                      else { format!("LAP {}/{}", self.current_lap, self.total_laps) };
        let time_str = format!("{:3.1}s", self.elapsed);
        let track_str = if self.car.on_track { "ON TRACK" } else { "OFF TRACK!" };
        let best_str = if self.best_lap < 999.0 { format!("Best: {:.1}s", self.best_lap) } else { "Best: --.-s".to_string() };
        
        // Speedometer (large, green)
        rb.draw_text(10, 10, &format!("{} km/h", speed_kmh), 255, 200, 80, 1.4);
        
        // Current time
        rb.draw_text(10, 45, &time_str, 200, 200, 200, 1.0);
        
        // Best lap
        rb.draw_text(10, 70, &best_str, 100, 200, 255, 0.9);
        
        // Lap counter
        rb.draw_text(10, 95, &lap_str, 200, 200, 200, 0.9);
        
        // Track status (red when off track)
        let track_color = if self.car.on_track { (80, 200, 80) } else { (255, 80, 80) };
        rb.draw_text(10, 120, track_str, track_color.0, track_color.1, track_color.2, 0.8);
    }

    fn project_vertex(&self, v: (f32, f32, f32)) -> (f32, f32, f32) {
        let r = get_thread_renderer();
        let b = r.borrow();
        let view_m = b.view_matrix;
        let proj_m = b.proj_matrix;
        drop(b);
        let v4 = [v.0, v.1, v.2, 1.0];
        let mut t = [0.0; 4];
        for i in 0..4 {
            for j in 0..4 { t[i] += view_m[i*4 + j] * v4[j]; }
        }
        let mut p = [0.0; 4];
        for i in 0..4 {
            for j in 0..4 { p[i] += proj_m[i*4 + j] * t[j]; }
        }
        if p[3].abs() > 0.001 { (p[0]/p[3], p[1]/p[3], p[2]/p[3]) } else { v }
    }
}

// ─── Helpers ──────────────────────────────────────────────────

fn centroid(verts: &[(f32, f32, f32)], i0: usize, i1: usize, i2: usize) -> (f32, f32, f32) {
    let v0 = verts.get(i0).copied().unwrap_or((0.0,0.0,0.0));
    let v1 = verts.get(i1).copied().unwrap_or((0.0,0.0,0.0));
    let v2 = verts.get(i2).copied().unwrap_or((0.0,0.0,0.0));
    ((v0.0 + v1.0 + v2.0) / 3.0, (v0.1 + v1.1 + v2.1) / 3.0, (v0.2 + v1.2 + v2.2) / 3.0)
}

/// Build spatial grid for fast triangle lookups.
fn build_track_grid(track: &TrackState) -> Option<TrackGrid> {
    if track.vertices.is_empty() || track.triangles.is_empty() { return None; }
    let mut min_x = f32::MAX; let mut max_x = f32::MIN;
    let mut min_z = f32::MAX; let mut max_z = f32::MIN;
    for &(x, _, z) in &track.vertices {
        if x < min_x { min_x = x; }
        if x > max_x { max_x = x; }
        if z < min_z { min_z = z; }
        if z > max_z { max_z = z; }
    }
    let cells_x = 20;
    let cells_z = 20;
    let cell_w = (max_x - min_x).max(1.0) / cells_x as f32;
    let cell_h = (max_z - min_z).max(1.0) / cells_z as f32;
    let mut cell_tris = vec![vec![]; cells_x * cells_z];
    for (ti, &tri) in track.triangles.iter().enumerate() {
        let c = centroid(&track.vertices, tri.0 as usize, tri.1 as usize, tri.2 as usize);
        let gx = ((c.0 - min_x) / cell_w).max(0.0).min((cells_x - 1) as f32) as usize;
        let gz = ((c.2 - min_z) / cell_h).max(0.0).min((cells_z - 1) as f32) as usize;
        cell_tris[gz * cells_x + gx].push(ti);
    }
    Some(TrackGrid { min_x, max_x, min_z, max_z, cells_x, cells_z, cell_w, cell_h, cell_tris })
}

// ─── 3D projection (uses provided matrices, no renderer borrow) ──

fn project_vertex_static(view_m: [f32; 16], proj_m: [f32; 16], v: (f32, f32, f32)) -> (f32, f32, f32) {
    let v4 = [v.0, v.1, v.2, 1.0];
    let mut t = [0.0; 4];
    for i in 0..4 {
        for j in 0..4 { t[i] += view_m[i*4 + j] * v4[j]; }
    }
    let mut p = [0.0; 4];
    for i in 0..4 {
        for j in 0..4 { p[i] += proj_m[i*4 + j] * t[j]; }
    }
    if p[3].abs() > 0.001 { (p[0]/p[3], p[1]/p[3], p[2]/p[3]) } else { v }
}

/// Project vertex and return None if behind camera or outside depth range.
/// This prevents triangles behind the camera from filling the entire screen.
fn project_vertex_safe(view_m: [f32; 16], proj_m: [f32; 16], v: (f32, f32, f32)) -> Option<(f32, f32, f32)> {
    let v4 = [v.0, v.1, v.2, 1.0];
    let mut t = [0.0; 4];
    for i in 0..4 {
        for j in 0..4 { t[i] += view_m[i*4 + j] * v4[j]; }
    }
    let mut p = [0.0; 4];
    for i in 0..4 {
        for j in 0..4 { p[i] += proj_m[i*4 + j] * t[j]; }
    }
    // Behind camera or too close to camera
    if p[3] < 0.001 { return None; }
    let ndc_x = p[0] / p[3];
    let ndc_y = p[1] / p[3];
    let ndc_z = p[2] / p[3];
    // Outside depth range
    if ndc_z < -1.0 || ndc_z > 1.0 { return None; }
    Some((ndc_x, ndc_y, ndc_z))
}

// ─── Simple triangle fill via scanline + edge outlines ───────────

/// Draw triangle with scanline fill. Includes screen-space clipping.
fn fill_triangle(renderer: &mut crate::engine::graphics::GraphicsRenderer,
                   a: (f32, f32), b: (f32, f32), c: (f32, f32),
                   color: sdl2::pixels::Color) {
    const SCREEN_W: i32 = 960;
    const SCREEN_H: i32 = 544;
    
    // Skip degenerate or NaN triangles
    if a.0.is_nan() || b.0.is_nan() || c.0.is_nan() { return; }
    if a.1.is_nan() || b.1.is_nan() || c.1.is_nan() { return; }
    
    // Skip triangles outside NDC range (clipped by frustum)
    let in_range = |v: (f32, f32)| v.0 >= -2.0 && v.0 <= 2.0 && v.1 >= -2.0 && v.1 <= 2.0;
    if !in_range(a) && !in_range(b) && !in_range(c) { return; }
    
    let half_w = SCREEN_W as f32 * 0.5;
    let half_h = SCREEN_H as f32 * 0.5;

    let to_screen = |p: (f32, f32)| -> (i32, i32) {
        (((p.0 + 1.0) * half_w) as i32, (SCREEN_H as f32 - (p.1 + 1.0) * half_h) as i32)
    };

    let p0 = to_screen(a);
    let p1 = to_screen(b);
    let p2 = to_screen(c);

    // Sort vertices by y (top to bottom)
    let mut v = [p0, p1, p2];
    v.sort_by_key(|p| p.1);
    let (v0, v1, v2) = (v[0], v[1], v[2]);

    let total_height = v2.1 - v0.1;
    if total_height == 0 { return; } // degenerate (line)
    
    // Clamp y range to screen bounds
    let y_start = v0.1.max(0);
    let y_mid = v1.1.min(SCREEN_H - 1);
    let y_end = v2.1.min(SCREEN_H - 1);

    renderer.canvas_set_color(color);

    // Fill top half (v0→v1)
    for y in y_start..=y_mid.min(v1.1) {
        if y < 0 || y >= SCREEN_H { continue; }
        let t = (y - v0.1) as f32 / total_height as f32;
        let t_seg = if v1.1 != v0.1 { (y - v0.1) as f32 / (v1.1 - v0.1) as f32 } else { 0.0 };
        let mut xa = v0.0 + (t * (v2.0 - v0.0) as f32) as i32;
        let mut xb = v0.0 + (t_seg * (v1.0 - v0.0) as f32) as i32;
        if xa > xb { std::mem::swap(&mut xa, &mut xb); }
        // Clamp x to screen bounds
        xa = xa.max(0).min(SCREEN_W - 1);
        xb = xb.max(0).min(SCREEN_W - 1);
        if xb > xa {
            let _ = renderer.canvas_draw_line(xa, y, xb, y);
        }
    }

    // Fill bottom half (v1→v2)
    for y in (v1.1.max(0))..=y_end {
        if y < 0 || y >= SCREEN_H { continue; }
        let t = (y - v0.1) as f32 / total_height as f32;
        let t_seg = if v2.1 != v1.1 { (y - v1.1) as f32 / (v2.1 - v1.1) as f32 } else { 0.0 };
        let mut xa = v0.0 + (t * (v2.0 - v0.0) as f32) as i32;
        let mut xb = v1.0 + (t_seg * (v2.0 - v1.0) as f32) as i32;
        if xa > xb { std::mem::swap(&mut xa, &mut xb); }
        // Clamp x to screen bounds
        xa = xa.max(0).min(SCREEN_W - 1);
        xb = xb.max(0).min(SCREEN_W - 1);
        if xb > xa {
            let _ = renderer.canvas_draw_line(xa, y, xb, y);
        }
    }
}

/// Draw triangle outline only (wireframe) for debugging
fn draw_triangle_outline(renderer: &mut crate::engine::graphics::GraphicsRenderer,
                   a: (f32, f32), b: (f32, f32), c: (f32, f32),
                   color: sdl2::pixels::Color) {
    let half_w = 960.0 * 0.5;
    let half_h = 544.0 * 0.5;
    
    let to_screen = |p: (f32, f32)| -> (i32, i32) {
        (((p.0 + 1.0) * half_w) as i32, (544.0 - (p.1 + 1.0) * half_h) as i32)
    };
    
    if a.0.is_nan() || b.0.is_nan() || c.0.is_nan() { return; }
    
    let p0 = to_screen(a);
    let p1 = to_screen(b);
    let p2 = to_screen(c);
    
    // Skip if all points are way off-screen
    let on_screen = |p: (i32, i32)| p.0 >= -100 && p.0 < 1060 && p.1 >= -100 && p.1 < 644;
    if !on_screen(p0) && !on_screen(p1) && !on_screen(p2) { return; }
    
    renderer.canvas_set_color(color);
    renderer.canvas_draw_line(p0.0, p0.1, p1.0, p1.1);
    renderer.canvas_draw_line(p1.0, p1.1, p2.0, p2.1);
    renderer.canvas_draw_line(p2.0, p2.1, p0.0, p0.1);
}

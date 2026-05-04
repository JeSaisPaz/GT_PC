use std::path::PathBuf;
use std::time::Instant;

use winit::event::{Event, WindowEvent, DeviceEvent, ElementState, MouseButton};
use winit::keyboard::{KeyCode, PhysicalKey};
use winit::dpi::LogicalSize;
use winit::window::{WindowAttributes, Window};

mod renderer;
mod loader;
mod camera;
mod texture;

use loader::{TrackModel, load_track, load_track_texture, load_course_metadata, load_embedded_texture};
use camera::Freecam;
use renderer::Renderer;

fn get_assets_path() -> PathBuf {
    let cwd_path = PathBuf::from("assets").join("game");
    if cwd_path.join("crs").exists() {
        return cwd_path;
    }
    if let Some(exe_dir) = std::env::current_exe().ok().and_then(|p| p.parent().map(|p| p.to_path_buf())) {
        let exe_path = exe_dir.join("assets").join("game");
        if exe_path.join("crs").exists() {
            return exe_path;
        }
    }
    cwd_path
}

struct AppState {
    window: Option<&'static Window>,
    renderer: Option<Renderer>,
    track: Option<TrackModel>,
    freecam: Freecam,
    last_frame: Instant,
    mouse_locked: bool,
}

impl AppState {
    fn new() -> Self {
        AppState {
            window: None,
            renderer: None,
            track: None,
            freecam: Freecam::new(),
            last_frame: Instant::now(),
            mouse_locked: false,
        }
    }
}

fn main() {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("Failed to create tokio runtime");

    runtime.block_on(async {
        run_app().await;
    });
}

async fn run_app() {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info"))
        .init();

    log::info!("GT PSP Freecam Tool");
    log::info!("Assets path: {:?}", get_assets_path());

    let event_loop = winit::event_loop::EventLoop::new().expect("Failed to create event loop");

    let mut state = AppState::new();
    let mut initialized = false;

    #[allow(deprecated)]
    event_loop.run(move |event, elwt| {
        if !initialized {
            initialized = true;

            let window_attributes = WindowAttributes::new()
                .with_title("GT PSP Freecam")
                .with_inner_size(LogicalSize::new(960.0, 544.0))
                .with_resizable(true);

            let window = elwt.create_window(window_attributes).unwrap();
            let window: &'static Window = Box::leak(Box::new(window));

            let assets_path = get_assets_path();
            let track_path = assets_path.join("crs").join("c114x");
            let texture_path = assets_path.join("crs").join("race.txs");

            match load_track(&track_path, &texture_path) {
                Ok(track) => {
                    log::info!("Loaded track: {} vertices, {} triangles",
                        track.vertices.len(), track.triangles.len());
                    state.track = Some(track);
                }
                Err(e) => {
                    log::error!("Failed to load track: {}", e);
                }
            }

            // Set camera spawn from .ad metadata
            if let Some((x, z)) = load_course_metadata(114) {
                state.freecam.position = glam::Vec3::new(x, 5.0, z);
                log::info!("Spawn position: ({:.1}, {:.1})", x, z);
            }

            let mut renderer = pollster::block_on(Renderer::new(window));

            // Load and apply track texture (try external TXS3 first, then embedded)
            let tex_path = assets_path.join("crs").join("race.txs");
            let tex = match load_track_texture(&tex_path) {
                Ok(t) => Some(t),
                Err(e) => {
                    log::info!("No external texture: {}, trying embedded...", e);
                    std::fs::read(&track_path).ok()
                        .and_then(|d| load_embedded_texture(&d))
                }
            };

            if let Some(t) = tex {
                log::info!("Loaded texture: {}x{} ({} pixels)", t.width, t.height, t.data.len());
                renderer.set_texture(&t);
            } else {
                log::warn!("No texture loaded, using fallback shader");
            }

            state.renderer = Some(renderer);
            state.window = Some(window);

            return;
        }

        match event {
            Event::WindowEvent { event, .. } => {
                match event {
                    WindowEvent::CloseRequested => {
                        elwt.exit();
                    }
                    WindowEvent::Resized(size) => {
                        if let Some(renderer) = &mut state.renderer {
                            renderer.resize(size.width, size.height);
                        }
                    }
                    WindowEvent::KeyboardInput { event, .. } => {
                        let pressed = event.state == ElementState::Pressed;

                        if let PhysicalKey::Code(keycode) = event.physical_key {
                            if pressed && keycode == KeyCode::Escape {
                                state.mouse_locked = false;
                                if let Some(window) = state.window {
                                    let _ = window.set_cursor_grab(winit::window::CursorGrabMode::None);
                                    window.set_cursor_visible(true);
                                }
                                return;
                            }

                            state.freecam.handle_key(keycode, pressed);
                        }
                    }
                    WindowEvent::MouseInput { state: btn_state, button, .. } => {
                        if button == MouseButton::Left
                            && btn_state == ElementState::Pressed
                        {
                            state.mouse_locked = true;
                            if let Some(window) = state.window {
                                let _ = window.set_cursor_grab(winit::window::CursorGrabMode::Confined);
                                window.set_cursor_visible(false);
                            }
                        }
                    }
                    _ => {}
                }
            }
            Event::DeviceEvent { event, .. } => {
                if let DeviceEvent::MouseMotion { delta: (dx, dy) } = event {
                    if state.mouse_locked {
                        state.freecam.handle_mouse_move(dx as f32, dy as f32);
                    }
                }
            }
            Event::AboutToWait => {
                let dt = state.last_frame.elapsed().as_secs_f32().min(0.05);
                state.last_frame = Instant::now();

                state.freecam.update(dt);

                if let Some(renderer) = &mut state.renderer {
                    if let Some(track) = &state.track {
                        if let Some(window) = state.window {
                            let size = window.inner_size();
                            renderer.render(track, &state.freecam, size.width, size.height);
                        }
                    }
                }

                if let Some(window) = state.window {
                    window.request_redraw();
                }
            }
            _ => {}
        }
    }).expect("Failed to run event loop");
}

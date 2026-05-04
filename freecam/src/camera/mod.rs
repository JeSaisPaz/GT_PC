use glam::{Vec3, Mat4};
use winit::keyboard::KeyCode;

pub struct Freecam {
    pub position: Vec3,
    pub yaw: f32,
    pub pitch: f32,
    pub move_speed: f32,
    pub look_speed: f32,
    keys: FreecamKeys,
}

struct FreecamKeys {
    forward: bool,
    backward: bool,
    left: bool,
    right: bool,
    up: bool,
    down: bool,
    fast: bool,
}

impl FreecamKeys {
    fn new() -> Self {
        FreecamKeys {
            forward: false,
            backward: false,
            left: false,
            right: false,
            up: false,
            down: false,
            fast: false,
        }
    }
}

impl Freecam {
    pub fn new() -> Self {
        Freecam {
            position: Vec3::new(0.0, 10.0, 50.0),
            yaw: 0.0,
            pitch: -0.2,
            move_speed: 50.0,
            look_speed: 0.003,
            keys: FreecamKeys::new(),
        }
    }

    pub fn update(&mut self, dt: f32) {
        let speed = if self.keys.fast { self.move_speed * 3.0 } else { self.move_speed };

        let forward = Vec3::new(
            self.yaw.sin() * self.pitch.cos(),
            self.pitch.sin(),
            self.yaw.cos() * self.pitch.cos(),
        );
        let right = Vec3::new(self.yaw.cos(), 0.0, -self.yaw.sin());

        if self.keys.forward {
            self.position += forward * speed * dt;
        }
        if self.keys.backward {
            self.position -= forward * speed * dt;
        }
        if self.keys.left {
            self.position -= right * speed * dt;
        }
        if self.keys.right {
            self.position += right * speed * dt;
        }
        if self.keys.up {
            self.position.y += speed * dt;
        }
        if self.keys.down {
            self.position.y -= speed * dt;
        }
    }

    pub fn get_view_matrix(&self) -> Mat4 {
        let target = self.position + Vec3::new(
            self.yaw.sin() * self.pitch.cos(),
            self.pitch.sin(),
            self.yaw.cos() * self.pitch.cos(),
        );
        Mat4::look_at_rh(self.position, target, Vec3::new(0.0, 1.0, 0.0))
    }

    pub fn get_view_projection_matrix(&self, aspect: f32) -> Mat4 {
        let proj = Mat4::perspective_rh(glm::radians(75.0), aspect, 0.1, 10000.0);
        proj * self.get_view_matrix()
    }

    pub fn handle_key(&mut self, key: KeyCode, pressed: bool) {
        match key {
            KeyCode::KeyW | KeyCode::ArrowUp => self.keys.forward = pressed,
            KeyCode::KeyS | KeyCode::ArrowDown => self.keys.backward = pressed,
            KeyCode::KeyA | KeyCode::ArrowLeft => self.keys.left = pressed,
            KeyCode::KeyD | KeyCode::ArrowRight => self.keys.right = pressed,
            KeyCode::Space => self.keys.up = pressed,
            KeyCode::ShiftLeft | KeyCode::ShiftRight => self.keys.down = pressed,
            KeyCode::ControlLeft | KeyCode::ControlRight => self.keys.fast = pressed,
            _ => {}
        }
    }

    pub fn handle_mouse_move(&mut self, dx: f32, dy: f32) {
        self.yaw += dx * self.look_speed;
        self.pitch -= dy * self.look_speed;
        self.pitch = self.pitch.clamp(-1.5, 1.5);
    }
}

mod glm {
    pub fn radians(deg: f32) -> f32 {
        deg * std::f32::consts::PI / 180.0
    }
}

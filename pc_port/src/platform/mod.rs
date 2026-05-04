// Stub module for platform
pub mod window {
    pub fn create_window() -> Result<Window, String> {
        Ok(Window)
    }
    
    pub struct Window;
    impl Window {
        pub fn set_title(&mut self, _title: &str) {}
    }
}

pub mod render {}

pub mod input {
    pub fn poll_input() -> u32 { 0 }
    pub fn is_button_pressed(btn: &str) -> bool { false }
}

pub mod audio {
    pub fn init_audio() -> Result<(), String> { Ok(()) }
    pub fn play_sound(_id: u32) {}
    pub fn stop_sound(_id: u32) {}
}

pub mod texture {
    pub fn load_texture(path: &str) -> Result<Texture, String> {
        Ok(Texture)
    }
    pub struct Texture;
    impl Texture {
        pub fn width(&self) -> i32 { 0 }
        pub fn height(&self) -> i32 { 0 }
    }
}
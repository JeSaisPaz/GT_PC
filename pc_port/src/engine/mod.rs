pub mod audio;
pub mod main_loop;
pub mod model;
pub mod race;
pub mod specdb;
pub mod sprite;
pub mod gtengine;
pub mod menu;
pub mod pdistd;
pub mod pdiext;
pub mod pdiapp;
pub mod ui;
pub mod test_tools;

#[cfg(windows)]
pub mod graphics;

/// Default directory containing extracted GT.VOL contents.
/// Set via `init_assets_root()` at startup before any engine module loads assets.
pub const DEFAULT_GT_VOL_PATH: &str = "assets";

use std::sync::OnceLock;
static ASSETS_ROOT: OnceLock<String> = OnceLock::new();

pub fn init_assets_root(path: &str) {
    let _ = ASSETS_ROOT.set(path.to_string());
}

pub fn assets_root() -> String {
    ASSETS_ROOT.get().cloned().unwrap_or_else(|| DEFAULT_GT_VOL_PATH.to_string())
}

/// DEPRECATED: Use `assets_root()` instead. Kept for compatibility.
pub const GT_VOL_PATH: &str = "assets";

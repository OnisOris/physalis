#[cfg(target_arch = "wasm32")]
mod ui_icons;

#[cfg(target_arch = "wasm32")]
mod wasm_app;

#[cfg(target_arch = "wasm32")]
pub use wasm_app::start;

#[cfg(not(target_arch = "wasm32"))]
pub fn start() {}

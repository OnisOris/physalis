use cad_geom::TriMesh;
use thiserror::Error;

/// Placeholder type for non-wasm targets.
pub struct Canvas;

#[derive(Clone, Copy, Debug)]
pub struct OverlayLine {
    pub a: [f32; 3],
    pub b: [f32; 3],
    pub color: [f32; 3],
}

#[derive(Debug, Error)]
pub enum RenderError {
    #[error("cad-render is only supported for wasm32 in this MVP")]
    Unsupported,
}

pub struct Renderer;

impl Renderer {
    pub async fn new(_canvas: Canvas) -> Result<Self, RenderError> {
        Err(RenderError::Unsupported)
    }

    pub fn attach_default_controls(&mut self, _canvas: &Canvas) {}

    pub fn resize(&mut self, _width: u32, _height: u32) {}

    pub fn set_mesh(&mut self, _mesh: TriMesh) {}

    pub fn set_plane_visibility(&mut self, _xy: bool, _yz: bool, _zx: bool) {}

    pub fn set_overlay_lines(&mut self, _lines: Vec<OverlayLine>) {}

    pub fn clear_overlay_lines(&mut self) {}

    pub fn camera_eye_target(&self) -> ([f32; 3], [f32; 3]) {
        ([0.0, 0.0, 0.0], [0.0, 0.0, 0.0])
    }

    pub fn camera_rotation(&self) -> [f32; 4] {
        [0.0, 0.0, 0.0, 1.0]
    }

    pub fn set_camera_rotation(&mut self, _rotation: [f32; 4]) {}

    pub fn screen_ray(
        &self,
        _cursor_x: f32,
        _cursor_y: f32,
        _viewport_width: f32,
        _viewport_height: f32,
    ) -> ([f32; 3], [f32; 3]) {
        ([0.0, 0.0, 0.0], [0.0, 0.0, 0.0])
    }

    pub fn render(&mut self) {}
}

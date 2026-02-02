use cad_geom::TriMesh;
use thiserror::Error;

/// Placeholder type for non-wasm targets.
pub struct Canvas;

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

    pub fn render(&mut self) {}
}

use cad_geom::TriMesh;
use glam::{Mat4, Vec3};
use std::cell::RefCell;
use std::rc::Rc;
use thiserror::Error;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use web_sys::{HtmlCanvasElement, MouseEvent, WheelEvent};

use wgpu::util::DeviceExt;

pub type Canvas = HtmlCanvasElement;

#[derive(Debug, Error)]
pub enum RenderError {
    #[error("surface creation failed: {0}")]
    Surface(#[from] wgpu::CreateSurfaceError),
    #[error("adapter request failed: {0}")]
    Adapter(#[from] wgpu::RequestAdapterError),
    #[error("device request failed: {0}")]
    Device(#[from] wgpu::RequestDeviceError),
    #[error("surface unsupported by adapter")]
    SurfaceUnsupported,
}

pub struct Renderer {
    state: Rc<RefCell<RendererState>>,
    _closures: Vec<Closure<dyn FnMut(web_sys::Event)>>,
}

impl Renderer {
    pub async fn new(canvas: HtmlCanvasElement) -> Result<Self, RenderError> {
        let (width, height) = canvas_size(&canvas);

        let instance = wgpu::Instance::default();
        let surface: wgpu::Surface<'static> =
            instance.create_surface(wgpu::SurfaceTarget::Canvas(canvas.clone()))?;

        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            })
            .await?;

        let limits = wgpu::Limits::downlevel_webgl2_defaults()
            .using_resolution(adapter.limits())
            .using_alignment(adapter.limits());
        let device_desc = wgpu::DeviceDescriptor {
            label: Some("physalis-device"),
            required_features: wgpu::Features::empty(),
            required_limits: limits,
            ..Default::default()
        };
        let (device, queue) = adapter.request_device(&device_desc).await?;

        let mut config = surface
            .get_default_config(&adapter, width.max(1), height.max(1))
            .ok_or(RenderError::SurfaceUnsupported)?;
        config.present_mode = wgpu::PresentMode::Fifo;
        surface.configure(&device, &config);

        let camera = Camera::new(width, height);
        let camera_uniform = CameraUniform::from_camera(&camera);
        let camera_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("camera-buffer"),
            contents: bytemuck::bytes_of(&camera_uniform),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });
        let camera_bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("camera-bind-group-layout"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        });
        let camera_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("camera-bind-group"),
            layout: &camera_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: camera_buffer.as_entire_binding(),
            }],
        });

        let depth_texture = DepthTexture::new(&device, config.width, config.height);

        let (mesh_pipeline, line_pipeline) =
            create_pipelines(&device, &camera_bind_group_layout, config.format);
        let line_settings = LineSettings::default();
        let plane_visibility = PlaneVisibility::default();
        let (line_vertex_buffer, line_vertex_count) =
            create_line_buffers(&device, line_settings, plane_visibility);

        let state = RendererState {
            surface,
            device,
            queue,
            config,
            camera,
            camera_buffer,
            camera_bind_group,
            mesh_pipeline,
            line_pipeline,
            mesh_vertex_buffer: None,
            mesh_index_buffer: None,
            mesh_index_count: 0,
            line_vertex_buffer,
            line_vertex_count,
            line_settings,
            plane_visibility,
            depth_texture,
        };

        Ok(Self {
            state: Rc::new(RefCell::new(state)),
            _closures: Vec::new(),
        })
    }

    pub fn attach_default_controls(&mut self, canvas: &HtmlCanvasElement) {
        let input = Rc::new(RefCell::new(InputState::default()));

        // Mouse down
        {
            let input = input.clone();
            let closure = Closure::wrap(Box::new(move |event: web_sys::Event| {
                let event = event.dyn_into::<MouseEvent>().unwrap();
                let button = event.button();
                if button == 1 {
                    event.prevent_default();
                    let mut input = input.borrow_mut();
                    input.active_button = Some(button);
                    input.last_pos = Some((event.client_x() as f32, event.client_y() as f32));
                }
            }) as Box<dyn FnMut(_)>);
            let _ = canvas.add_event_listener_with_callback("mousedown", closure.as_ref().unchecked_ref());
            self._closures.push(closure);
        }

        // Mouse move
        {
            let state = self.state.clone();
            let input = input.clone();
            let closure = Closure::wrap(Box::new(move |event: web_sys::Event| {
                let event = event.dyn_into::<MouseEvent>().unwrap();
                let (prev, curr, dx, dy, button) = {
                    let mut input = input.borrow_mut();
                    if let (Some((lx, ly)), Some(button)) = (input.last_pos, input.active_button) {
                        let cx = event.client_x() as f32;
                        let cy = event.client_y() as f32;
                        let dx = cx - lx;
                        let dy = cy - ly;
                        input.last_pos = Some((cx, cy));
                        (Some((lx, ly)), (cx, cy), dx, dy, Some(button))
                    } else {
                        (None, (0.0, 0.0), 0.0, 0.0, None)
                    }
                };

                if let Some(button) = button {
                    event.prevent_default();
                    let mut state = state.borrow_mut();
                    if button == 1 {
                        if event.shift_key() {
                            if let Some(prev) = prev {
                                let (width, height) = (state.config.width, state.config.height);
                                state
                                    .camera
                                    .orbit_arcball(prev, curr, width, height);
                            }
                        } else {
                            state.camera.pan(dx, dy);
                        }
                    }
                    state.update_camera();
                    state.render();
                }
            }) as Box<dyn FnMut(_)>);
            let _ = canvas.add_event_listener_with_callback("mousemove", closure.as_ref().unchecked_ref());
            self._closures.push(closure);
        }

        // Mouse up / leave
        for event_name in ["mouseup", "mouseleave"] {
            let input = input.clone();
            let closure = Closure::wrap(Box::new(move |_event: web_sys::Event| {
                let mut input = input.borrow_mut();
                input.active_button = None;
                input.last_pos = None;
            }) as Box<dyn FnMut(_)>);
            let _ = canvas.add_event_listener_with_callback(event_name, closure.as_ref().unchecked_ref());
            self._closures.push(closure);
        }

        // Wheel
        {
            let state = self.state.clone();
            let closure = Closure::wrap(Box::new(move |event: web_sys::Event| {
                let event = event.dyn_into::<WheelEvent>().unwrap();
                event.prevent_default();
                let mut state = state.borrow_mut();
                state.camera.zoom(event.delta_y() as f32);
                state.update_camera();
                state.render();
            }) as Box<dyn FnMut(_)>);
            let _ = canvas.add_event_listener_with_callback("wheel", closure.as_ref().unchecked_ref());
            self._closures.push(closure);
        }

        // Prevent context menu on right-click.
        {
            let closure = Closure::wrap(Box::new(move |event: web_sys::Event| {
                event.prevent_default();
            }) as Box<dyn FnMut(_)>);
            let _ = canvas.add_event_listener_with_callback("contextmenu", closure.as_ref().unchecked_ref());
            self._closures.push(closure);
        }

        // Resize handler
        {
            let state = self.state.clone();
            let canvas = canvas.clone();
            let closure = Closure::wrap(Box::new(move |_event: web_sys::Event| {
                let (width, height) = canvas_size(&canvas);
                let mut state = state.borrow_mut();
                state.resize(width, height);
                state.update_camera();
                state.render();
            }) as Box<dyn FnMut(_)>);
            if let Some(window) = web_sys::window() {
                let _ = window.add_event_listener_with_callback("resize", closure.as_ref().unchecked_ref());
            }
            self._closures.push(closure);
        }
    }

    pub fn resize(&mut self, width: u32, height: u32) {
        let mut state = self.state.borrow_mut();
        state.resize(width, height);
        state.update_camera();
    }

    pub fn set_mesh(&mut self, mesh: TriMesh) {
        let mut state = self.state.borrow_mut();
        state.set_mesh(mesh);
    }

    pub fn set_plane_visibility(&mut self, xy: bool, yz: bool, zx: bool) {
        let mut state = self.state.borrow_mut();
        state.set_plane_visibility(xy, yz, zx);
    }

    pub fn render(&mut self) {
        let mut state = self.state.borrow_mut();
        state.render();
    }
}

#[derive(Default)]
struct InputState {
    last_pos: Option<(f32, f32)>,
    active_button: Option<i16>,
}

#[derive(Clone, Copy, PartialEq)]
struct PlaneVisibility {
    xy: bool,
    yz: bool,
    zx: bool,
}

impl Default for PlaneVisibility {
    fn default() -> Self {
        Self {
            xy: true,
            yz: false,
            zx: false,
        }
    }
}

#[derive(Clone, Copy)]
struct LineSettings {
    grid_half_extent: i32,
    spacing: f32,
    axis_len: f32,
    cube_size: f32,
}

impl Default for LineSettings {
    fn default() -> Self {
        Self {
            grid_half_extent: 12,
            spacing: 1.0,
            axis_len: 3.0,
            cube_size: 0.45,
        }
    }
}

struct RendererState {
    surface: wgpu::Surface<'static>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
    camera: Camera,
    camera_buffer: wgpu::Buffer,
    camera_bind_group: wgpu::BindGroup,
    mesh_pipeline: wgpu::RenderPipeline,
    line_pipeline: wgpu::RenderPipeline,
    mesh_vertex_buffer: Option<wgpu::Buffer>,
    mesh_index_buffer: Option<wgpu::Buffer>,
    mesh_index_count: u32,
    line_vertex_buffer: wgpu::Buffer,
    line_vertex_count: u32,
    line_settings: LineSettings,
    plane_visibility: PlaneVisibility,
    depth_texture: DepthTexture,
}

impl RendererState {
    fn set_mesh(&mut self, mesh: TriMesh) {
        if mesh.positions.is_empty() || mesh.indices.is_empty() {
            self.mesh_vertex_buffer = None;
            self.mesh_index_buffer = None;
            self.mesh_index_count = 0;
            return;
        }

        let mut vertices = Vec::with_capacity(mesh.positions.len());
        for (pos, normal) in mesh.positions.into_iter().zip(mesh.normals.into_iter()) {
            vertices.push(Vertex { position: pos, normal });
        }

        let vertex_buffer = self.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("mesh-vertex-buffer"),
            contents: bytemuck::cast_slice(&vertices),
            usage: wgpu::BufferUsages::VERTEX,
        });
        let index_buffer = self.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("mesh-index-buffer"),
            contents: bytemuck::cast_slice(&mesh.indices),
            usage: wgpu::BufferUsages::INDEX,
        });

        self.mesh_vertex_buffer = Some(vertex_buffer);
        self.mesh_index_buffer = Some(index_buffer);
        self.mesh_index_count = mesh.indices.len() as u32;
    }

    fn set_plane_visibility(&mut self, xy: bool, yz: bool, zx: bool) {
        let visibility = PlaneVisibility { xy, yz, zx };
        if self.plane_visibility != visibility {
            self.plane_visibility = visibility;
            self.rebuild_line_buffer();
        }
    }

    fn rebuild_line_buffer(&mut self) {
        let vertices = build_line_vertices(self.line_settings, self.plane_visibility);
        self.line_vertex_count = vertices.len() as u32;
        self.line_vertex_buffer = self.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("line-vertex-buffer"),
            contents: bytemuck::cast_slice(&vertices),
            usage: wgpu::BufferUsages::VERTEX,
        });
    }

    fn update_camera(&mut self) {
        let uniform = CameraUniform::from_camera(&self.camera);
        self.queue.write_buffer(&self.camera_buffer, 0, bytemuck::bytes_of(&uniform));
    }

    fn resize(&mut self, width: u32, height: u32) {
        if width == 0 || height == 0 {
            return;
        }
        self.config.width = width;
        self.config.height = height;
        self.surface.configure(&self.device, &self.config);
        self.depth_texture = DepthTexture::new(&self.device, width, height);
        self.camera.aspect = width as f32 / height as f32;
    }

    fn render(&mut self) {
        let frame = match self.surface.get_current_texture() {
            Ok(frame) => frame,
            Err(wgpu::SurfaceError::Lost) => {
                self.surface.configure(&self.device, &self.config);
                return;
            }
            Err(wgpu::SurfaceError::Outdated) => {
                self.surface.configure(&self.device, &self.config);
                return;
            }
            Err(wgpu::SurfaceError::Timeout) => {
                return;
            }
            Err(wgpu::SurfaceError::OutOfMemory) => {
                return;
            }
            Err(wgpu::SurfaceError::Other) => {
                return;
            }
        };

        let view = frame
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("render-encoder"),
            });

        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("render-pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 0.06,
                            g: 0.07,
                            b: 0.08,
                            a: 1.0,
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &self.depth_texture.view,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.0),
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });

            pass.set_bind_group(0, &self.camera_bind_group, &[]);

            // Mesh
            if let (Some(vertex_buffer), Some(index_buffer)) =
                (&self.mesh_vertex_buffer, &self.mesh_index_buffer)
            {
                pass.set_pipeline(&self.mesh_pipeline);
                pass.set_vertex_buffer(0, vertex_buffer.slice(..));
                pass.set_index_buffer(index_buffer.slice(..), wgpu::IndexFormat::Uint32);
                pass.draw_indexed(0..self.mesh_index_count, 0, 0..1);
            }

            // Grid + axes
            pass.set_pipeline(&self.line_pipeline);
            pass.set_vertex_buffer(0, self.line_vertex_buffer.slice(..));
            pass.draw(0..self.line_vertex_count, 0..1);
        }

        self.queue.submit(Some(encoder.finish()));
        frame.present();
    }
}

fn canvas_size(canvas: &HtmlCanvasElement) -> (u32, u32) {
    let window = web_sys::window().expect("window");
    let dpr = window.device_pixel_ratio() as f32;
    let width = (canvas.client_width() as f32 * dpr).max(1.0) as u32;
    let height = (canvas.client_height() as f32 * dpr).max(1.0) as u32;
    canvas.set_width(width);
    canvas.set_height(height);
    (width, height)
}

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct CameraUniform {
    view_proj: [[f32; 4]; 4],
}

impl CameraUniform {
    fn from_camera(camera: &Camera) -> Self {
        Self {
            view_proj: camera.view_proj().to_cols_array_2d(),
        }
    }
}

struct Camera {
    target: Vec3,
    radius: f32,
    rotation: glam::Quat,
    fov_y: f32,
    aspect: f32,
    near: f32,
    far: f32,
}

impl Camera {
    fn new(width: u32, height: u32) -> Self {
        let aspect = width as f32 / height.max(1) as f32;
        let yaw = 0.6;
        let pitch = 0.4;
        let rotation = glam::Quat::from_rotation_y(yaw) * glam::Quat::from_rotation_x(pitch);
        Self {
            target: Vec3::ZERO,
            radius: 4.0,
            rotation,
            fov_y: 45f32.to_radians(),
            aspect,
            near: 0.01,
            far: 1000.0,
        }
    }

    fn view_proj(&self) -> Mat4 {
        let offset = self.rotation * Vec3::new(0.0, 0.0, self.radius);
        let eye = self.target + offset;
        let up = self.rotation * Vec3::Y;
        let view = Mat4::look_at_rh(eye, self.target, up);
        let proj = Mat4::perspective_rh(self.fov_y, self.aspect.max(0.01), self.near, self.far);
        proj * view
    }

    fn orbit_arcball(&mut self, prev: (f32, f32), curr: (f32, f32), width: u32, height: u32) {
        let width = width.max(1) as f32;
        let height = height.max(1) as f32;
        let v0 = arcball_vector(prev.0, prev.1, width, height);
        let v1 = arcball_vector(curr.0, curr.1, width, height);
        // Invert arcball direction to match expected drag behavior.
        let axis = v1.cross(v0);
        let axis_len = axis.length();
        if axis_len < 1.0e-5 {
            return;
        }
        let dot = v0.dot(v1).clamp(-1.0, 1.0);
        let angle = dot.acos();
        let q = glam::Quat::from_axis_angle(axis / axis_len, angle);
        self.rotation = (q * self.rotation).normalize();
        self.constrain_up();
    }

    fn pan(&mut self, dx: f32, dy: f32) {
        let right = (self.rotation * Vec3::X).normalize();
        let up = (self.rotation * Vec3::Y).normalize();
        let scale = self.radius * 0.0025;
        // Invert vertical pan only to match Fusion-style drag.
        self.target += (right * dx + up * dy) * scale;
    }

    fn zoom(&mut self, delta: f32) {
        let zoom = (1.0 + delta * 0.001).max(0.05);
        self.radius = (self.radius * zoom).clamp(0.2, 200.0);
    }

    fn constrain_up(&mut self) {
        // Remove roll to keep "up" consistent and prevent inverted vertical drag.
        let back = (self.rotation * Vec3::Z).normalize();
        let world_up = Vec3::Y;
        let mut right = world_up.cross(back);
        if right.length_squared() < 1.0e-6 {
            right = (self.rotation * Vec3::X).normalize();
        } else {
            right = right.normalize();
        }
        let mut up = back.cross(right).normalize();
        if up.dot(world_up) < 0.0 {
            right = -right;
            up = -up;
        }
        let basis = glam::Mat3::from_cols(right, up, back);
        self.rotation = glam::Quat::from_mat3(&basis).normalize();
    }
}

fn arcball_vector(x: f32, y: f32, width: f32, height: f32) -> Vec3 {
    let nx = (2.0 * x - width) / width;
    let ny = (height - 2.0 * y) / height;
    let len2 = nx * nx + ny * ny;
    if len2 <= 1.0 {
        let z = (1.0 - len2).sqrt();
        Vec3::new(nx, ny, z)
    } else {
        let norm = len2.sqrt();
        Vec3::new(nx / norm, ny / norm, 0.0)
    }
}

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct Vertex {
    position: [f32; 3],
    normal: [f32; 3],
}

impl Vertex {
    fn desc() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Vertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[
                wgpu::VertexAttribute {
                    offset: 0,
                    shader_location: 0,
                    format: wgpu::VertexFormat::Float32x3,
                },
                wgpu::VertexAttribute {
                    offset: std::mem::size_of::<[f32; 3]>() as wgpu::BufferAddress,
                    shader_location: 1,
                    format: wgpu::VertexFormat::Float32x3,
                },
            ],
        }
    }
}

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct LineVertex {
    position: [f32; 3],
    color: [f32; 3],
}

impl LineVertex {
    fn desc() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<LineVertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[
                wgpu::VertexAttribute {
                    offset: 0,
                    shader_location: 0,
                    format: wgpu::VertexFormat::Float32x3,
                },
                wgpu::VertexAttribute {
                    offset: std::mem::size_of::<[f32; 3]>() as wgpu::BufferAddress,
                    shader_location: 1,
                    format: wgpu::VertexFormat::Float32x3,
                },
            ],
        }
    }
}

fn create_pipelines(
    device: &wgpu::Device,
    camera_layout: &wgpu::BindGroupLayout,
    color_format: wgpu::TextureFormat,
) -> (wgpu::RenderPipeline, wgpu::RenderPipeline) {
    let mesh_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("mesh-shader"),
        source: wgpu::ShaderSource::Wgsl(MESH_SHADER.into()),
    });
    let line_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("line-shader"),
        source: wgpu::ShaderSource::Wgsl(LINE_SHADER.into()),
    });

    let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("pipeline-layout"),
        bind_group_layouts: &[camera_layout],
        immediate_size: 0,
    });

    let mesh_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("mesh-pipeline"),
        layout: Some(&pipeline_layout),
        vertex: wgpu::VertexState {
            module: &mesh_shader,
            entry_point: Some("vs_main"),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            buffers: &[Vertex::desc()],
        },
        fragment: Some(wgpu::FragmentState {
            module: &mesh_shader,
            entry_point: Some("fs_main"),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            targets: &[Some(wgpu::ColorTargetState {
                format: color_format,
                blend: Some(wgpu::BlendState::REPLACE),
                write_mask: wgpu::ColorWrites::ALL,
            })],
        }),
        primitive: wgpu::PrimitiveState {
            topology: wgpu::PrimitiveTopology::TriangleList,
            strip_index_format: None,
            front_face: wgpu::FrontFace::Ccw,
            cull_mode: Some(wgpu::Face::Back),
            polygon_mode: wgpu::PolygonMode::Fill,
            unclipped_depth: false,
            conservative: false,
        },
        depth_stencil: Some(wgpu::DepthStencilState {
            format: wgpu::TextureFormat::Depth32Float,
            depth_write_enabled: true,
            depth_compare: wgpu::CompareFunction::LessEqual,
            stencil: wgpu::StencilState::default(),
            bias: wgpu::DepthBiasState::default(),
        }),
        multisample: wgpu::MultisampleState::default(),
        multiview_mask: None,
        cache: None,
    });

    let line_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("line-pipeline"),
        layout: Some(&pipeline_layout),
        vertex: wgpu::VertexState {
            module: &line_shader,
            entry_point: Some("vs_main"),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            buffers: &[LineVertex::desc()],
        },
        fragment: Some(wgpu::FragmentState {
            module: &line_shader,
            entry_point: Some("fs_main"),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            targets: &[Some(wgpu::ColorTargetState {
                format: color_format,
                blend: Some(wgpu::BlendState::REPLACE),
                write_mask: wgpu::ColorWrites::ALL,
            })],
        }),
        primitive: wgpu::PrimitiveState {
            topology: wgpu::PrimitiveTopology::LineList,
            strip_index_format: None,
            front_face: wgpu::FrontFace::Ccw,
            cull_mode: None,
            polygon_mode: wgpu::PolygonMode::Fill,
            unclipped_depth: false,
            conservative: false,
        },
        depth_stencil: Some(wgpu::DepthStencilState {
            format: wgpu::TextureFormat::Depth32Float,
            depth_write_enabled: false,
            depth_compare: wgpu::CompareFunction::LessEqual,
            stencil: wgpu::StencilState::default(),
            bias: wgpu::DepthBiasState::default(),
        }),
        multisample: wgpu::MultisampleState::default(),
        multiview_mask: None,
        cache: None,
    });

    (mesh_pipeline, line_pipeline)
}

fn create_line_buffers(
    device: &wgpu::Device,
    settings: LineSettings,
    visibility: PlaneVisibility,
) -> (wgpu::Buffer, u32) {
    let vertices = build_line_vertices(settings, visibility);
    let buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("line-vertex-buffer"),
        contents: bytemuck::cast_slice(&vertices),
        usage: wgpu::BufferUsages::VERTEX,
    });
    (buffer, vertices.len() as u32)
}

fn build_line_vertices(settings: LineSettings, visibility: PlaneVisibility) -> Vec<LineVertex> {
    let mut vertices = Vec::new();

    if visibility.xy {
        add_grid_xy(&mut vertices, settings);
    }
    if visibility.yz {
        add_grid_yz(&mut vertices, settings);
    }
    if visibility.zx {
        add_grid_zx(&mut vertices, settings);
    }

    add_axes(&mut vertices, settings.axis_len);
    add_origin_cube(&mut vertices, settings.cube_size);

    vertices
}

fn push_line(vertices: &mut Vec<LineVertex>, a: [f32; 3], b: [f32; 3], color: [f32; 3]) {
    vertices.push(LineVertex { position: a, color });
    vertices.push(LineVertex { position: b, color });
}

fn add_grid_xy(vertices: &mut Vec<LineVertex>, settings: LineSettings) {
    let grid_color = [0.23, 0.23, 0.23];
    let axis_grid_color = [0.35, 0.35, 0.35];
    let extent = settings.grid_half_extent as f32 * settings.spacing;
    for i in -settings.grid_half_extent..=settings.grid_half_extent {
        let t = i as f32 * settings.spacing;
        let color = if i == 0 { axis_grid_color } else { grid_color };
        push_line(vertices, [t, -extent, 0.0], [t, extent, 0.0], color);
        push_line(vertices, [-extent, t, 0.0], [extent, t, 0.0], color);
    }
}

fn add_grid_yz(vertices: &mut Vec<LineVertex>, settings: LineSettings) {
    let grid_color = [0.16, 0.28, 0.32];
    let axis_grid_color = [0.22, 0.42, 0.48];
    let extent = settings.grid_half_extent as f32 * settings.spacing;
    for i in -settings.grid_half_extent..=settings.grid_half_extent {
        let t = i as f32 * settings.spacing;
        let color = if i == 0 { axis_grid_color } else { grid_color };
        push_line(vertices, [0.0, -extent, t], [0.0, extent, t], color);
        push_line(vertices, [0.0, t, -extent], [0.0, t, extent], color);
    }
}

fn add_grid_zx(vertices: &mut Vec<LineVertex>, settings: LineSettings) {
    let grid_color = [0.28, 0.2, 0.32];
    let axis_grid_color = [0.42, 0.28, 0.48];
    let extent = settings.grid_half_extent as f32 * settings.spacing;
    for i in -settings.grid_half_extent..=settings.grid_half_extent {
        let t = i as f32 * settings.spacing;
        let color = if i == 0 { axis_grid_color } else { grid_color };
        push_line(vertices, [t, 0.0, -extent], [t, 0.0, extent], color);
        push_line(vertices, [-extent, 0.0, t], [extent, 0.0, t], color);
    }
}

fn add_axes(vertices: &mut Vec<LineVertex>, axis_len: f32) {
    push_line(vertices, [0.0, 0.0, 0.0], [axis_len, 0.0, 0.0], [1.0, 0.1, 0.1]);
    push_line(vertices, [0.0, 0.0, 0.0], [0.0, axis_len, 0.0], [0.1, 1.0, 0.1]);
    push_line(vertices, [0.0, 0.0, 0.0], [0.0, 0.0, axis_len], [0.1, 0.3, 1.0]);
}

fn add_origin_cube(vertices: &mut Vec<LineVertex>, size: f32) {
    let h = size / 2.0;
    let color = [0.7, 0.72, 0.75];
    let p = [
        [-h, -h, -h],
        [h, -h, -h],
        [h, h, -h],
        [-h, h, -h],
        [-h, -h, h],
        [h, -h, h],
        [h, h, h],
        [-h, h, h],
    ];
    let edges = [
        (0, 1),
        (1, 2),
        (2, 3),
        (3, 0),
        (4, 5),
        (5, 6),
        (6, 7),
        (7, 4),
        (0, 4),
        (1, 5),
        (2, 6),
        (3, 7),
    ];
    for (a, b) in edges {
        push_line(vertices, p[a], p[b], color);
    }
}

struct DepthTexture {
    _texture: wgpu::Texture,
    view: wgpu::TextureView,
}

impl DepthTexture {
    fn new(device: &wgpu::Device, width: u32, height: u32) -> Self {
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("depth-texture"),
            size: wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Depth32Float,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        });
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        Self {
            _texture: texture,
            view,
        }
    }
}

const MESH_SHADER: &str = r#"
struct Camera {
  view_proj: mat4x4<f32>,
};

@group(0) @binding(0)
var<uniform> camera: Camera;

struct VertexInput {
  @location(0) position: vec3<f32>,
  @location(1) normal: vec3<f32>,
};

struct VertexOutput {
  @builtin(position) position: vec4<f32>,
  @location(0) normal: vec3<f32>,
};

@vertex
fn vs_main(input: VertexInput) -> VertexOutput {
  var out: VertexOutput;
  out.position = camera.view_proj * vec4<f32>(input.position, 1.0);
  out.normal = normalize(input.normal);
  return out;
}

@fragment
fn fs_main(input: VertexOutput) -> @location(0) vec4<f32> {
  let light_dir = normalize(vec3<f32>(0.4, 0.7, 1.0));
  let diffuse = max(dot(input.normal, light_dir), 0.0);
  let base = vec3<f32>(0.78, 0.8, 0.84);
  let color = base * (0.2 + 0.8 * diffuse);
  return vec4<f32>(color, 1.0);
}
"#;

const LINE_SHADER: &str = r#"
struct Camera {
  view_proj: mat4x4<f32>,
};

@group(0) @binding(0)
var<uniform> camera: Camera;

struct VertexInput {
  @location(0) position: vec3<f32>,
  @location(1) color: vec3<f32>,
};

struct VertexOutput {
  @builtin(position) position: vec4<f32>,
  @location(0) color: vec3<f32>,
};

@vertex
fn vs_main(input: VertexInput) -> VertexOutput {
  var out: VertexOutput;
  out.position = camera.view_proj * vec4<f32>(input.position, 1.0);
  out.color = input.color;
  return out;
}

@fragment
fn fs_main(input: VertexOutput) -> @location(0) vec4<f32> {
  return vec4<f32>(input.color, 1.0);
}
"#;

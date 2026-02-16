use cad_core::{ObjectId, Transform};
use cad_geom::GeomScene;
use cad_protocol::{ClientMsg, ServerMsg};
use cad_render::{OverlayLine, Renderer};
use glam::{EulerRot, Mat3, Quat, Vec3};
use leptos::html::Canvas;
use leptos::prelude::*;
use std::cell::RefCell;
use std::rc::Rc;
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::spawn_local;
use web_sys::{
    CanvasRenderingContext2d, HtmlInputElement, KeyboardEvent, MessageEvent, MouseEvent, WebSocket,
};

#[wasm_bindgen(start)]
pub fn start() {
    console_error_panic_hook::set_once();
    mount_to_body(|| view! { <App /> });
}

#[component]
fn App() -> impl IntoView {
    let canvas_ref = NodeRef::<Canvas>::new();
    let viewcube_ref = NodeRef::<Canvas>::new();
    let scene = Rc::new(RefCell::new(GeomScene::new()));
    let renderer = Rc::new(RefCell::new(None::<Renderer>));
    let ws_handle = Rc::new(RefCell::new(None::<WebSocket>));
    let (renderer_ready, set_renderer_ready) = signal(false);
    let (plane_xy, set_plane_xy) = signal(true);
    let (plane_yz, set_plane_yz) = signal(false);
    let (plane_zx, set_plane_zx) = signal(false);
    let (object_count, set_object_count) = signal(0usize);

    let (tool_mode, set_tool_mode) = signal(EditorTool::None);
    let (selected_id, set_selected_id) = signal(None::<ObjectId>);
    let (baseline_transform, set_baseline_transform) = signal(None::<Transform>);
    let (transform_ui, set_transform_ui) = signal(TransformUi::default());
    let drag_state = Rc::new(RefCell::new(None::<DragState>));
    let editor_attached = Rc::new(RefCell::new(false));

    // WebSocket connection
    {
        let ws_handle = ws_handle.clone();
        Effect::new(move |_| {
            if ws_handle.borrow().is_none() {
                connect_ws(ws_handle.clone());
            }
        });
    }

    schedule_renderer_init(
        canvas_ref,
        renderer.clone(),
        set_renderer_ready,
        plane_xy,
        plane_yz,
        plane_zx,
    );

    // Attach editor controls once we have both the canvas and renderer.
    {
        let scene = scene.clone();
        let renderer = renderer.clone();
        let editor_attached = editor_attached.clone();
        Effect::new(move |_| {
            if *editor_attached.borrow() {
                return;
            }
            let Some(canvas) = canvas_ref.get() else {
                return;
            };
            let Some(viewcube_canvas) = viewcube_ref.get() else {
                return;
            };
            if !renderer_ready.get() {
                return;
            }

            attach_editor_controls(
                canvas.clone(),
                viewcube_canvas.clone(),
                scene.clone(),
                renderer.clone(),
                tool_mode,
                set_tool_mode,
                set_selected_id,
                selected_id,
                set_baseline_transform,
                set_transform_ui,
                drag_state.clone(),
            );
            *editor_attached.borrow_mut() = true;
        });
    }

    let on_add_box = {
        let scene = scene.clone();
        let renderer = renderer.clone();
        let set_object_count = set_object_count;
        move |_| {
            let id = {
                let mut scene = scene.borrow_mut();
                let id = scene.add_box(1.0, 1.0, 1.0);
                set_object_count.set(scene.model().objects().len());
                id
            };
            update_mesh(&scene, &renderer);
            set_selected_id.set(Some(id));
            if let Some(transform) = scene.borrow().object_transform(id) {
                set_baseline_transform.set(Some(transform));
                set_transform_ui.set(TransformUi::from_transform(transform));
            }
        }
    };

    let on_add_cylinder = {
        let scene = scene.clone();
        let renderer = renderer.clone();
        let set_object_count = set_object_count;
        move |_| {
            let id = {
                let mut scene = scene.borrow_mut();
                let id = scene.add_cylinder(0.5, 1.5);
                set_object_count.set(scene.model().objects().len());
                id
            };
            update_mesh(&scene, &renderer);
            set_selected_id.set(Some(id));
            if let Some(transform) = scene.borrow().object_transform(id) {
                set_baseline_transform.set(Some(transform));
                set_transform_ui.set(TransformUi::from_transform(transform));
            }
        }
    };

    let on_boolean_stub = move |_| {
        log("Boolean subtract is not implemented yet.");
    };

    let on_export_stub = move |_| {
        log("Export STEP is not implemented yet.");
    };

    {
        let renderer = renderer.clone();
        let plane_xy = plane_xy.clone();
        let plane_yz = plane_yz.clone();
        let plane_zx = plane_zx.clone();
        Effect::new(move |_| {
            let xy = plane_xy.get();
            let yz = plane_yz.get();
            let zx = plane_zx.get();
            if let Some(renderer) = renderer.borrow_mut().as_mut() {
                renderer.set_plane_visibility(xy, yz, zx);
                renderer.render();
            }
        });
    }

    {
        let scene = scene.clone();
        let renderer = renderer.clone();
        Effect::new(move |_| {
            if !renderer_ready.get() {
                return;
            }
            let selected = selected_id.get();
            let show_gizmo = tool_mode.get() == EditorTool::Move;
            update_overlay(&scene, &renderer, selected, show_gizmo);
        });
    }

    view! {
        <div class="cad-shell">
            <header class="cad-tabs">
                <div class="tabs-left">
                    <div class="brand">"physalis"</div>
                    <button class="tab-btn active">"Model"</button>
                    <button class="tab-btn">"Surface"</button>
                    <button class="tab-btn">"Mesh"</button>
                    <button class="tab-btn">"Tools"</button>
                </div>
                <div class="tabs-right">
                    <span class="save-dot"></span>
                    <span class="tabs-meta">"Saved"</span>
                </div>
            </header>
            <section class="cad-ribbon">
                <div class="ribbon-group">
                    <div class="ribbon-title">"Create"</div>
                    <div class="ribbon-buttons">
                        <button class="tool-btn" on:click=on_add_box>"Box"</button>
                        <button class="tool-btn" on:click=on_add_cylinder>"Cylinder"</button>
                    </div>
                </div>
                <div class="ribbon-group">
                    <div class="ribbon-title">"Modify"</div>
                    <div class="ribbon-buttons">
                        <button
                            class="tool-btn"
                            class:active=move || tool_mode.get() == EditorTool::Move
                            on:click=move |_| set_tool_mode.set(EditorTool::Move)
                        >
                            "Move"
                        </button>
                        <button
                            class="tool-btn"
                            class:active=move || tool_mode.get() == EditorTool::None
                            on:click=move |_| set_tool_mode.set(EditorTool::None)
                        >
                            "View"
                        </button>
                        <button class="tool-btn" on:click=on_boolean_stub>"Boolean"</button>
                    </div>
                </div>
                <div class="ribbon-group">
                    <div class="ribbon-title">"Export"</div>
                    <div class="ribbon-buttons">
                        <button class="tool-btn" on:click=on_export_stub>"STEP"</button>
                    </div>
                </div>
            </section>
            <div class="cad-main">
                <aside class="browser">
                    <div class="browser-search">
                        <input class="browser-input" type="text" placeholder="Search browser..." />
                    </div>
                    <div class="browser-content">
                        <div class="browser-section">
                            <h2>"Selection"</h2>
                            <div class="tree-row">
                                <span class="tree-label">"Active body"</span>
                                <span class="tree-value">
                                    {move || {
                                        selected_id
                                            .get()
                                            .map(|id| format!("#{id}"))
                                            .unwrap_or_else(|| "none".to_string())
                                    }}
                                </span>
                            </div>
                            <div class="tree-row">
                                <span class="tree-label">"Objects"</span>
                                <span class="tree-value">{move || object_count.get().to_string()}</span>
                            </div>
                        </div>
                        <div class="browser-section">
                            <h2>"Origin"</h2>
                            <label class="toggle">
                                <input
                                    type="checkbox"
                                    prop:checked=plane_xy
                                    on:change=move |ev| set_plane_xy.set(event_target_checked(&ev))
                                />
                                <span>"XY plane"</span>
                            </label>
                            <label class="toggle">
                                <input
                                    type="checkbox"
                                    prop:checked=plane_yz
                                    on:change=move |ev| set_plane_yz.set(event_target_checked(&ev))
                                />
                                <span>"YZ plane"</span>
                            </label>
                            <label class="toggle">
                                <input
                                    type="checkbox"
                                    prop:checked=plane_zx
                                    on:change=move |ev| set_plane_zx.set(event_target_checked(&ev))
                                />
                                <span>"ZX plane"</span>
                            </label>
                        </div>
                        <div class="panel-note">
                            <p>"MMB drag: pan"</p>
                            <p>"Shift + MMB: orbit"</p>
                            <p>"Wheel: zoom"</p>
                        </div>
                    </div>
                </aside>
                <main class="viewport-frame">
                    <div class="viewport-grid-bg"></div>
                    <canvas id="viewport-canvas" node_ref=canvas_ref></canvas>
                    <canvas id="viewcube-canvas" node_ref=viewcube_ref></canvas>
                    <div class="viewport-toolbar">
                        <button
                            class="nav-btn"
                            class:active=move || tool_mode.get() == EditorTool::None
                            on:click=move |_| set_tool_mode.set(EditorTool::None)
                        >
                            "Select"
                        </button>
                        <button
                            class="nav-btn"
                            class:active=move || tool_mode.get() == EditorTool::Move
                            on:click=move |_| set_tool_mode.set(EditorTool::Move)
                        >
                            "Move"
                        </button>
                        <button class="nav-btn" prop:disabled=true>
                            "Zoom"
                        </button>
                    </div>
                    <aside
                        class="inspector-card"
                        class:open=move || selected_id.get().is_some() && tool_mode.get() == EditorTool::Move
                    >
                        <h2>"Transform"</h2>
                        <TransformPanel
                            selected_id=selected_id
                            transform_ui=transform_ui
                            on_change={
                                let scene = scene.clone();
                                let renderer = renderer.clone();
                                Rc::new(move |ui| {
                                    set_transform_ui.set(ui);
                                    if let Some(id) = selected_id.get_untracked() {
                                        let t = ui.to_transform();
                                        apply_transform(&scene, &renderer, id, t);
                                        update_overlay(
                                            &scene,
                                            &renderer,
                                            Some(id),
                                            tool_mode.get_untracked() == EditorTool::Move,
                                        );
                                    }
                                })
                            }
                            on_ok={
                                let selected_id = selected_id;
                                let transform_ui = transform_ui;
                                Rc::new(move || {
                                    if selected_id.get_untracked().is_some() {
                                        set_baseline_transform
                                            .set(Some(transform_ui.get_untracked().to_transform()));
                                    }
                                    set_tool_mode.set(EditorTool::None);
                                })
                            }
                            on_cancel={
                                let scene = scene.clone();
                                let renderer = renderer.clone();
                                Rc::new(move || {
                                    let Some(id) = selected_id.get_untracked() else {
                                        return;
                                    };
                                    let Some(base) = baseline_transform.get_untracked() else {
                                        return;
                                    };
                                    apply_transform(&scene, &renderer, id, base);
                                    set_transform_ui.set(TransformUi::from_transform(base));
                                    update_overlay(
                                        &scene,
                                        &renderer,
                                        Some(id),
                                        tool_mode.get_untracked() == EditorTool::Move,
                                    );
                                    set_tool_mode.set(EditorTool::None);
                                })
                            }
                        />
                    </aside>
                    <div class="viewport-status">
                        <span>{move || format!("Objects: {}", object_count.get())}</span>
                        <span>
                            {move || {
                                if tool_mode.get() == EditorTool::Move {
                                    "Tool: Move".to_string()
                                } else {
                                    "Tool: View".to_string()
                                }
                            }}
                        </span>
                        <span class="status-hint">"LMB select | M move | MMB pan | Wheel zoom"</span>
                    </div>
                </main>
            </div>
            <footer class="cad-timeline">
                <div class="timeline-controls">
                    <span class="timeline-title">"Feature History"</span>
                    <button class="timeline-control">"Step Back"</button>
                    <button class="timeline-control">"Play"</button>
                    <button class="timeline-control">"Step Forward"</button>
                </div>
                <div class="timeline-track">
                    <button class="timeline-chip">"01 Sketch"</button>
                    <button class="timeline-chip active">"02 Extrude"</button>
                    <button class="timeline-chip">"03 Fillet"</button>
                    <button class="timeline-chip">"04 Pattern"</button>
                    <button class="timeline-chip">"05 Inspect"</button>
                </div>
            </footer>
        </div>
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum EditorTool {
    None,
    Move,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Axis {
    X,
    Y,
    Z,
}

#[derive(Clone, Copy)]
enum DragMode {
    Translate,
    Rotate(Axis),
}

#[derive(Clone, Copy)]
struct DragState {
    object_id: ObjectId,
    mode: DragMode,
    start_transform: Transform,
    start_origin_world: Vec3,
    // Translate-only.
    axis_dir_world: Vec3,
    plane_normal_world: Vec3,
    start_axis_t: f32,
    // Rotate-only.
    ring_u_world: Vec3,
    ring_v_world: Vec3,
    start_angle: f32,
}

#[derive(Clone, Copy)]
struct TransformUi {
    tx: f32,
    ty: f32,
    tz: f32,
    rx_deg: f32,
    ry_deg: f32,
    rz_deg: f32,
}

impl Default for TransformUi {
    fn default() -> Self {
        Self {
            tx: 0.0,
            ty: 0.0,
            tz: 0.0,
            rx_deg: 0.0,
            ry_deg: 0.0,
            rz_deg: 0.0,
        }
    }
}

impl TransformUi {
    fn from_transform(transform: Transform) -> Self {
        let q = quat_from_transform(transform);
        let (rx, ry, rz) = q.to_euler(EulerRot::XYZ);
        Self {
            tx: transform.translation[0],
            ty: transform.translation[1],
            tz: transform.translation[2],
            rx_deg: rx.to_degrees(),
            ry_deg: ry.to_degrees(),
            rz_deg: rz.to_degrees(),
        }
    }

    fn to_transform(self) -> Transform {
        let q = Quat::from_euler(
            EulerRot::XYZ,
            self.rx_deg.to_radians(),
            self.ry_deg.to_radians(),
            self.rz_deg.to_radians(),
        )
        .normalize();
        Transform {
            translation: [self.tx, self.ty, self.tz],
            rotation: [q.x, q.y, q.z, q.w],
        }
    }
}

#[component]
fn TransformPanel(
    selected_id: ReadSignal<Option<ObjectId>>,
    transform_ui: ReadSignal<TransformUi>,
    on_change: Rc<dyn Fn(TransformUi)>,
    on_ok: Rc<dyn Fn()>,
    on_cancel: Rc<dyn Fn()>,
) -> impl IntoView {
    let (tx_text, set_tx_text) = signal(String::new());
    let (ty_text, set_ty_text) = signal(String::new());
    let (tz_text, set_tz_text) = signal(String::new());
    let (rx_text, set_rx_text) = signal(String::new());
    let (ry_text, set_ry_text) = signal(String::new());
    let (rz_text, set_rz_text) = signal(String::new());

    let (tx_focused, set_tx_focused) = signal(false);
    let (ty_focused, set_ty_focused) = signal(false);
    let (tz_focused, set_tz_focused) = signal(false);
    let (rx_focused, set_rx_focused) = signal(false);
    let (ry_focused, set_ry_focused) = signal(false);
    let (rz_focused, set_rz_focused) = signal(false);

    {
        let set_tx_text = set_tx_text;
        Effect::new(move |_| {
            if tx_focused.get() {
                return;
            }
            let ui = transform_ui.get();
            set_tx_text.set(format!("{:.4}", ui.tx));
        });
    }
    {
        let set_ty_text = set_ty_text;
        Effect::new(move |_| {
            if ty_focused.get() {
                return;
            }
            let ui = transform_ui.get();
            set_ty_text.set(format!("{:.4}", ui.ty));
        });
    }
    {
        let set_tz_text = set_tz_text;
        Effect::new(move |_| {
            if tz_focused.get() {
                return;
            }
            let ui = transform_ui.get();
            set_tz_text.set(format!("{:.4}", ui.tz));
        });
    }
    {
        let set_rx_text = set_rx_text;
        Effect::new(move |_| {
            if rx_focused.get() {
                return;
            }
            let ui = transform_ui.get();
            set_rx_text.set(format!("{:.1}", ui.rx_deg));
        });
    }
    {
        let set_ry_text = set_ry_text;
        Effect::new(move |_| {
            if ry_focused.get() {
                return;
            }
            let ui = transform_ui.get();
            set_ry_text.set(format!("{:.1}", ui.ry_deg));
        });
    }
    {
        let set_rz_text = set_rz_text;
        Effect::new(move |_| {
            if rz_focused.get() {
                return;
            }
            let ui = transform_ui.get();
            set_rz_text.set(format!("{:.1}", ui.rz_deg));
        });
    }

    let make_input = {
        let on_ok = on_ok.clone();
        let on_change = on_change.clone();
        move |label: &'static str,
              text: ReadSignal<String>,
              set_text: WriteSignal<String>,
              set_focused: WriteSignal<bool>,
              set: fn(&mut TransformUi, f32),
              format_hint: &'static str| {
            let on_ok = on_ok.clone();
            let on_change = on_change.clone();
            view! {
                <label class="field">
                    <span class="field-label">{label}</span>
                    <input
                        class="field-input"
                        type="text"
                        inputmode={format_hint}
                        prop:value=move || text.get()
                        on:focus=move |ev| {
                            set_focused.set(true);
                            if let Some(target) = ev.target() {
                                if let Ok(input) = target.dyn_into::<HtmlInputElement>() {
                                    input.select();
                                }
                            }
                        }
                        on:blur=move |_| set_focused.set(false)
                        on:input=move |ev| {
                            let raw = event_target_value(&ev);
                            set_text.set(raw.clone());

                            let Some(v) = parse_f32_input(&raw) else {
                                return;
                            };
                            let mut ui = transform_ui.get_untracked();
                            set(&mut ui, v);
                            (on_change.as_ref())(ui);
                        }
                        on:keydown=move |ev| {
                            let ev = ev.dyn_into::<KeyboardEvent>().unwrap();
                            if ev.key() == "Enter" {
                                ev.prevent_default();
                                (on_ok.as_ref())();
                            }
                        }
                    />
                </label>
            }
        }
    };

    view! {
        <div class="transform-panel" class:disabled=move || selected_id.get().is_none()>
            <h3>"Translate (m)"</h3>
            <div class="field-grid">
                {make_input(
                    "X",
                    tx_text,
                    set_tx_text,
                    set_tx_focused,
                    |u, v| u.tx = v,
                    "decimal",
                )}
                {make_input(
                    "Y",
                    ty_text,
                    set_ty_text,
                    set_ty_focused,
                    |u, v| u.ty = v,
                    "decimal",
                )}
                {make_input(
                    "Z",
                    tz_text,
                    set_tz_text,
                    set_tz_focused,
                    |u, v| u.tz = v,
                    "decimal",
                )}
            </div>
            <h3>"Rotate (deg)"</h3>
            <div class="field-grid">
                {make_input(
                    "X",
                    rx_text,
                    set_rx_text,
                    set_rx_focused,
                    |u, v| u.rx_deg = v,
                    "decimal",
                )}
                {make_input(
                    "Y",
                    ry_text,
                    set_ry_text,
                    set_ry_focused,
                    |u, v| u.ry_deg = v,
                    "decimal",
                )}
                {make_input(
                    "Z",
                    rz_text,
                    set_rz_text,
                    set_rz_focused,
                    |u, v| u.rz_deg = v,
                    "decimal",
                )}
            </div>
            <div class="transform-actions">
                <button
                    class="action-btn primary"
                    prop:disabled=move || selected_id.get().is_none()
                    on:click={
                        let on_ok = on_ok.clone();
                        move |_| (on_ok.as_ref())()
                    }
                >
                    "OK"
                </button>
                <button
                    class="action-btn"
                    prop:disabled=move || selected_id.get().is_none()
                    on:click={
                        let on_cancel = on_cancel.clone();
                        move |_| (on_cancel.as_ref())()
                    }
                >
                    "Cancel"
                </button>
            </div>
        </div>
    }
}

fn parse_f32_input(raw: &str) -> Option<f32> {
    let s = raw.trim();
    if s.is_empty() {
        return None;
    }
    let s = s.replace(',', ".");
    s.parse::<f32>().ok()
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ViewCubeFace {
    PosX,
    NegX,
    PosY,
    NegY,
    PosZ,
    NegZ,
}

impl ViewCubeFace {
    fn normal(self) -> Vec3 {
        match self {
            Self::PosX => Vec3::X,
            Self::NegX => -Vec3::X,
            Self::PosY => Vec3::Y,
            Self::NegY => -Vec3::Y,
            Self::PosZ => Vec3::Z,
            Self::NegZ => -Vec3::Z,
        }
    }

    fn base_color(self) -> &'static str {
        match self {
            Self::PosX => "rgb(255, 110, 110)",
            Self::NegX => "rgb(170, 70, 70)",
            Self::PosY => "rgb(120, 255, 150)",
            Self::NegY => "rgb(70, 160, 95)",
            Self::PosZ => "rgb(110, 150, 255)",
            Self::NegZ => "rgb(70, 95, 170)",
        }
    }

    fn snap_vectors(self) -> (Vec3, Vec3) {
        let dir = self.normal();
        // Prefer a stable up-hint that's not collinear with the snap direction.
        let up_hint = if dir.dot(Vec3::Z).abs() < 0.9 {
            Vec3::Z
        } else {
            Vec3::Y
        };
        (dir, up_hint)
    }
}

#[derive(Clone)]
struct ViewCubeFaceHit {
    face: ViewCubeFace,
    poly: [(f64, f64); 4],
    depth: f32,
}

#[derive(Clone)]
struct ViewCubeState {
    canvas: web_sys::HtmlCanvasElement,
    ctx: CanvasRenderingContext2d,
    pending: Rc<RefCell<bool>>,
    hit_faces: Rc<RefCell<Vec<ViewCubeFaceHit>>>,
}

impl ViewCubeState {
    const SIZE_CSS: f64 = 84.0;

    fn new(canvas: web_sys::HtmlCanvasElement) -> Self {
        let ctx = canvas
            .get_context("2d")
            .ok()
            .flatten()
            .and_then(|v| v.dyn_into::<CanvasRenderingContext2d>().ok())
            .unwrap();
        let state = Self {
            canvas,
            ctx,
            pending: Rc::new(RefCell::new(false)),
            hit_faces: Rc::new(RefCell::new(Vec::new())),
        };
        state.ensure_canvas_resolution();
        state
    }

    fn request_draw(&self, renderer: &Rc<RefCell<Option<Renderer>>>) {
        if *self.pending.borrow() {
            return;
        }
        *self.pending.borrow_mut() = true;
        let renderer = renderer.clone();
        let state = self.clone();
        request_animation_frame(move || {
            *state.pending.borrow_mut() = false;
            state.draw_now(&renderer);
        });
    }

    fn draw_now(&self, renderer: &Rc<RefCell<Option<Renderer>>>) {
        let rotation = {
            let renderer_borrow = renderer.borrow();
            let Some(r) = renderer_borrow.as_ref() else {
                return;
            };
            Quat::from_array(r.camera_rotation()).normalize()
        };
        self.draw(rotation);
    }

    fn hit_face(&self, x: f64, y: f64) -> Option<ViewCubeFace> {
        let faces = self.hit_faces.borrow();
        for f in faces.iter() {
            if point_in_poly((x, y), &f.poly) {
                return Some(f.face);
            }
        }
        None
    }

    fn ensure_canvas_resolution(&self) {
        let dpr = web_sys::window()
            .map(|w| w.device_pixel_ratio())
            .unwrap_or(1.0)
            .max(1.0);
        let w = (Self::SIZE_CSS * dpr).round() as u32;
        let h = w;
        if self.canvas.width() != w {
            self.canvas.set_width(w);
            self.canvas.set_height(h);
        }
        let _ = self.ctx.set_transform(dpr, 0.0, 0.0, dpr, 0.0, 0.0);
    }

    fn draw(&self, camera_rot: Quat) {
        self.ensure_canvas_resolution();

        let size = Self::SIZE_CSS;
        self.ctx.clear_rect(0.0, 0.0, size, size);

        let view_rot = camera_rot.conjugate();

        let cube = [
            Vec3::new(-1.0, -1.0, -1.0),
            Vec3::new(1.0, -1.0, -1.0),
            Vec3::new(1.0, 1.0, -1.0),
            Vec3::new(-1.0, 1.0, -1.0),
            Vec3::new(-1.0, -1.0, 1.0),
            Vec3::new(1.0, -1.0, 1.0),
            Vec3::new(1.0, 1.0, 1.0),
            Vec3::new(-1.0, 1.0, 1.0),
        ];
        let cube_scale = 0.82;
        let verts_cam: [Vec3; 8] = cube.map(|v| view_rot * (v * cube_scale));

        let project = |p: Vec3| -> (f64, f64) {
            // Tiny perspective projection, camera at (0,0,dist) looking at origin.
            let dist = 4.0;
            let denom = (dist - p.z).max(0.05);
            let x = p.x / denom;
            let y = p.y / denom;
            let scale_px = size * 0.36;
            let cx = size * 0.5;
            let cy = size * 0.5;
            (cx + x as f64 * scale_px, cy - y as f64 * scale_px)
        };

        let verts_2d: [(f64, f64); 8] = verts_cam.map(project);

        let mut visible = Vec::<ViewCubeFaceHit>::new();
        let faces = [
            (ViewCubeFace::PosX, [1, 2, 6, 5]),
            (ViewCubeFace::NegX, [0, 4, 7, 3]),
            (ViewCubeFace::PosY, [3, 2, 6, 7]),
            (ViewCubeFace::NegY, [0, 1, 5, 4]),
            (ViewCubeFace::PosZ, [4, 5, 6, 7]),
            (ViewCubeFace::NegZ, [0, 3, 2, 1]),
        ];

        for (face, idx) in faces {
            let n_cam = view_rot * face.normal();
            if n_cam.z <= 0.0 {
                continue;
            }
            let poly = [
                verts_2d[idx[0]],
                verts_2d[idx[1]],
                verts_2d[idx[2]],
                verts_2d[idx[3]],
            ];
            let depth = (verts_cam[idx[0]].z
                + verts_cam[idx[1]].z
                + verts_cam[idx[2]].z
                + verts_cam[idx[3]].z)
                / 4.0;
            visible.push(ViewCubeFaceHit { face, poly, depth });
        }

        visible.sort_by(|a, b| {
            a.depth
                .partial_cmp(&b.depth)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        for f in visible.iter() {
            let n_cam = view_rot * f.face.normal();
            let z = n_cam.z.clamp(0.0, 1.0) as f64;
            let base = if matches!(
                f.face,
                ViewCubeFace::PosX | ViewCubeFace::PosY | ViewCubeFace::PosZ
            ) {
                0.92
            } else {
                0.72
            };
            let alpha = (base * (0.58 + 0.42 * z)).clamp(0.25, 0.96);

            self.ctx.begin_path();
            self.ctx.move_to(f.poly[0].0, f.poly[0].1);
            for p in &f.poly[1..] {
                self.ctx.line_to(p.0, p.1);
            }
            self.ctx.close_path();

            self.ctx.set_global_alpha(alpha);
            self.ctx.set_fill_style_str(f.face.base_color());
            let _ = self.ctx.fill();
            self.ctx.set_global_alpha(1.0);

            self.ctx.set_stroke_style_str("rgba(235, 240, 246, 0.55)");
            self.ctx.set_line_width(1.0);
            let _ = self.ctx.stroke();
        }

        // Axes (origin-aligned).
        let center = project(Vec3::ZERO);
        let axis_scale = cube_scale * 1.25;
        let axes = [
            ("X", Vec3::X, "rgb(255, 90, 90)"),
            ("Y", Vec3::Y, "rgb(90, 255, 120)"),
            ("Z", Vec3::Z, "rgb(90, 140, 255)"),
        ];
        self.ctx
            .set_font("600 10px \"Space Grotesk\", system-ui, sans-serif");
        self.ctx.set_text_align("center");
        self.ctx.set_text_baseline("middle");

        for (label, dir, color) in axes {
            let p = view_rot * (dir * axis_scale);
            let end = project(p);

            self.ctx.begin_path();
            self.ctx.move_to(center.0, center.1);
            self.ctx.line_to(end.0, end.1);
            self.ctx.set_stroke_style_str(color);
            self.ctx.set_line_width(1.6);
            let _ = self.ctx.stroke();

            self.ctx.begin_path();
            let r = 6.2;
            let _ = self.ctx.arc(end.0, end.1, r, 0.0, std::f64::consts::TAU);
            self.ctx.set_fill_style_str(color);
            let _ = self.ctx.fill();

            self.ctx.set_fill_style_str("rgba(7, 9, 12, 0.92)");
            let _ = self.ctx.fill_text(label, end.0, end.1);
        }

        // Hit areas: keep only visible faces, nearest-first.
        let mut hit = visible;
        hit.sort_by(|a, b| {
            b.depth
                .partial_cmp(&a.depth)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        *self.hit_faces.borrow_mut() = hit;
    }
}

fn point_in_poly(p: (f64, f64), poly: &[(f64, f64); 4]) -> bool {
    let (px, py) = p;
    let mut inside = false;
    let mut j = poly.len() - 1;
    for i in 0..poly.len() {
        let (xi, yi) = poly[i];
        let (xj, yj) = poly[j];
        let denom = yj - yi;
        if denom.abs() < 1.0e-12 {
            j = i;
            continue;
        }
        let intersects = ((yi > py) != (yj > py)) && (px < (xj - xi) * (py - yi) / denom + xi);
        if intersects {
            inside = !inside;
        }
        j = i;
    }
    inside
}

fn snap_camera_rotation(current_rot: Quat, dir_world: Vec3, up_hint: Vec3) -> Quat {
    let dir = dir_world.normalize_or_zero();

    let current_up = (current_rot * Vec3::Y).normalize_or_zero();
    let mut up = current_up - dir * current_up.dot(dir);
    if up.length_squared() < 1.0e-6 {
        up = up_hint - dir * up_hint.dot(dir);
    }
    if up.length_squared() < 1.0e-6 {
        let alt = if dir.dot(Vec3::Z).abs() < 0.9 {
            Vec3::Z
        } else {
            Vec3::Y
        };
        up = alt - dir * alt.dot(dir);
    }
    up = up.normalize_or_zero();

    let mut right = up.cross(dir);
    if right.length_squared() < 1.0e-6 {
        right = Vec3::X;
    }
    right = right.normalize_or_zero();
    let up = dir.cross(right).normalize_or_zero();

    Quat::from_mat3(&Mat3::from_cols(right, up, dir)).normalize()
}

fn attach_editor_controls(
    canvas_el: web_sys::HtmlCanvasElement,
    viewcube_el: web_sys::HtmlCanvasElement,
    scene: Rc<RefCell<GeomScene>>,
    renderer: Rc<RefCell<Option<Renderer>>>,
    tool_mode: ReadSignal<EditorTool>,
    set_tool_mode: WriteSignal<EditorTool>,
    set_selected_id: WriteSignal<Option<ObjectId>>,
    selected_id: ReadSignal<Option<ObjectId>>,
    set_baseline_transform: WriteSignal<Option<Transform>>,
    set_transform_ui: WriteSignal<TransformUi>,
    drag_state: Rc<RefCell<Option<DragState>>>,
) {
    let viewcube_state = ViewCubeState::new(viewcube_el.clone());
    viewcube_state.draw_now(&renderer);

    let overlay_refresh_pending = Rc::new(RefCell::new(false));
    let request_overlay_refresh = {
        let scene = scene.clone();
        let renderer = renderer.clone();
        let selected_id = selected_id;
        let tool_mode = tool_mode;
        let overlay_refresh_pending = overlay_refresh_pending.clone();
        Rc::new(move || {
            if *overlay_refresh_pending.borrow() {
                return;
            }
            *overlay_refresh_pending.borrow_mut() = true;

            let scene = scene.clone();
            let renderer = renderer.clone();
            let overlay_refresh_pending = overlay_refresh_pending.clone();
            request_animation_frame(move || {
                *overlay_refresh_pending.borrow_mut() = false;
                let selected = selected_id.get_untracked();
                if selected.is_none() {
                    return;
                }
                let show_gizmo = tool_mode.get_untracked() == EditorTool::Move;
                update_overlay(&scene, &renderer, selected, show_gizmo);
            });
        })
    };

    let request_viewcube_refresh = {
        let renderer = renderer.clone();
        let viewcube_state = viewcube_state.clone();
        Rc::new(move || {
            viewcube_state.request_draw(&renderer);
        })
    };

    // Mousedown on canvas (LMB)
    {
        let canvas_for_closure = canvas_el.clone();
        let canvas_for_listener = canvas_el.clone();
        let scene = scene.clone();
        let renderer = renderer.clone();
        let drag_state = drag_state.clone();
        let closure = Closure::wrap(Box::new(move |event: web_sys::Event| {
            let event = event.dyn_into::<MouseEvent>().unwrap();
            if event.button() != 0 {
                return;
            }
            let (ray_o, ray_d, gizmo_hit) = {
                let renderer_borrow = renderer.borrow();
                let Some(r) = renderer_borrow.as_ref() else {
                    return;
                };

                let (cursor_x, cursor_y, w, h) = canvas_cursor(&canvas_for_closure, &event);
                let (ray_o, ray_d) = r.screen_ray(cursor_x, cursor_y, w, h);
                let ray_o = Vec3::from_array(ray_o);
                let ray_d = Vec3::from_array(ray_d);

                let gizmo_hit = if tool_mode.get_untracked() == EditorTool::Move {
                    selected_id
                        .get_untracked()
                        .and_then(|id| hit_gizmo(&scene, r, id, ray_o, ray_d).map(|hit| (id, hit)))
                } else {
                    None
                };
                (ray_o, ray_d, gizmo_hit)
            };

            if let Some((id, (mode, start_axis_t, plane_n, axis_dir_world, u, v, ang0))) = gizmo_hit
            {
                event.prevent_default();
                let start_transform = scene
                    .borrow()
                    .object_transform(id)
                    .unwrap_or_else(Transform::default);
                let start_origin_world = Vec3::from_array(start_transform.translation);
                *drag_state.borrow_mut() = Some(DragState {
                    object_id: id,
                    mode,
                    start_transform,
                    start_origin_world,
                    axis_dir_world,
                    plane_normal_world: plane_n,
                    start_axis_t,
                    ring_u_world: u,
                    ring_v_world: v,
                    start_angle: ang0,
                });
                return;
            }

            // Pick object by bounding sphere.
            if let Some(hit) = pick_object(&scene, ray_o, ray_d) {
                event.prevent_default();
                set_selected_id.set(Some(hit));
                if let Some(t) = scene.borrow().object_transform(hit) {
                    set_baseline_transform.set(Some(t));
                    set_transform_ui.set(TransformUi::from_transform(t));
                }
            } else {
                set_selected_id.set(None);
                set_baseline_transform.set(None);
            }
        }) as Box<dyn FnMut(_)>);
        let _ = canvas_for_listener
            .add_event_listener_with_callback("mousedown", closure.as_ref().unchecked_ref());
        closure.forget();
    }

    // Mouse move / up on window while dragging.
    if let Some(window) = web_sys::window() {
        // Refresh overlay on camera moves (MMB drag) and zoom (wheel).
        {
            let request_overlay_refresh = request_overlay_refresh.clone();
            let request_viewcube_refresh = request_viewcube_refresh.clone();
            let drag_state = drag_state.clone();
            let closure = Closure::wrap(Box::new(move |event: web_sys::Event| {
                let event = event.dyn_into::<MouseEvent>().unwrap();
                // MMB pressed while moving -> camera pan/orbit in renderer controls.
                if (event.buttons() & 4) == 0 {
                    return;
                }
                // If we're dragging the gizmo with LMB, we already refresh overlay there.
                if drag_state.borrow().is_some() {
                    return;
                }
                (request_overlay_refresh.as_ref())();
                (request_viewcube_refresh.as_ref())();
            }) as Box<dyn FnMut(_)>);
            let _ = window
                .add_event_listener_with_callback("mousemove", closure.as_ref().unchecked_ref());
            closure.forget();
        }

        {
            let request_overlay_refresh = request_overlay_refresh.clone();
            let closure = Closure::wrap(Box::new(move |_event: web_sys::Event| {
                (request_overlay_refresh.as_ref())();
            }) as Box<dyn FnMut(_)>);
            let _ = canvas_el
                .add_event_listener_with_callback("wheel", closure.as_ref().unchecked_ref());
            closure.forget();
        }

        {
            let request_viewcube_refresh = request_viewcube_refresh.clone();
            let closure = Closure::wrap(Box::new(move |_event: web_sys::Event| {
                (request_viewcube_refresh.as_ref())();
            }) as Box<dyn FnMut(_)>);
            let _ = canvas_el
                .add_event_listener_with_callback("wheel", closure.as_ref().unchecked_ref());
            closure.forget();
        }

        // Move
        {
            let canvas_el = canvas_el.clone();
            let scene = scene.clone();
            let renderer = renderer.clone();
            let drag_state = drag_state.clone();
            let viewcube_state = viewcube_state.clone();
            let closure = Closure::wrap(Box::new(move |event: web_sys::Event| {
                let event = event.dyn_into::<MouseEvent>().unwrap();
                let Some(ds) = *drag_state.borrow() else {
                    return;
                };
                let (ray_o, ray_d) = {
                    let renderer_borrow = renderer.borrow();
                    let Some(r) = renderer_borrow.as_ref() else {
                        return;
                    };
                    let (cursor_x, cursor_y, w, h) = canvas_cursor(&canvas_el, &event);
                    r.screen_ray(cursor_x, cursor_y, w, h)
                };
                let ray_o = Vec3::from_array(ray_o);
                let ray_d = Vec3::from_array(ray_d);

                let new_t = match ds.mode {
                    DragMode::Translate => {
                        if let Some(t) = drag_translate(ds, ray_o, ray_d) {
                            t
                        } else {
                            return;
                        }
                    }
                    DragMode::Rotate(axis) => {
                        if let Some(t) = drag_rotate(ds, axis, ray_o, ray_d) {
                            t
                        } else {
                            return;
                        }
                    }
                };

                apply_transform(&scene, &renderer, ds.object_id, new_t);
                set_transform_ui.set(TransformUi::from_transform(new_t));
                update_overlay(
                    &scene,
                    &renderer,
                    Some(ds.object_id),
                    tool_mode.get_untracked() == EditorTool::Move,
                );
                viewcube_state.request_draw(&renderer);
            }) as Box<dyn FnMut(_)>);
            let _ = window
                .add_event_listener_with_callback("mousemove", closure.as_ref().unchecked_ref());
            closure.forget();
        }

        // Up
        {
            let drag_state = drag_state.clone();
            let closure = Closure::wrap(Box::new(move |event: web_sys::Event| {
                let event = event.dyn_into::<MouseEvent>().unwrap();
                if event.button() == 0 {
                    *drag_state.borrow_mut() = None;
                }
            }) as Box<dyn FnMut(_)>);
            let _ = window
                .add_event_listener_with_callback("mouseup", closure.as_ref().unchecked_ref());
            closure.forget();
        }

        // Keyboard shortcuts
        {
            let closure = Closure::wrap(Box::new(move |event: web_sys::Event| {
                let event = event.dyn_into::<KeyboardEvent>().unwrap();

                if event.repeat() {
                    return;
                }

                if let Some(document) = web_sys::window().and_then(|w| w.document()) {
                    if let Some(active) = document.active_element() {
                        let tag = active.tag_name().to_ascii_uppercase();
                        if tag == "INPUT" || tag == "TEXTAREA" {
                            return;
                        }
                    }
                }

                let key = event.key();
                if key == "m" || key == "M" {
                    event.prevent_default();
                    set_tool_mode.set(EditorTool::Move);
                } else if key == "Escape" {
                    event.prevent_default();
                    set_tool_mode.set(EditorTool::None);
                }
            }) as Box<dyn FnMut(_)>);
            let _ = window
                .add_event_listener_with_callback("keydown", closure.as_ref().unchecked_ref());
            closure.forget();
        }
    }

    // ViewCube dblclick: snap camera to face.
    {
        let renderer = renderer.clone();
        let request_overlay_refresh = request_overlay_refresh.clone();
        let request_viewcube_refresh = request_viewcube_refresh.clone();
        let viewcube_state = viewcube_state.clone();
        let viewcube_for_cursor = viewcube_el.clone();
        let closure = Closure::wrap(Box::new(move |event: web_sys::Event| {
            let event = event.dyn_into::<MouseEvent>().unwrap();
            event.prevent_default();
            let (cursor_x, cursor_y, _w, _h) = canvas_cursor(&viewcube_for_cursor, &event);
            let Some(face) = viewcube_state.hit_face(cursor_x as f64, cursor_y as f64) else {
                return;
            };

            let (dir, up_hint) = face.snap_vectors();
            let mut renderer_borrow = renderer.borrow_mut();
            let Some(r) = renderer_borrow.as_mut() else {
                return;
            };
            let current_rot = Quat::from_array(r.camera_rotation()).normalize();
            let snapped = snap_camera_rotation(current_rot, dir, up_hint);
            r.set_camera_rotation(snapped.to_array());
            r.render();

            (request_overlay_refresh.as_ref())();
            (request_viewcube_refresh.as_ref())();
        }) as Box<dyn FnMut(_)>);
        let _ = viewcube_el
            .add_event_listener_with_callback("dblclick", closure.as_ref().unchecked_ref());
        closure.forget();
    }
}

fn apply_transform(
    scene: &Rc<RefCell<GeomScene>>,
    renderer: &Rc<RefCell<Option<Renderer>>>,
    id: ObjectId,
    transform: Transform,
) {
    let mesh = {
        let mut scene = scene.borrow_mut();
        let _ = scene.set_object_transform(id, transform);
        match scene.mesh() {
            Ok(mesh) => mesh,
            Err(err) => {
                log(&format!("tessellation failed: {err}"));
                return;
            }
        }
    };
    if let Some(renderer) = renderer.borrow_mut().as_mut() {
        renderer.set_mesh(mesh);
        renderer.render();
    }
}

fn gizmo_dimensions(base_r: f32, dist_to_obj: f32) -> (f32, f32) {
    let dist_to_obj = dist_to_obj.max(0.001);
    let axis_len = (dist_to_obj * 0.12).max(base_r * 0.25);
    let ring_r = axis_len * 0.75;
    (axis_len, ring_r)
}

fn update_overlay(
    scene: &Rc<RefCell<GeomScene>>,
    renderer: &Rc<RefCell<Option<Renderer>>>,
    selected: Option<ObjectId>,
    show_gizmo: bool,
) {
    let mut renderer_borrow = renderer.borrow_mut();
    let Some(renderer) = renderer_borrow.as_mut() else {
        return;
    };
    let Some(id) = selected else {
        renderer.clear_overlay_lines();
        renderer.render();
        return;
    };
    let scene_ref = scene.borrow();
    let Some(t) = scene_ref.object_transform(id) else {
        renderer.clear_overlay_lines();
        renderer.render();
        return;
    };

    let origin = Vec3::from_array(t.translation);
    let rot = quat_from_transform(t);
    let (eye, _target) = renderer.camera_eye_target();
    let eye = Vec3::from_array(eye);
    let to_camera = (eye - origin).normalize_or_zero();
    let mut lines = Vec::new();
    // Selection highlight (oriented local AABB).
    if let Some(aabb) = scene_ref.local_aabb(id) {
        add_aabb_wireframe(&mut lines, origin, rot, aabb, [1.0, 0.85, 0.25]);
    }

    if show_gizmo {
        let axis_x = (rot * Vec3::X).normalize();
        let axis_y = (rot * Vec3::Y).normalize();
        let axis_z = (rot * Vec3::Z).normalize();

        let base_r = scene_ref.bounds_radius(id).unwrap_or(1.0).max(0.25);
        let dist_to_obj = (eye - origin).length().max(0.001);
        let (axis_len, ring_r) = gizmo_dimensions(base_r, dist_to_obj);

        // Translation axes
        lines.push(OverlayLine {
            a: origin.to_array(),
            b: (origin + axis_x * axis_len).to_array(),
            color: [1.0, 0.25, 0.25],
        });
        add_axis_arrow(
            &mut lines,
            origin,
            axis_x,
            axis_len,
            to_camera,
            [1.0, 0.25, 0.25],
        );
        lines.push(OverlayLine {
            a: origin.to_array(),
            b: (origin + axis_y * axis_len).to_array(),
            color: [0.25, 1.0, 0.25],
        });
        add_axis_arrow(
            &mut lines,
            origin,
            axis_y,
            axis_len,
            to_camera,
            [0.25, 1.0, 0.25],
        );
        lines.push(OverlayLine {
            a: origin.to_array(),
            b: (origin + axis_z * axis_len).to_array(),
            color: [0.35, 0.55, 1.0],
        });
        add_axis_arrow(
            &mut lines,
            origin,
            axis_z,
            axis_len,
            to_camera,
            [0.35, 0.55, 1.0],
        );

        // Rotation rings (visual only + used for picking)
        add_ring(
            &mut lines,
            origin,
            axis_y,
            axis_z,
            ring_r,
            [1.0, 0.25, 0.25],
        );
        add_ring_arrow(
            &mut lines,
            origin,
            axis_x,
            axis_y,
            axis_z,
            ring_r,
            to_camera,
            [1.0, 0.25, 0.25],
        );
        add_ring(
            &mut lines,
            origin,
            axis_z,
            axis_x,
            ring_r,
            [0.25, 1.0, 0.25],
        );
        add_ring_arrow(
            &mut lines,
            origin,
            axis_y,
            axis_z,
            axis_x,
            ring_r,
            to_camera,
            [0.25, 1.0, 0.25],
        );
        add_ring(
            &mut lines,
            origin,
            axis_x,
            axis_y,
            ring_r,
            [0.35, 0.55, 1.0],
        );
        add_ring_arrow(
            &mut lines,
            origin,
            axis_z,
            axis_x,
            axis_y,
            ring_r,
            to_camera,
            [0.35, 0.55, 1.0],
        );
    }

    renderer.set_overlay_lines(lines);
    renderer.render();
}

fn add_axis_arrow(
    lines: &mut Vec<OverlayLine>,
    origin: Vec3,
    axis_dir: Vec3,
    axis_len: f32,
    to_camera: Vec3,
    color: [f32; 3],
) {
    let tip = origin + axis_dir * axis_len;
    let arrow_len = axis_len * 0.18;
    let mut side = axis_dir.cross(to_camera);
    if side.length_squared() < 1.0e-10 {
        side = axis_dir.cross(Vec3::Y);
        if side.length_squared() < 1.0e-10 {
            side = axis_dir.cross(Vec3::X);
        }
    }
    let side = side.normalize_or_zero();
    let wing = arrow_len * 0.45;
    let base = tip - axis_dir * arrow_len;
    let left = base + side * wing;
    let right = base - side * wing;
    lines.push(OverlayLine {
        a: tip.to_array(),
        b: left.to_array(),
        color,
    });
    lines.push(OverlayLine {
        a: tip.to_array(),
        b: right.to_array(),
        color,
    });
}

fn add_ring_arrow(
    lines: &mut Vec<OverlayLine>,
    origin: Vec3,
    ring_normal: Vec3,
    ring_u: Vec3,
    ring_v: Vec3,
    radius: f32,
    to_camera: Vec3,
    color: [f32; 3],
) {
    // Place the arrow on the side of the ring facing the camera.
    let mut dir = to_camera - ring_normal * to_camera.dot(ring_normal);
    if dir.length_squared() < 1.0e-10 {
        dir = ring_u;
    }
    dir = dir.normalize_or_zero();
    let a0 = dir.dot(ring_v).atan2(dir.dot(ring_u));

    let p = origin + (ring_u * a0.cos() + ring_v * a0.sin()) * radius;
    let tangent = (-ring_u * a0.sin() + ring_v * a0.cos()).normalize_or_zero();
    let arrow_len = radius * 0.30;
    let base = p - tangent * (arrow_len * 0.25);
    let tip = p + tangent * (arrow_len * 0.75);

    let wing_dir = ring_normal.cross(tangent).normalize_or_zero();
    let wing_len = arrow_len * 0.35;
    let wing_base = tip - tangent * (arrow_len * 0.35);
    let left = wing_base + wing_dir * wing_len;
    let right = wing_base - wing_dir * wing_len;

    lines.push(OverlayLine {
        a: base.to_array(),
        b: tip.to_array(),
        color,
    });
    lines.push(OverlayLine {
        a: tip.to_array(),
        b: left.to_array(),
        color,
    });
    lines.push(OverlayLine {
        a: tip.to_array(),
        b: right.to_array(),
        color,
    });
}

fn add_aabb_wireframe(
    lines: &mut Vec<OverlayLine>,
    origin: Vec3,
    rot: Quat,
    aabb: cad_geom::Aabb,
    color: [f32; 3],
) {
    let min = Vec3::from_array(aabb.min);
    let max = Vec3::from_array(aabb.max);
    let corners = [
        Vec3::new(min.x, min.y, min.z),
        Vec3::new(max.x, min.y, min.z),
        Vec3::new(max.x, max.y, min.z),
        Vec3::new(min.x, max.y, min.z),
        Vec3::new(min.x, min.y, max.z),
        Vec3::new(max.x, min.y, max.z),
        Vec3::new(max.x, max.y, max.z),
        Vec3::new(min.x, max.y, max.z),
    ]
    .map(|p| origin + rot * p);

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
        lines.push(OverlayLine {
            a: corners[a].to_array(),
            b: corners[b].to_array(),
            color,
        });
    }
}

fn add_ring(
    lines: &mut Vec<OverlayLine>,
    origin: Vec3,
    u: Vec3,
    v: Vec3,
    radius: f32,
    color: [f32; 3],
) {
    let segs = 48;
    let mut prev = origin + u * radius;
    for i in 1..=segs {
        let a = (i as f32 / segs as f32) * std::f32::consts::TAU;
        let p = origin + (u * a.cos() + v * a.sin()) * radius;
        lines.push(OverlayLine {
            a: prev.to_array(),
            b: p.to_array(),
            color,
        });
        prev = p;
    }
}

fn pick_object(scene: &Rc<RefCell<GeomScene>>, ray_o: Vec3, ray_d: Vec3) -> Option<ObjectId> {
    let scene_ref = scene.borrow();
    let mut best_t = f32::INFINITY;
    let mut best_id = None;
    for obj in scene_ref.model().objects() {
        let t = obj.transform;
        let center = Vec3::from_array(t.translation);
        let radius = scene_ref.bounds_radius(obj.id).unwrap_or(0.5).max(0.05);
        if let Some(hit_t) = ray_sphere_intersect(ray_o, ray_d, center, radius) {
            if hit_t < best_t {
                best_t = hit_t;
                best_id = Some(obj.id);
            }
        }
    }
    best_id
}

fn hit_gizmo(
    scene: &Rc<RefCell<GeomScene>>,
    renderer: &Renderer,
    id: ObjectId,
    ray_o: Vec3,
    ray_d: Vec3,
) -> Option<(DragMode, f32, Vec3, Vec3, Vec3, Vec3, f32)> {
    let Some(t) = scene.borrow().object_transform(id) else {
        return None;
    };
    let origin = Vec3::from_array(t.translation);
    let rot = quat_from_transform(t);
    let axis_x = (rot * Vec3::X).normalize();
    let axis_y = (rot * Vec3::Y).normalize();
    let axis_z = (rot * Vec3::Z).normalize();

    let base_r = scene.borrow().bounds_radius(id).unwrap_or(1.0).max(0.25);
    let (eye, _target) = renderer.camera_eye_target();
    let eye = Vec3::from_array(eye);
    let view_dir = (origin - eye).normalize_or_zero();
    let dist_to_obj = (eye - origin).length().max(0.001);
    let (axis_len, ring_r) = gizmo_dimensions(base_r, dist_to_obj);

    let threshold = (axis_len * 0.18).max(dist_to_obj * 0.015).max(0.05);

    // Axis hit test
    let axes = [(Axis::X, axis_x), (Axis::Y, axis_y), (Axis::Z, axis_z)];
    let mut best_axis = None;
    let mut best_dist = f32::INFINITY;
    let mut best_t_axis = 0.0;
    for (ax, dir) in axes {
        let a = origin;
        let b = origin + dir * axis_len;
        let (dist, t_seg) = ray_segment_distance(ray_o, ray_d, a, b);
        if dist < threshold && dist < best_dist {
            best_dist = dist;
            best_axis = Some((ax, dir));
            best_t_axis = t_seg;
        }
    }
    if let Some((_axis, dir)) = best_axis {
        let mut plane_n = dir.cross(view_dir).cross(dir);
        if plane_n.length_squared() < 1.0e-10 {
            plane_n = dir.cross(Vec3::Y).cross(dir);
        }
        plane_n = plane_n.normalize_or_zero();
        let hit_point = origin + dir * best_t_axis;
        let start_axis_t = dir.dot(hit_point - origin);
        return Some((
            DragMode::Translate,
            start_axis_t,
            plane_n,
            dir,
            Vec3::ZERO,
            Vec3::ZERO,
            0.0,
        ));
    }

    // Ring hit test (plane intersection + radius check)
    let ring_threshold = (ring_r * 0.20).max(dist_to_obj * 0.015).max(0.05);
    let rings = [
        (Axis::X, axis_x, axis_y, axis_z, [1.0, 0.25, 0.25]),
        (Axis::Y, axis_y, axis_z, axis_x, [0.25, 1.0, 0.25]),
        (Axis::Z, axis_z, axis_x, axis_y, [0.35, 0.55, 1.0]),
    ];
    for (axis, n, u, v, _c) in rings {
        let denom = n.dot(ray_d);
        if denom.abs() < 1.0e-6 {
            continue;
        }
        let t_hit = n.dot(origin - ray_o) / denom;
        if t_hit <= 0.0 {
            continue;
        }
        let p = ray_o + ray_d * t_hit;
        let r = (p - origin).length();
        if (r - ring_r).abs() <= ring_threshold {
            let vdir = (p - origin).normalize_or_zero();
            let ang0 = vdir.dot(v).atan2(vdir.dot(u));
            return Some((DragMode::Rotate(axis), 0.0, n, n, u, v, ang0));
        }
    }

    None
}

fn drag_translate(ds: DragState, ray_o: Vec3, ray_d: Vec3) -> Option<Transform> {
    let denom = ds.plane_normal_world.dot(ray_d);
    if denom.abs() < 1.0e-6 {
        return None;
    }
    let t = ds.plane_normal_world.dot(ds.start_origin_world - ray_o) / denom;
    let p = ray_o + ray_d * t;
    let axis_t = ds.axis_dir_world.dot(p - ds.start_origin_world);
    let delta = axis_t - ds.start_axis_t;

    let mut out = ds.start_transform;
    let start = Vec3::from_array(ds.start_transform.translation);
    let next = start + ds.axis_dir_world * delta;
    out.translation = next.to_array();
    Some(out)
}

fn drag_rotate(ds: DragState, axis: Axis, ray_o: Vec3, ray_d: Vec3) -> Option<Transform> {
    let n = ds.plane_normal_world;
    let denom = n.dot(ray_d);
    if denom.abs() < 1.0e-6 {
        return None;
    }
    let t = n.dot(ds.start_origin_world - ray_o) / denom;
    if t <= 0.0 {
        return None;
    }
    let p = ray_o + ray_d * t;
    let vdir = (p - ds.start_origin_world).normalize_or_zero();
    let angle = vdir.dot(ds.ring_v_world).atan2(vdir.dot(ds.ring_u_world));
    let mut delta = angle - ds.start_angle;
    if delta > std::f32::consts::PI {
        delta -= std::f32::consts::TAU;
    } else if delta < -std::f32::consts::PI {
        delta += std::f32::consts::TAU;
    }

    let start_q = quat_from_transform(ds.start_transform);
    let axis_local = match axis {
        Axis::X => Vec3::X,
        Axis::Y => Vec3::Y,
        Axis::Z => Vec3::Z,
    };
    let q_local = Quat::from_axis_angle(axis_local, delta);
    let q = (start_q * q_local).normalize();

    let mut out = ds.start_transform;
    out.rotation = [q.x, q.y, q.z, q.w];
    Some(out)
}

fn ray_sphere_intersect(ray_o: Vec3, ray_d: Vec3, center: Vec3, radius: f32) -> Option<f32> {
    let oc = ray_o - center;
    let b = oc.dot(ray_d);
    let c = oc.dot(oc) - radius * radius;
    let disc = b * b - c;
    if disc < 0.0 {
        return None;
    }
    let t = -b - disc.sqrt();
    if t > 0.0 {
        Some(t)
    } else {
        None
    }
}

fn ray_segment_distance(ray_o: Vec3, ray_d: Vec3, a: Vec3, b: Vec3) -> (f32, f32) {
    // Closest points between ray (o + s*d, s>=0) and segment (a + t*(b-a), t in [0,1]).
    // Based on clamped closest-point solution (Ericson, RTCD-style).
    let u = ray_d;
    let v = b - a;
    let w = ray_o - a;

    let a_ = u.dot(u);
    let b_ = u.dot(v);
    let c_ = v.dot(v);
    let d_ = u.dot(w);
    let e_ = v.dot(w);
    let det = a_ * c_ - b_ * b_;

    let mut s;
    let mut t;

    if det > 1.0e-8 {
        // Unclamped solution.
        s = (b_ * e_ - c_ * d_) / det;
        t = (a_ * e_ - b_ * d_) / det;
    } else {
        // Nearly parallel: take s = 0 (ray origin) and project onto segment.
        s = 0.0;
        t = if c_ > 1.0e-12 { e_ / c_ } else { 0.0 };
    }

    // Clamp t to [0,1] (segment).
    if t < 0.0 {
        t = 0.0;
        s = -d_ / a_;
    } else if t > 1.0 {
        t = 1.0;
        s = (b_ - d_) / a_;
    }

    // Clamp s to ray (s >= 0). If clamped, recompute t as closest point on segment to ray origin.
    if s < 0.0 {
        s = 0.0;
        t = if c_ > 1.0e-12 { e_ / c_ } else { 0.0 };
        t = t.clamp(0.0, 1.0);
    }

    let p_ray = ray_o + u * s;
    let p_seg = a + v * t;
    let dist = (p_ray - p_seg).length();
    (dist, t * v.length())
}

fn canvas_cursor(canvas: &web_sys::HtmlCanvasElement, event: &MouseEvent) -> (f32, f32, f32, f32) {
    let rect = canvas.get_bounding_client_rect();
    let left = rect.left() as f32;
    let top = rect.top() as f32;
    let x = event.client_x() as f32 - left;
    let y = event.client_y() as f32 - top;
    (
        x,
        y,
        canvas.client_width() as f32,
        canvas.client_height() as f32,
    )
}

fn quat_from_transform(transform: Transform) -> Quat {
    Quat::from_xyzw(
        transform.rotation[0],
        transform.rotation[1],
        transform.rotation[2],
        transform.rotation[3],
    )
    .normalize()
}

fn update_mesh(scene: &Rc<RefCell<GeomScene>>, renderer: &Rc<RefCell<Option<Renderer>>>) {
    let mesh = match scene.borrow_mut().mesh() {
        Ok(mesh) => mesh,
        Err(err) => {
            log(&format!("tessellation failed: {err}"));
            return;
        }
    };
    if let Some(renderer) = renderer.borrow_mut().as_mut() {
        renderer.set_mesh(mesh);
        renderer.render();
    }
}

fn schedule_renderer_init(
    canvas_ref: NodeRef<Canvas>,
    renderer: Rc<RefCell<Option<Renderer>>>,
    set_renderer_ready: WriteSignal<bool>,
    plane_xy: ReadSignal<bool>,
    plane_yz: ReadSignal<bool>,
    plane_zx: ReadSignal<bool>,
) {
    let renderer = renderer.clone();
    let set_renderer_ready = set_renderer_ready.clone();
    let plane_xy = plane_xy.clone();
    let plane_yz = plane_yz.clone();
    let plane_zx = plane_zx.clone();
    request_animation_frame(move || {
        if let Some(canvas) = canvas_ref.get() {
            let renderer = renderer.clone();
            let set_renderer_ready = set_renderer_ready;
            spawn_local(async move {
                match Renderer::new(canvas.clone()).await {
                    Ok(mut r) => {
                        r.attach_default_controls(&canvas);
                        r.set_plane_visibility(
                            plane_xy.get_untracked(),
                            plane_yz.get_untracked(),
                            plane_zx.get_untracked(),
                        );
                        r.render();
                        *renderer.borrow_mut() = Some(r);
                        set_renderer_ready.set(true);
                    }
                    Err(err) => {
                        log(&format!("renderer init failed: {err}"));
                    }
                }
            });
        } else {
            // Canvas not ready yet, try again on the next frame.
            schedule_renderer_init(
                canvas_ref,
                renderer,
                set_renderer_ready,
                plane_xy,
                plane_yz,
                plane_zx,
            );
        }
    });
}

fn connect_ws(handle: Rc<RefCell<Option<WebSocket>>>) {
    let window = match web_sys::window() {
        Some(window) => window,
        None => return,
    };
    let location = window.location();
    let hostname = location
        .hostname()
        .unwrap_or_else(|_| "localhost".to_string());
    let port = location.port().unwrap_or_default();
    let host = if port == "8080" || (hostname != "localhost" && hostname != "127.0.0.1") {
        if port.is_empty() {
            hostname
        } else {
            format!("{hostname}:{port}")
        }
    } else {
        format!("{hostname}:8080")
    };
    let protocol = location.protocol().unwrap_or_else(|_| "http:".to_string());
    let scheme = if protocol == "https:" { "wss" } else { "ws" };
    let url = format!("{scheme}://{host}/ws");

    let ws = match WebSocket::new(&url) {
        Ok(ws) => ws,
        Err(err) => {
            log(&format!("ws init failed: {err:?}"));
            return;
        }
    };

    let ws_open = ws.clone();
    let onopen = Closure::wrap(Box::new(move |_event: web_sys::Event| {
        let msg = ClientMsg::Hello {
            client_version: env!("CARGO_PKG_VERSION").to_string(),
        };
        if let Ok(text) = serde_json::to_string(&msg) {
            let _ = ws_open.send_with_str(&text);
        }
    }) as Box<dyn FnMut(_)>);
    ws.set_onopen(Some(onopen.as_ref().unchecked_ref()));
    onopen.forget();

    let onmessage = Closure::wrap(Box::new(move |event: MessageEvent| {
        if let Some(text) = event.data().as_string() {
            if let Ok(msg) = serde_json::from_str::<ServerMsg>(&text) {
                log(&format!("server: {msg:?}"));
            } else {
                log(&format!("ws message: {text}"));
            }
        }
    }) as Box<dyn FnMut(_)>);
    ws.set_onmessage(Some(onmessage.as_ref().unchecked_ref()));
    onmessage.forget();

    let onclose = Closure::wrap(Box::new(move |_event: web_sys::CloseEvent| {
        log("ws closed");
    }) as Box<dyn FnMut(_)>);
    ws.set_onclose(Some(onclose.as_ref().unchecked_ref()));
    onclose.forget();

    *handle.borrow_mut() = Some(ws);
}

fn log(text: &str) {
    web_sys::console::log_1(&text.into());
}

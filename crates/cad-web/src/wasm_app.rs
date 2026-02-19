use crate::ui_icons::{IconName, UiIcon};
use cad_core::{ObjectId, Transform};
use cad_geom::{GeomScene, SurfaceHit};
use cad_protocol::{ClientMsg, ServerMsg};
use cad_render::{OverlayLine, Renderer};
use glam::{EulerRot, Mat3, Quat, Vec3};
use js_sys::Date;
use leptos::html::Canvas;
use leptos::prelude::*;
use std::cell::RefCell;
use std::rc::Rc;
use wasm_bindgen::prelude::*;
use wasm_bindgen::{closure::Closure, JsCast};
use wasm_bindgen_futures::spawn_local;
use web_sys::{
    CanvasRenderingContext2d, HtmlInputElement, KeyboardEvent, MessageEvent, MouseEvent, WebSocket,
};

#[wasm_bindgen(start)]
pub fn start() {
    console_error_panic_hook::set_once();
    mount_to_body(|| view! { <App /> });
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum UiLogLevel {
    Success,
    Warning,
    Info,
}

#[derive(Clone)]
struct UiLogEntry {
    level: UiLogLevel,
    message: String,
    timestamp: String,
}

#[derive(Clone, Copy)]
struct UiCommand {
    id: &'static str,
    label: &'static str,
    category: &'static str,
    shortcut: Option<&'static str>,
}

#[derive(Clone, Copy)]
struct UiShortcut {
    keys: &'static [&'static str],
    description: &'static str,
    category: &'static str,
}

const TOP_TABS: [&str; 5] = ["Model", "Surface", "Mesh", "Sheet", "Tools"];

const UI_COMMANDS: [UiCommand; 10] = [
    UiCommand {
        id: "box",
        label: "Create Box",
        category: "Create",
        shortcut: Some("B"),
    },
    UiCommand {
        id: "sphere",
        label: "Create Sphere",
        category: "Create",
        shortcut: Some("S"),
    },
    UiCommand {
        id: "extrude",
        label: "Extrude",
        category: "Modify",
        shortcut: Some("E"),
    },
    UiCommand {
        id: "move",
        label: "Move",
        category: "Modify",
        shortcut: Some("M"),
    },
    UiCommand {
        id: "rotate",
        label: "Rotate",
        category: "Modify",
        shortcut: Some("R"),
    },
    UiCommand {
        id: "scale",
        label: "Scale",
        category: "Modify",
        shortcut: Some("Ctrl+S"),
    },
    UiCommand {
        id: "measure",
        label: "Measure Distance",
        category: "Inspect",
        shortcut: Some("Ctrl+M"),
    },
    UiCommand {
        id: "section",
        label: "Section Analysis",
        category: "Inspect",
        shortcut: None,
    },
    UiCommand {
        id: "import",
        label: "Import File",
        category: "File",
        shortcut: Some("Ctrl+I"),
    },
    UiCommand {
        id: "export",
        label: "Export Model",
        category: "File",
        shortcut: Some("Ctrl+E"),
    },
];

const TIMELINE_FEATURES: [(&str, &str, &str); 10] = [
    ("f1", "01", "Sketch"),
    ("f2", "02", "Extrude"),
    ("f3", "03", "Fillet"),
    ("f4", "04", "Chamfer"),
    ("f5", "05", "Shell"),
    ("f6", "06", "Pattern"),
    ("f7", "07", "Mirror"),
    ("f8", "08", "Thread"),
    ("f9", "09", "Hole"),
    ("f10", "10", "Extrude Cut"),
];

const UI_SHORTCUTS: [UiShortcut; 12] = [
    UiShortcut {
        keys: &["Ctrl", "K"],
        description: "Open Command Palette",
        category: "General",
    },
    UiShortcut {
        keys: &["Ctrl", "N"],
        description: "New Document",
        category: "File",
    },
    UiShortcut {
        keys: &["Ctrl", "S"],
        description: "Save",
        category: "File",
    },
    UiShortcut {
        keys: &["Ctrl", "Z"],
        description: "Undo",
        category: "Edit",
    },
    UiShortcut {
        keys: &["Ctrl", "Y"],
        description: "Redo",
        category: "Edit",
    },
    UiShortcut {
        keys: &["B"],
        description: "Create Box",
        category: "Create",
    },
    UiShortcut {
        keys: &["S"],
        description: "Create Sphere",
        category: "Create",
    },
    UiShortcut {
        keys: &["E"],
        description: "Extrude",
        category: "Modify",
    },
    UiShortcut {
        keys: &["M"],
        description: "Move",
        category: "Modify",
    },
    UiShortcut {
        keys: &["R"],
        description: "Rotate",
        category: "Modify",
    },
    UiShortcut {
        keys: &["F"],
        description: "Fit View",
        category: "View",
    },
    UiShortcut {
        keys: &["Space"],
        description: "Pan View",
        category: "View",
    },
];

fn ui_time_hms() -> String {
    let now = Date::new_0();
    format!(
        "{:02}:{:02}:{:02}",
        now.get_hours() as u32,
        now.get_minutes() as u32,
        now.get_seconds() as u32
    )
}

fn command_icon(id: &str) -> IconName {
    match id {
        "box" => IconName::Box,
        "sphere" => IconName::Circle,
        "extrude" => IconName::Square,
        "move" => IconName::Move,
        "rotate" => IconName::RotateCw,
        "scale" => IconName::Scale,
        "measure" => IconName::Ruler,
        "section" => IconName::Eye,
        "import" => IconName::File,
        "export" => IconName::FileText,
        _ => IconName::Command,
    }
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
    let (object_ids, set_object_ids) = signal(Vec::<ObjectId>::new());

    let (tool_mode, set_tool_mode) = signal(EditorTool::None);
    let (selected_id, set_selected_id) = signal(None::<ObjectId>);
    let (baseline_transform, set_baseline_transform) = signal(None::<Transform>);
    let (transform_ui, set_transform_ui) = signal(TransformUi::default());
    let (sketch_plane, set_sketch_plane) = signal(None::<SketchPlane>);
    let (sketch_plane_name, set_sketch_plane_name) = signal(String::new());
    let (sketch_segments, set_sketch_segments) = signal(Vec::<SketchSegment>::new());
    let (sketch_anchor, set_sketch_anchor) = signal(None::<Vec3>);
    let (sketch_cursor, set_sketch_cursor) = signal(None::<Vec3>);
    let (saved_sketches, set_saved_sketches) = signal(Vec::<SavedSketch>::new());
    let (next_sketch_id, set_next_sketch_id) = signal(1usize);
    let (active_tab, set_active_tab) = signal("Model".to_string());
    let (active_tool, set_active_tool) = signal("select".to_string());
    let (active_feature, set_active_feature) = signal("f3".to_string());
    let (show_palette, set_show_palette) = signal(false);
    let (palette_query, set_palette_query) = signal(String::new());
    let (pending_command, set_pending_command) = signal(None::<String>);
    let (show_project_info, set_show_project_info) = signal(true);
    let (show_console, set_show_console) = signal(false);
    let (console_expanded, set_console_expanded) = signal(true);
    let (show_shortcuts, set_show_shortcuts) = signal(false);
    let (browser_selected, set_browser_selected) = signal("body-1".to_string());
    let (browser_search, set_browser_search) = signal(String::new());
    let (expand_origin, set_expand_origin) = signal(true);
    let (expand_sketches, set_expand_sketches) = signal(true);
    let (expand_bodies, set_expand_bodies) = signal(true);
    let (expand_components, set_expand_components) = signal(true);
    let (expand_component_1, set_expand_component_1) = signal(true);
    let (log_entries, set_log_entries) = signal(vec![
        UiLogEntry {
            level: UiLogLevel::Success,
            message: "Extrude operation completed".to_string(),
            timestamp: ui_time_hms(),
        },
        UiLogEntry {
            level: UiLogLevel::Info,
            message: "Sketch 1 created".to_string(),
            timestamp: ui_time_hms(),
        },
        UiLogEntry {
            level: UiLogLevel::Success,
            message: "Fillet applied to 4 edges".to_string(),
            timestamp: ui_time_hms(),
        },
    ]);
    let drag_state = Rc::new(RefCell::new(None::<DragState>));
    let editor_attached = Rc::new(RefCell::new(false));
    let palette_key_listener = Rc::new(RefCell::new(false));

    let push_log: Rc<dyn Fn(UiLogLevel, String)> = {
        let set_log_entries = set_log_entries;
        Rc::new(move |level, message| {
            let entry = UiLogEntry {
                level,
                message,
                timestamp: ui_time_hms(),
            };
            set_log_entries.update(|entries| {
                entries.insert(0, entry);
                if entries.len() > 50 {
                    entries.truncate(50);
                }
            });
        })
    };

    let enter_sketch_draw: Rc<dyn Fn(SketchPlane, String)> = {
        let renderer = renderer.clone();
        let set_tool_mode = set_tool_mode;
        let set_active_tool = set_active_tool;
        let set_sketch_plane = set_sketch_plane;
        let set_sketch_plane_name = set_sketch_plane_name;
        let set_sketch_segments = set_sketch_segments;
        let set_sketch_anchor = set_sketch_anchor;
        let set_sketch_cursor = set_sketch_cursor;
        let push_log = push_log.clone();
        Rc::new(move |plane, label| {
            set_sketch_plane.set(Some(plane));
            set_sketch_plane_name.set(label.clone());
            set_sketch_segments.set(Vec::new());
            set_sketch_anchor.set(None);
            set_sketch_cursor.set(None);
            set_tool_mode.set(EditorTool::SketchDraw);
            set_active_tool.set("sketch".to_string());
            animate_camera_to_sketch_plane(renderer.clone(), plane);
            (push_log.as_ref())(UiLogLevel::Info, format!("Sketch started on {label}"));
        })
    };

    {
        let palette_key_listener = palette_key_listener.clone();
        let set_show_palette = set_show_palette;
        Effect::new(move |_| {
            if *palette_key_listener.borrow() {
                return;
            }
            let Some(window) = web_sys::window() else {
                return;
            };
            let handler = Closure::wrap(Box::new(move |ev: KeyboardEvent| {
                if (ev.ctrl_key() || ev.meta_key()) && ev.key().eq_ignore_ascii_case("k") {
                    ev.prevent_default();
                    set_show_palette.update(|open| *open = !*open);
                    return;
                }
                if ev.key() == "Escape" {
                    set_show_palette.set(false);
                }
            }) as Box<dyn FnMut(_)>);
            let _ = window
                .add_event_listener_with_callback("keydown", handler.as_ref().unchecked_ref());
            handler.forget();
            *palette_key_listener.borrow_mut() = true;
        });
    }

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
        let enter_sketch_draw_for_controls = enter_sketch_draw.clone();
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
                sketch_plane,
                sketch_segments,
                set_sketch_segments,
                sketch_anchor,
                set_sketch_anchor,
                set_sketch_cursor,
                enter_sketch_draw_for_controls.clone(),
            );
            *editor_attached.borrow_mut() = true;
        });
    }

    let add_box_action: Rc<dyn Fn()> = {
        let scene = scene.clone();
        let renderer = renderer.clone();
        let set_object_count = set_object_count;
        let set_object_ids = set_object_ids;
        let set_selected_id = set_selected_id;
        let set_transform_ui = set_transform_ui;
        let set_baseline_transform = set_baseline_transform;
        let set_browser_selected = set_browser_selected;
        let set_active_tool = set_active_tool;
        let push_log = push_log.clone();
        Rc::new(move || {
            let id = {
                let mut scene = scene.borrow_mut();
                let id = scene.add_box(1.0, 1.0, 1.0);
                set_object_count.set(scene.model().objects().len());
                id
            };
            set_object_ids.update(|ids| ids.push(id));
            update_mesh(&scene, &renderer);
            set_selected_id.set(Some(id));
            set_browser_selected.set(format!("body-{}", id.saturating_add(1)));
            set_active_tool.set("box".to_string());
            if let Some(transform) = scene.borrow().object_transform(id) {
                set_baseline_transform.set(Some(transform));
                set_transform_ui.set(TransformUi::from_transform(transform));
            }
            (push_log.as_ref())(UiLogLevel::Success, format!("Body {} created", id + 1));
        })
    };

    let add_cylinder_action: Rc<dyn Fn()> = {
        let scene = scene.clone();
        let renderer = renderer.clone();
        let set_object_count = set_object_count;
        let set_object_ids = set_object_ids;
        let set_selected_id = set_selected_id;
        let set_transform_ui = set_transform_ui;
        let set_baseline_transform = set_baseline_transform;
        let set_browser_selected = set_browser_selected;
        let set_active_tool = set_active_tool;
        let push_log = push_log.clone();
        Rc::new(move || {
            let id = {
                let mut scene = scene.borrow_mut();
                let id = scene.add_cylinder(0.5, 1.5);
                set_object_count.set(scene.model().objects().len());
                id
            };
            set_object_ids.update(|ids| ids.push(id));
            update_mesh(&scene, &renderer);
            set_selected_id.set(Some(id));
            set_browser_selected.set(format!("body-{}", id.saturating_add(1)));
            set_active_tool.set("cylinder".to_string());
            if let Some(transform) = scene.borrow().object_transform(id) {
                set_baseline_transform.set(Some(transform));
                set_transform_ui.set(TransformUi::from_transform(transform));
            }
            (push_log.as_ref())(UiLogLevel::Success, format!("Cylinder {} created", id + 1));
        })
    };

    let activate_move_tool: Rc<dyn Fn()> = {
        let set_active_tool = set_active_tool;
        let set_tool_mode = set_tool_mode;
        let set_sketch_anchor = set_sketch_anchor;
        let set_sketch_cursor = set_sketch_cursor;
        Rc::new(move || {
            set_active_tool.set("move".to_string());
            set_tool_mode.set(EditorTool::Move);
            set_sketch_anchor.set(None);
            set_sketch_cursor.set(None);
        })
    };

    let activate_select_tool: Rc<dyn Fn()> = {
        let set_active_tool = set_active_tool;
        let set_tool_mode = set_tool_mode;
        let set_sketch_anchor = set_sketch_anchor;
        let set_sketch_cursor = set_sketch_cursor;
        Rc::new(move || {
            set_active_tool.set("select".to_string());
            set_tool_mode.set(EditorTool::None);
            set_sketch_anchor.set(None);
            set_sketch_cursor.set(None);
        })
    };

    let start_sketch_select: Rc<dyn Fn()> = {
        let set_active_tool = set_active_tool;
        let set_tool_mode = set_tool_mode;
        let set_sketch_plane = set_sketch_plane;
        let set_sketch_plane_name = set_sketch_plane_name;
        let set_sketch_segments = set_sketch_segments;
        let set_sketch_anchor = set_sketch_anchor;
        let set_sketch_cursor = set_sketch_cursor;
        let push_log = push_log.clone();
        Rc::new(move || {
            set_active_tool.set("sketch".to_string());
            set_tool_mode.set(EditorTool::SketchSelect);
            set_sketch_plane.set(None);
            set_sketch_plane_name.set(String::new());
            set_sketch_segments.set(Vec::new());
            set_sketch_anchor.set(None);
            set_sketch_cursor.set(None);
            (push_log.as_ref())(
                UiLogLevel::Info,
                "Sketch: select a planar face or a base plane".to_string(),
            );
        })
    };

    let finish_sketch: Rc<dyn Fn()> = {
        let set_active_tool = set_active_tool;
        let set_tool_mode = set_tool_mode;
        let sketch_plane = sketch_plane;
        let sketch_plane_name = sketch_plane_name;
        let set_sketch_plane = set_sketch_plane;
        let set_sketch_plane_name = set_sketch_plane_name;
        let set_sketch_segments = set_sketch_segments;
        let set_sketch_anchor = set_sketch_anchor;
        let set_sketch_cursor = set_sketch_cursor;
        let sketch_segments = sketch_segments;
        let set_saved_sketches = set_saved_sketches;
        let next_sketch_id = next_sketch_id;
        let set_next_sketch_id = set_next_sketch_id;
        let set_browser_selected = set_browser_selected;
        let push_log = push_log.clone();
        Rc::new(move || {
            if sketch_plane.get_untracked().is_some() {
                let sketch_id = next_sketch_id.get_untracked();
                let name = format!("Sketch {sketch_id}");
                let plane_label = sketch_plane_name.get_untracked();
                let segments = sketch_segments.get_untracked();
                set_saved_sketches.update(|items| {
                    items.push(SavedSketch {
                        id: sketch_id,
                        name: name.clone(),
                        plane_label: plane_label.clone(),
                        segments: segments.clone(),
                    });
                });
                set_next_sketch_id.set(sketch_id + 1);
                set_browser_selected.set(format!("sketch-{sketch_id}"));
                (push_log.as_ref())(
                    UiLogLevel::Success,
                    format!("{} saved with {} segments", name, segments.len()),
                );
            }

            set_tool_mode.set(EditorTool::None);
            set_active_tool.set("select".to_string());
            set_sketch_plane.set(None);
            set_sketch_plane_name.set(String::new());
            set_sketch_segments.set(Vec::new());
            set_sketch_anchor.set(None);
            set_sketch_cursor.set(None);
        })
    };

    let cancel_sketch: Rc<dyn Fn()> = {
        let set_active_tool = set_active_tool;
        let set_tool_mode = set_tool_mode;
        let set_sketch_plane = set_sketch_plane;
        let set_sketch_plane_name = set_sketch_plane_name;
        let set_sketch_segments = set_sketch_segments;
        let set_sketch_anchor = set_sketch_anchor;
        let set_sketch_cursor = set_sketch_cursor;
        let push_log = push_log.clone();
        Rc::new(move || {
            set_tool_mode.set(EditorTool::None);
            set_active_tool.set("select".to_string());
            set_sketch_plane.set(None);
            set_sketch_plane_name.set(String::new());
            set_sketch_segments.set(Vec::new());
            set_sketch_anchor.set(None);
            set_sketch_cursor.set(None);
            (push_log.as_ref())(UiLogLevel::Warning, "Sketch canceled".to_string());
        })
    };

    let on_add_box = {
        let add_box_action = add_box_action.clone();
        move |_| (add_box_action.as_ref())()
    };

    let on_add_cylinder = {
        let add_cylinder_action = add_cylinder_action.clone();
        move |_| (add_cylinder_action.as_ref())()
    };

    let on_boolean_stub = {
        let push_log = push_log.clone();
        let set_active_tool = set_active_tool;
        move |_| {
            set_active_tool.set("join".to_string());
            log("Boolean subtract is not implemented yet.");
            (push_log.as_ref())(
                UiLogLevel::Warning,
                "Boolean subtract is not implemented yet".to_string(),
            );
        }
    };

    {
        let add_box_action = add_box_action.clone();
        let add_cylinder_action = add_cylinder_action.clone();
        let activate_move_tool = activate_move_tool.clone();
        let activate_select_tool = activate_select_tool.clone();
        let set_show_palette = set_show_palette;
        let set_pending_command = set_pending_command;
        let set_active_tool = set_active_tool;
        let push_log = push_log.clone();
        Effect::new(move |_| {
            let Some(command_id) = pending_command.get() else {
                return;
            };
            match command_id.as_str() {
                "box" => (add_box_action.as_ref())(),
                "move" => (activate_move_tool.as_ref())(),
                "sphere" => {
                    set_active_tool.set("sphere".to_string());
                    (push_log.as_ref())(
                        UiLogLevel::Info,
                        "Sphere primitive is not connected yet".to_string(),
                    );
                }
                "export" => {
                    set_active_tool.set("export".to_string());
                    (push_log.as_ref())(
                        UiLogLevel::Warning,
                        "Export command is not implemented yet".to_string(),
                    );
                }
                "section" => {
                    set_active_tool.set("section".to_string());
                    (push_log.as_ref())(
                        UiLogLevel::Info,
                        "Section mode is not connected yet".to_string(),
                    );
                }
                "import" => {
                    set_active_tool.set("import".to_string());
                    (push_log.as_ref())(
                        UiLogLevel::Info,
                        "Import is not connected yet".to_string(),
                    );
                }
                "rotate" => {
                    set_active_tool.set("rotate".to_string());
                    (push_log.as_ref())(
                        UiLogLevel::Info,
                        "Rotate tool is not connected yet".to_string(),
                    );
                }
                "extrude" => {
                    set_active_tool.set("extrude".to_string());
                    (push_log.as_ref())(
                        UiLogLevel::Info,
                        "Extrude is not connected yet".to_string(),
                    );
                }
                "scale" => {
                    set_active_tool.set("scale".to_string());
                    (push_log.as_ref())(
                        UiLogLevel::Info,
                        "Scale tool is not connected yet".to_string(),
                    );
                }
                "measure" => {
                    (activate_select_tool.as_ref())();
                    set_active_tool.set("measure".to_string());
                    (push_log.as_ref())(
                        UiLogLevel::Info,
                        "Measure mode is not connected yet".to_string(),
                    );
                }
                "cylinder" => (add_cylinder_action.as_ref())(),
                _ => {}
            }
            set_show_palette.set(false);
            set_pending_command.set(None);
        });
    }

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
        let sketch_plane = sketch_plane;
        let sketch_segments = sketch_segments;
        let sketch_anchor = sketch_anchor;
        let sketch_cursor = sketch_cursor;
        Effect::new(move |_| {
            if !renderer_ready.get() {
                return;
            }
            let mode = tool_mode.get();
            match mode {
                EditorTool::Move => {
                    update_overlay(&scene, &renderer, selected_id.get(), true);
                }
                EditorTool::SketchDraw => {
                    let segments = sketch_segments.get();
                    update_sketch_overlay(
                        &renderer,
                        sketch_plane.get(),
                        &segments,
                        sketch_anchor.get(),
                        sketch_cursor.get(),
                    );
                }
                EditorTool::SketchSelect => {
                    update_sketch_overlay(&renderer, None, &[], None, None);
                }
                EditorTool::None => {
                    update_overlay(&scene, &renderer, selected_id.get(), false);
                }
            }
        });
    }

    view! {
        <div class="cad-shell">
            <div class="cad-topbar">
                <div class="topbar-tabs">
                    {TOP_TABS
                        .into_iter()
                        .map(|tab| {
                            view! {
                                <button
                                    class="top-tab-btn"
                                    class:active=move || active_tab.get() == tab
                                    on:click=move |_| set_active_tab.set(tab.to_string())
                                >
                                    {tab}
                                </button>
                            }
                        })
                        .collect_view()}
                </div>
                <div class="topbar-right">
                    <span class="save-dot"></span>
                    <span class="topbar-meta">"Saved"</span>
                    <button class="icon-btn">
                        <UiIcon name=IconName::User size=16 class="icon-btn-icon" />
                    </button>
                    <button class="icon-btn">
                        <UiIcon name=IconName::Settings size=16 class="icon-btn-icon" />
                    </button>
                </div>
            </div>

            <section class="cad-ribbon">
                <div class="ribbon-group">
                    <div class="ribbon-title">"CREATE"</div>
                    <div class="ribbon-tools">
                        <button class="ribbon-tool" class:active=move || active_tool.get() == "box" on:click=on_add_box>
                            <UiIcon name=IconName::Box size=20 class="ribbon-icon" />
                            <span class="ribbon-label">"Box"</span>
                        </button>
                        <button class="ribbon-tool" class:active=move || active_tool.get() == "sphere" on:click={
                            let set_active_tool = set_active_tool;
                            let push_log = push_log.clone();
                            move |_| {
                                set_active_tool.set("sphere".to_string());
                                (push_log.as_ref())(UiLogLevel::Info, "Sphere primitive is not connected yet".to_string());
                            }
                        }>
                            <UiIcon name=IconName::Circle size=20 class="ribbon-icon" />
                            <span class="ribbon-label">"Sphere"</span>
                        </button>
                        <button class="ribbon-tool" class:active=move || active_tool.get() == "cylinder" on:click=on_add_cylinder>
                            <UiIcon name=IconName::Cylinder size=20 class="ribbon-icon" />
                            <span class="ribbon-label">"Cylinder"</span>
                        </button>
                        <button class="ribbon-tool" class:active=move || active_tool.get() == "cone" on:click={
                            let set_active_tool = set_active_tool;
                            let push_log = push_log.clone();
                            move |_| {
                                set_active_tool.set("cone".to_string());
                                (push_log.as_ref())(UiLogLevel::Info, "Cone primitive is not connected yet".to_string());
                            }
                        }>
                            <UiIcon name=IconName::Cone size=20 class="ribbon-icon" />
                            <span class="ribbon-label">"Cone"</span>
                        </button>
                        <button class="ribbon-tool" class:active=move || active_tool.get() == "torus" on:click={
                            let set_active_tool = set_active_tool;
                            let push_log = push_log.clone();
                            move |_| {
                                set_active_tool.set("torus".to_string());
                                (push_log.as_ref())(UiLogLevel::Info, "Torus primitive is not connected yet".to_string());
                            }
                        }>
                            <UiIcon name=IconName::Torus size=20 class="ribbon-icon" />
                            <span class="ribbon-label">"Torus"</span>
                        </button>
                        <button class="ribbon-tool" class:active=move || active_tool.get() == "sketch" on:click={
                            let start_sketch_select = start_sketch_select.clone();
                            move |_| (start_sketch_select.as_ref())()
                        }>
                            <UiIcon name=IconName::Square size=20 class="ribbon-icon" />
                            <span class="ribbon-label">"Sketch"</span>
                        </button>
                        <button class="ribbon-tool" class:active=move || active_tool.get() == "more" on:click={
                            let set_active_tool = set_active_tool;
                            let push_log = push_log.clone();
                            move |_| {
                                set_active_tool.set("more".to_string());
                                (push_log.as_ref())(UiLogLevel::Info, "More tools are not connected yet".to_string());
                            }
                        }>
                            <UiIcon name=IconName::ChevronDown size=20 class="ribbon-icon" />
                            <span class="ribbon-label">"More"</span>
                        </button>
                    </div>
                </div>
                <div class="ribbon-group">
                    <div class="ribbon-title">"MODIFY"</div>
                    <div class="ribbon-tools">
                        <button class="ribbon-tool" class:active=move || active_tool.get() == "move" on:click={
                            let activate_move_tool = activate_move_tool.clone();
                            move |_| (activate_move_tool.as_ref())()
                        }>
                            <UiIcon name=IconName::Move size=20 class="ribbon-icon" />
                            <span class="ribbon-label">"Move"</span>
                        </button>
                        <button class="ribbon-tool" class:active=move || active_tool.get() == "rotate" on:click={
                            let set_active_tool = set_active_tool;
                            let push_log = push_log.clone();
                            move |_| {
                                set_active_tool.set("rotate".to_string());
                                (push_log.as_ref())(UiLogLevel::Info, "Rotate tool is not connected yet".to_string());
                            }
                        }>
                            <UiIcon name=IconName::RotateCw size=20 class="ribbon-icon" />
                            <span class="ribbon-label">"Rotate"</span>
                        </button>
                        <button class="ribbon-tool" class:active=move || active_tool.get() == "scale" on:click={
                            let set_active_tool = set_active_tool;
                            let push_log = push_log.clone();
                            move |_| {
                                set_active_tool.set("scale".to_string());
                                (push_log.as_ref())(UiLogLevel::Info, "Scale tool is not connected yet".to_string());
                            }
                        }>
                            <UiIcon name=IconName::Scale size=20 class="ribbon-icon" />
                            <span class="ribbon-label">"Scale"</span>
                        </button>
                        <button class="ribbon-tool" class:active=move || active_tool.get() == "copy" on:click={
                            let set_active_tool = set_active_tool;
                            let push_log = push_log.clone();
                            move |_| {
                                set_active_tool.set("copy".to_string());
                                (push_log.as_ref())(UiLogLevel::Info, "Copy tool is not connected yet".to_string());
                            }
                        }>
                            <UiIcon name=IconName::Copy size=20 class="ribbon-icon" />
                            <span class="ribbon-label">"Copy"</span>
                        </button>
                        <button class="ribbon-tool" class:active=move || active_tool.get() == "delete" on:click={
                            let set_active_tool = set_active_tool;
                            let push_log = push_log.clone();
                            move |_| {
                                set_active_tool.set("delete".to_string());
                                (push_log.as_ref())(UiLogLevel::Warning, "Delete tool is not connected yet".to_string());
                            }
                        }>
                            <UiIcon name=IconName::Trash2 size=20 class="ribbon-icon" />
                            <span class="ribbon-label">"Delete"</span>
                        </button>
                    </div>
                </div>
                <div class="ribbon-group">
                    <div class="ribbon-title">"ASSEMBLE"</div>
                    <div class="ribbon-tools">
                        <button class="ribbon-tool" class:active=move || active_tool.get() == "join" on:click=on_boolean_stub>
                            <UiIcon name=IconName::Link size=20 class="ribbon-icon" />
                            <span class="ribbon-label">"Join"</span>
                        </button>
                        <button class="ribbon-tool" class:active=move || active_tool.get() == "pattern" on:click={
                            let set_active_tool = set_active_tool;
                            let push_log = push_log.clone();
                            move |_| {
                                set_active_tool.set("pattern".to_string());
                                (push_log.as_ref())(UiLogLevel::Info, "Pattern tool is not connected yet".to_string());
                            }
                        }>
                            <UiIcon name=IconName::Grid3x3 size=20 class="ribbon-icon" />
                            <span class="ribbon-label">"Pattern"</span>
                        </button>
                        <button class="ribbon-tool" class:active=move || active_tool.get() == "mirror" on:click={
                            let set_active_tool = set_active_tool;
                            let push_log = push_log.clone();
                            move |_| {
                                set_active_tool.set("mirror".to_string());
                                (push_log.as_ref())(UiLogLevel::Info, "Mirror tool is not connected yet".to_string());
                            }
                        }>
                            <UiIcon name=IconName::Layers size=20 class="ribbon-icon" />
                            <span class="ribbon-label">"Mirror"</span>
                        </button>
                    </div>
                </div>
                <div class="ribbon-group">
                    <div class="ribbon-title">"CONSTRUCT"</div>
                    <div class="ribbon-tools">
                        <button class="ribbon-tool" class:active=move || active_tool.get() == "plane" on:click={
                            let set_active_tool = set_active_tool;
                            move |_| set_active_tool.set("plane".to_string())
                        }>
                            <UiIcon name=IconName::Square size=20 class="ribbon-icon" />
                            <span class="ribbon-label">"Plane"</span>
                        </button>
                        <button class="ribbon-tool" class:active=move || active_tool.get() == "axis" on:click={
                            let set_active_tool = set_active_tool;
                            move |_| set_active_tool.set("axis".to_string())
                        }>
                            <UiIcon name=IconName::Ruler size=20 class="ribbon-icon" />
                            <span class="ribbon-label">"Axis"</span>
                        </button>
                        <button class="ribbon-tool" class:active=move || active_tool.get() == "point" on:click={
                            let set_active_tool = set_active_tool;
                            move |_| set_active_tool.set("point".to_string())
                        }>
                            <UiIcon name=IconName::Circle size=20 class="ribbon-icon" />
                            <span class="ribbon-label">"Point"</span>
                        </button>
                    </div>
                </div>
                <div class="ribbon-group">
                    <div class="ribbon-title">"INSPECT"</div>
                    <div class="ribbon-tools">
                        <button class="ribbon-tool" class:active=move || active_tool.get() == "measure" on:click={
                            let set_active_tool = set_active_tool;
                            let push_log = push_log.clone();
                            move |_| {
                                set_active_tool.set("measure".to_string());
                                (push_log.as_ref())(UiLogLevel::Info, "Measure mode is not connected yet".to_string());
                            }
                        }>
                            <UiIcon name=IconName::Ruler size=20 class="ribbon-icon" />
                            <span class="ribbon-label">"Measure"</span>
                        </button>
                        <button class="ribbon-tool" class:active=move || active_tool.get() == "analyze" on:click={
                            let set_active_tool = set_active_tool;
                            move |_| set_active_tool.set("analyze".to_string())
                        }>
                            <UiIcon name=IconName::Gauge size=20 class="ribbon-icon" />
                            <span class="ribbon-label">"Analyze"</span>
                        </button>
                        <button class="ribbon-tool" class:active=move || active_tool.get() == "section" on:click={
                            let set_active_tool = set_active_tool;
                            move |_| set_active_tool.set("section".to_string())
                        }>
                            <UiIcon name=IconName::Eye size=20 class="ribbon-icon" />
                            <span class="ribbon-label">"Section"</span>
                        </button>
                    </div>
                </div>
                <div class="ribbon-group">
                    <div class="ribbon-title">"INSERT"</div>
                    <div class="ribbon-tools">
                        <button class="ribbon-tool" class:active=move || active_tool.get() == "import" on:click={
                            let set_active_tool = set_active_tool;
                            move |_| set_active_tool.set("import".to_string())
                        }>
                            <UiIcon name=IconName::File size=20 class="ribbon-icon" />
                            <span class="ribbon-label">"Import"</span>
                        </button>
                        <button class="ribbon-tool" class:active=move || active_tool.get() == "decal" on:click={
                            let set_active_tool = set_active_tool;
                            move |_| set_active_tool.set("decal".to_string())
                        }>
                            <UiIcon name=IconName::Image size=20 class="ribbon-icon" />
                            <span class="ribbon-label">"Decal"</span>
                        </button>
                        <button class="ribbon-tool" class:active=move || active_tool.get() == "mesh" on:click={
                            let set_active_tool = set_active_tool;
                            move |_| set_active_tool.set("mesh".to_string())
                        }>
                            <UiIcon name=IconName::Database size=20 class="ribbon-icon" />
                            <span class="ribbon-label">"Mesh"</span>
                        </button>
                    </div>
                </div>
                <div class="ribbon-group">
                    <div class="ribbon-title">"SELECT"</div>
                    <div class="ribbon-tools">
                        <button class="ribbon-tool" class:active=move || active_tool.get() == "select" on:click={
                            let activate_select_tool = activate_select_tool.clone();
                            move |_| (activate_select_tool.as_ref())()
                        }>
                            <UiIcon name=IconName::MousePointer size=20 class="ribbon-icon" />
                            <span class="ribbon-label">"Select"</span>
                        </button>
                        <button class="ribbon-tool" class:active=move || active_tool.get() == "window" on:click={
                            let set_active_tool = set_active_tool;
                            move |_| set_active_tool.set("window".to_string())
                        }>
                            <UiIcon name=IconName::Square size=20 class="ribbon-icon" />
                            <span class="ribbon-label">"Window"</span>
                        </button>
                        <button class="ribbon-tool" class:active=move || active_tool.get() == "freeform" on:click={
                            let set_active_tool = set_active_tool;
                            move |_| set_active_tool.set("freeform".to_string())
                        }>
                            <UiIcon name=IconName::Hand size=20 class="ribbon-icon" />
                            <span class="ribbon-label">"Freeform"</span>
                        </button>
                    </div>
                </div>
            </section>

            <div class="cad-main">
                <aside class="browser">
                    <div class="browser-search-wrap">
                        <UiIcon name=IconName::Search size=16 class="browser-search-icon" />
                        <input
                            class="browser-input"
                            type="text"
                            placeholder="Search browser..."
                            prop:value=move || browser_search.get()
                            on:input=move |ev| set_browser_search.set(event_target_value(&ev))
                        />
                        <div class="browser-search-actions">
                            <button class="small-icon-btn">
                                <UiIcon name=IconName::Filter size=14 class="small-icon" />
                            </button>
                            <button class="small-icon-btn">
                                <UiIcon name=IconName::Eye size=14 class="small-icon" />
                            </button>
                        </div>
                    </div>
                    <div class="browser-tree">
                        <button class="tree-row" class:selected=move || browser_selected.get() == "doc-settings" on:click=move |_| set_browser_selected.set("doc-settings".to_string())>
                            <span class="tree-toggle blank">""</span>
                            <UiIcon name=IconName::FileText size=16 class="tree-icon" />
                            <span class="tree-text">"Document Settings"</span>
                        </button>
                        <button class="tree-row" class:selected=move || browser_selected.get() == "named-views" on:click=move |_| set_browser_selected.set("named-views".to_string())>
                            <span class="tree-toggle blank">""</span>
                            <UiIcon name=IconName::Bookmark size=16 class="tree-icon" />
                            <span class="tree-text">"Named Views"</span>
                        </button>

                        <div class="tree-row tree-group" class:selected=move || browser_selected.get() == "origin">
                            <button class="tree-toggle" on:click=move |_| set_expand_origin.update(|v| *v = !*v)>
                                {move || {
                                    if expand_origin.get() {
                                        view! { <UiIcon name=IconName::ChevronDown size=14 class="tree-toggle-icon" /> }
                                    } else {
                                        view! { <UiIcon name=IconName::ChevronRight size=14 class="tree-toggle-icon" /> }
                                    }
                                }}
                            </button>
                            <button class="tree-main-btn" on:click=move |_| set_browser_selected.set("origin".to_string())>
                                <UiIcon name=IconName::Compass size=16 class="tree-icon" />
                                <span class="tree-text">"Origin"</span>
                            </button>
                        </div>
                        <Show when=move || expand_origin.get()>
                            <div class="tree-children">
                                <label class="tree-check">
                                    <input type="checkbox" prop:checked=plane_xy on:change=move |ev| set_plane_xy.set(event_target_checked(&ev)) />
                                    <span>"XY Plane"</span>
                                </label>
                                <label class="tree-check">
                                    <input type="checkbox" prop:checked=plane_zx on:change=move |ev| set_plane_zx.set(event_target_checked(&ev)) />
                                    <span>"XZ Plane"</span>
                                </label>
                                <label class="tree-check">
                                    <input type="checkbox" prop:checked=plane_yz on:change=move |ev| set_plane_yz.set(event_target_checked(&ev)) />
                                    <span>"YZ Plane"</span>
                                </label>
                            </div>
                        </Show>

                        <div class="tree-row tree-group" class:selected=move || browser_selected.get() == "sketches">
                            <button class="tree-toggle" on:click=move |_| set_expand_sketches.update(|v| *v = !*v)>
                                {move || {
                                    if expand_sketches.get() {
                                        view! { <UiIcon name=IconName::ChevronDown size=14 class="tree-toggle-icon" /> }
                                    } else {
                                        view! { <UiIcon name=IconName::ChevronRight size=14 class="tree-toggle-icon" /> }
                                    }
                                }}
                            </button>
                            <button class="tree-main-btn" on:click=move |_| set_browser_selected.set("sketches".to_string())>
                                <UiIcon name=IconName::PenTool size=16 class="tree-icon" />
                                <span class="tree-text">"Sketches"</span>
                            </button>
                        </div>
                        <Show when=move || expand_sketches.get()>
                            <div class="tree-children">
                                {move || {
                                    let items = saved_sketches.get();
                                    if items.is_empty() {
                                        return view! {
                                            <div class="tree-empty">"No sketches yet"</div>
                                        }
                                            .into_any();
                                    }
                                    items
                                        .into_iter()
                                        .map(|item| {
                                            let row_id = format!("sketch-{}", item.id);
                                            let row_id_for_class = row_id.clone();
                                            let label = format!(
                                                "{} · {} seg · {}",
                                                item.name,
                                                item.segments.len(),
                                                item.plane_label
                                            );
                                            view! {
                                                <button
                                                    class="tree-row tree-leaf"
                                                    class:selected=move || browser_selected.get() == row_id_for_class
                                                    on:click={
                                                        let row_id = row_id.clone();
                                                        move |_| set_browser_selected.set(row_id.clone())
                                                    }
                                                >
                                                    {label}
                                                </button>
                                            }
                                        })
                                        .collect_view()
                                        .into_any()
                                }}
                            </div>
                        </Show>

                        <div class="tree-row tree-group" class:selected=move || browser_selected.get() == "bodies">
                            <button class="tree-toggle" on:click=move |_| set_expand_bodies.update(|v| *v = !*v)>
                                {move || {
                                    if expand_bodies.get() {
                                        view! { <UiIcon name=IconName::ChevronDown size=14 class="tree-toggle-icon" /> }
                                    } else {
                                        view! { <UiIcon name=IconName::ChevronRight size=14 class="tree-toggle-icon" /> }
                                    }
                                }}
                            </button>
                            <button class="tree-main-btn" on:click=move |_| set_browser_selected.set("bodies".to_string())>
                                <UiIcon name=IconName::Box size=16 class="tree-icon" />
                                <span class="tree-text">
                                    {move || format!("Bodies ({})", object_count.get())}
                                </span>
                            </button>
                        </div>
                        <Show when=move || expand_bodies.get()>
                            <div class="tree-children">
                                {move || {
                                    object_ids
                                        .get()
                                        .into_iter()
                                        .enumerate()
                                        .map(|(idx, object_id)| {
                                            let row_id = format!("body-{}", idx + 1);
                                            let row_id_for_class = row_id.clone();
                                            view! {
                                                <button
                                                    class="tree-row tree-leaf"
                                                    class:selected=move || browser_selected.get() == row_id_for_class
                                                    on:click={
                                                        let row_id = row_id.clone();
                                                        move |_| {
                                                            set_browser_selected.set(row_id.clone());
                                                            set_selected_id.set(Some(object_id));
                                                        }
                                                    }
                                                >
                                                    <UiIcon name=IconName::Box size=16 class="tree-icon" />
                                                    <span class="tree-text">{format!("Body {}", idx + 1)}</span>
                                                </button>
                                            }
                                        })
                                        .collect_view()
                                }}
                            </div>
                        </Show>

                        <div class="tree-row tree-group" class:selected=move || browser_selected.get() == "components">
                            <button class="tree-toggle" on:click=move |_| set_expand_components.update(|v| *v = !*v)>
                                {move || {
                                    if expand_components.get() {
                                        view! { <UiIcon name=IconName::ChevronDown size=14 class="tree-toggle-icon" /> }
                                    } else {
                                        view! { <UiIcon name=IconName::ChevronRight size=14 class="tree-toggle-icon" /> }
                                    }
                                }}
                            </button>
                            <button class="tree-main-btn" on:click=move |_| set_browser_selected.set("components".to_string())>
                                <UiIcon name=IconName::Folder size=16 class="tree-icon" />
                                <span class="tree-text">"Components"</span>
                            </button>
                        </div>
                        <Show when=move || expand_components.get()>
                            <div class="tree-children">
                                <div class="tree-row tree-group">
                                    <button class="tree-toggle" on:click=move |_| set_expand_component_1.update(|v| *v = !*v)>
                                        {move || {
                                            if expand_component_1.get() {
                                                view! { <UiIcon name=IconName::ChevronDown size=14 class="tree-toggle-icon" /> }
                                            } else {
                                                view! { <UiIcon name=IconName::ChevronRight size=14 class="tree-toggle-icon" /> }
                                            }
                                        }}
                                    </button>
                                    <UiIcon name=IconName::Folder size=16 class="tree-icon" />
                                    <span class="tree-text">"Component 1"</span>
                                </div>
                                <Show when=move || expand_component_1.get()>
                                    <div class="tree-children">
                                        <button class="tree-row tree-leaf">"Part A"</button>
                                        <button class="tree-row tree-leaf">"Part B"</button>
                                    </div>
                                </Show>
                                <button class="tree-row tree-leaf">"Component 2"</button>
                            </div>
                        </Show>
                    </div>
                </aside>

                <main class="viewport-frame">
                    <div class="viewport-grid"></div>
                    <canvas id="viewport-canvas" node_ref=canvas_ref></canvas>
                    <div class="viewcube-wrap">
                        <canvas id="viewcube-canvas" node_ref=viewcube_ref></canvas>
                        <div class="viewcube-label">"View: Perspective"</div>
                    </div>

                    <div class="viewport-nav">
                        <button class="nav-tool" class:active=move || active_tool.get() == "select" on:click={
                            let activate_select_tool = activate_select_tool.clone();
                            move |_| (activate_select_tool.as_ref())()
                        }>
                            <UiIcon name=IconName::MousePointer2 size=20 class="nav-icon" />
                        </button>
                        <button class="nav-tool" class:active=move || active_tool.get() == "freeform" on:click={
                            let set_active_tool = set_active_tool;
                            move |_| set_active_tool.set("freeform".to_string())
                        }>
                            <UiIcon name=IconName::Hand size=20 class="nav-icon" />
                        </button>
                        <div class="nav-divider"></div>
                        <button class="nav-tool" title="Zoom In">
                            <UiIcon name=IconName::ZoomIn size=20 class="nav-icon" />
                        </button>
                        <button class="nav-tool" title="Zoom Out">
                            <UiIcon name=IconName::ZoomOut size=20 class="nav-icon" />
                        </button>
                        <button class="nav-tool" title="Fit View">
                            <UiIcon name=IconName::Maximize2 size=20 class="nav-icon" />
                        </button>
                    </div>

                    <div
                        class="sketch-prompt-card"
                        style:display=move || {
                            if tool_mode.get() == EditorTool::SketchSelect {
                                "block"
                            } else {
                                "none"
                            }
                        }
                    >
                        <div class="sketch-prompt-title">"Create Sketch"</div>
                        <div class="sketch-prompt-text">
                            "Select any planar face on a body or choose a base plane."
                        </div>
                        <div class="sketch-prompt-actions">
                            <button class="sketch-plane-btn" on:click={
                                let enter_sketch_draw = enter_sketch_draw.clone();
                                move |_| {
                                    let (plane, label) = base_sketch_plane(BaseSketchPlane::XY);
                                    (enter_sketch_draw.as_ref())(plane, label.to_string());
                                }
                            }>
                                "XY Plane"
                            </button>
                            <button class="sketch-plane-btn" on:click={
                                let enter_sketch_draw = enter_sketch_draw.clone();
                                move |_| {
                                    let (plane, label) = base_sketch_plane(BaseSketchPlane::XZ);
                                    (enter_sketch_draw.as_ref())(plane, label.to_string());
                                }
                            }>
                                "XZ Plane"
                            </button>
                            <button class="sketch-plane-btn" on:click={
                                let enter_sketch_draw = enter_sketch_draw.clone();
                                move |_| {
                                    let (plane, label) = base_sketch_plane(BaseSketchPlane::YZ);
                                    (enter_sketch_draw.as_ref())(plane, label.to_string());
                                }
                            }>
                                "YZ Plane"
                            </button>
                        </div>
                        <div class="sketch-prompt-foot">
                            <button class="sketch-cancel-btn" on:click={
                                let cancel_sketch = cancel_sketch.clone();
                                move |_| (cancel_sketch.as_ref())()
                            }>
                                "Cancel"
                            </button>
                        </div>
                    </div>

                    <div
                        class="sketch-mode-card"
                        style:display=move || {
                            if tool_mode.get() == EditorTool::SketchDraw {
                                "block"
                            } else {
                                "none"
                            }
                        }
                    >
                        <div class="sketch-mode-head">
                            <span class="sketch-mode-title">
                                {move || format!("Sketch: {}", sketch_plane_name.get())}
                            </span>
                            <span class="sketch-mode-count">
                                {move || format!("{} segments", sketch_segments.get().len())}
                            </span>
                        </div>
                        <div class="sketch-mode-text">
                            "Click to place points. Each next click adds a line segment on the sketch plane."
                        </div>
                        <div class="sketch-mode-actions">
                            <button class="sketch-finish-btn" on:click={
                                let finish_sketch = finish_sketch.clone();
                                move |_| (finish_sketch.as_ref())()
                            }>
                                "Finish Sketch"
                            </button>
                            <button class="sketch-cancel-btn" on:click={
                                let cancel_sketch = cancel_sketch.clone();
                                move |_| (cancel_sketch.as_ref())()
                            }>
                                "Cancel"
                            </button>
                        </div>
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
                                let activate_select_tool = activate_select_tool.clone();
                                Rc::new(move || {
                                    if selected_id.get_untracked().is_some() {
                                        set_baseline_transform
                                            .set(Some(transform_ui.get_untracked().to_transform()));
                                    }
                                    (activate_select_tool.as_ref())();
                                })
                            }
                            on_cancel={
                                let scene = scene.clone();
                                let renderer = renderer.clone();
                                let activate_select_tool = activate_select_tool.clone();
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
                                    (activate_select_tool.as_ref())();
                                })
                            }
                        />
                    </aside>

                    <div class="viewport-status">
                        <div class="status-left">
                            <span>"Zoom: 100%"</span>
                            <span>"•"</span>
                            <span class="status-ok">"Snap: On"</span>
                            <span>"•"</span>
                            <span>"Units: mm"</span>
                        </div>
                        <div class="status-right">
                            <span>{move || format!("Objects: {}", object_count.get())}</span>
                            <span>"•"</span>
                            <span>{move || {
                                match tool_mode.get() {
                                    EditorTool::Move => "Tool: Move".to_string(),
                                    EditorTool::SketchSelect => "Tool: Sketch Select".to_string(),
                                    EditorTool::SketchDraw => "Tool: Sketch Draw".to_string(),
                                    EditorTool::None => "Tool: View".to_string(),
                                }
                            }}</span>
                            <span>"•"</span>
                            <span>"FPS: 60"</span>
                            <button class="help-btn">"?"</button>
                        </div>
                    </div>
                </main>
            </div>

            <footer class="timeline">
                <div class="timeline-controls">
                    <button class="timeline-control" title="Step Back">
                        <UiIcon name=IconName::SkipBack size=16 class="timeline-control-icon" />
                    </button>
                    <button class="timeline-control" title="Play">
                        <UiIcon name=IconName::Play size=16 class="timeline-control-icon" />
                    </button>
                    <button class="timeline-control" title="Step Forward">
                        <UiIcon name=IconName::SkipForward size=16 class="timeline-control-icon" />
                    </button>
                    <div class="timeline-divider"></div>
                    <span class="timeline-title">"Feature History"</span>
                </div>
                <div class="timeline-track">
                    <button class="timeline-scroll-btn">
                        <UiIcon name=IconName::ChevronLeft size=16 class="timeline-scroll-icon" />
                    </button>
                    <div class="timeline-items">
                        {TIMELINE_FEATURES
                            .into_iter()
                            .map(|(id, number, label)| {
                                view! {
                                    <button
                                        class="timeline-chip"
                                        class:active=move || active_feature.get() == id
                                        on:click=move |_| set_active_feature.set(id.to_string())
                                    >
                                        <span class="chip-number">{number}</span>
                                        <span class="chip-label">{label}</span>
                                    </button>
                                }
                            })
                            .collect_view()}
                    </div>
                    <button class="timeline-scroll-btn">
                        <UiIcon name=IconName::ChevronRight size=16 class="timeline-scroll-icon" />
                    </button>
                </div>
            </footer>

            <Show when=move || show_palette.get()>
                <div class="command-backdrop" on:click=move |_| set_show_palette.set(false)>
                    <div class="command-dialog" on:click=move |ev| ev.stop_propagation()>
                        <div class="command-head">
                            <div class="command-input-wrap">
                                <UiIcon name=IconName::Search size=20 class="command-search-icon" />
                                <input
                                    class="command-input"
                                    type="text"
                                    placeholder="Search commands..."
                                    prop:value=move || palette_query.get()
                                    on:input=move |ev| set_palette_query.set(event_target_value(&ev))
                                />
                                <button class="command-close" on:click=move |_| set_show_palette.set(false)>
                                    <UiIcon name=IconName::X size=16 class="command-close-icon" />
                                </button>
                            </div>
                        </div>
                        <div class="command-list">
                            {move || {
                                let query = palette_query.get().to_lowercase();
                                let filtered: Vec<UiCommand> = UI_COMMANDS
                                    .into_iter()
                                    .filter(|cmd| {
                                        if query.is_empty() {
                                            return true;
                                        }
                                        cmd.label.to_lowercase().contains(&query)
                                            || cmd.category.to_lowercase().contains(&query)
                                    })
                                    .collect();

                                if filtered.is_empty() {
                                    view! { <div class="command-empty">"No commands found"</div> }.into_any()
                                } else {
                                    view! {
                                        <>
                                            {filtered
                                                .into_iter()
                                                .map(|cmd| {
                                                    view! {
                                                        <button
                                                            class="command-row"
                                                            on:click=move |_| {
                                                                set_pending_command.set(Some(cmd.id.to_string()));
                                                            }
                                                        >
                                                            <div class="command-row-main">
                                                                <UiIcon
                                                                    name=command_icon(cmd.id)
                                                                    size=16
                                                                    class="command-row-icon"
                                                                />
                                                                <div class="command-row-text">
                                                                    <span class="command-row-label">{cmd.label}</span>
                                                                    <span class="command-row-category">{cmd.category}</span>
                                                                </div>
                                                            </div>
                                                            <span class="command-row-shortcut">
                                                                {if let Some(shortcut) = cmd.shortcut {
                                                                    view! {
                                                                        <>
                                                                            {shortcut
                                                                                .split('+')
                                                                                .map(|key| {
                                                                                    view! { <kbd>{key}</kbd> }
                                                                                })
                                                                                .collect_view()}
                                                                        </>
                                                                    }
                                                                        .into_any()
                                                                } else {
                                                                    view! { <></> }.into_any()
                                                                }}
                                                            </span>
                                                        </button>
                                                    }
                                                })
                                                .collect_view()}
                                        </>
                                    }
                                        .into_any()
                                }
                            }}
                        </div>
                        <div class="command-foot">
                            <span>"Type to search"</span>
                            <span class="command-foot-actions">
                                <kbd>"↑↓"</kbd>
                                <span>"Navigate"</span>
                                <kbd>"↵"</kbd>
                                <span>"Execute"</span>
                                <kbd>"Esc"</kbd>
                                <span>"Close"</span>
                            </span>
                        </div>
                    </div>
                </div>
            </Show>

            <Show
                when=move || !show_console.get()
                fallback=move || {
                    view! {
                        <div class="console-panel">
                            <div class="console-head">
                                <div class="console-head-left">
                                    <UiIcon name=IconName::Terminal size=16 class="console-icon" />
                                    <span class="console-title">"Console"</span>
                                    <span class="console-badge">{move || log_entries.get().len().to_string()}</span>
                                </div>
                                <div class="console-head-right">
                                    <button class="console-head-btn" on:click=move |_| set_console_expanded.update(|open| *open = !*open)>
                                        {move || {
                                            if console_expanded.get() {
                                                view! { <UiIcon name=IconName::ChevronDown size=16 class="console-head-icon" /> }
                                            } else {
                                                view! { <UiIcon name=IconName::ChevronUp size=16 class="console-head-icon" /> }
                                            }
                                        }}
                                    </button>
                                    <button class="console-head-btn" on:click=move |_| set_show_console.set(false)>
                                        <UiIcon name=IconName::X size=16 class="console-head-icon" />
                                    </button>
                                </div>
                            </div>
                            <Show when=move || console_expanded.get()>
                                <div class="console-list">
                                    {move || {
                                        log_entries
                                            .get()
                                            .into_iter()
                                            .map(|entry| {
                                                let level_class = match entry.level {
                                                    UiLogLevel::Success => "success",
                                                    UiLogLevel::Warning => "warning",
                                                    UiLogLevel::Info => "info",
                                                };
                                                let level_icon = match entry.level {
                                                    UiLogLevel::Success => IconName::Check,
                                                    UiLogLevel::Warning => IconName::AlertTriangle,
                                                    UiLogLevel::Info => IconName::Info,
                                                };
                                                view! {
                                                    <div class="console-row">
                                                        <span class={format!("console-level {}", level_class)}>
                                                            <UiIcon name=level_icon size=16 class="console-level-icon" />
                                                        </span>
                                                        <div class="console-row-main">
                                                            <div class="console-msg">{entry.message}</div>
                                                            <div class="console-time">{entry.timestamp}</div>
                                                        </div>
                                                    </div>
                                                }
                                            })
                                            .collect_view()
                                    }}
                                </div>
                                <div class="console-foot">
                                    <button class="console-clear" on:click=move |_| set_log_entries.set(Vec::new())>
                                        "Clear all"
                                    </button>
                                    <span>"Last updated: now"</span>
                                </div>
                            </Show>
                        </div>
                    }
                        .into_any()
                }
            >
                <button class="console-fab" on:click=move |_| set_show_console.set(true)>
                    <UiIcon name=IconName::Terminal size=16 class="console-icon" />
                    <span>"Console"</span>
                    <span class="console-badge">{move || log_entries.get().len().to_string()}</span>
                </button>
            </Show>

            <Show
                when=move || !show_shortcuts.get()
                fallback=move || {
                    view! {
                        <div class="shortcuts-panel">
                            <div class="shortcuts-head">
                                <div class="shortcuts-title-wrap">
                                    <UiIcon name=IconName::Keyboard size=16 class="shortcuts-icon" />
                                    <span class="shortcuts-title">"Keyboard Shortcuts"</span>
                                </div>
                                <button class="shortcuts-close" on:click=move |_| set_show_shortcuts.set(false)>
                                    <UiIcon name=IconName::X size=16 class="shortcuts-close-icon" />
                                </button>
                            </div>
                            <div class="shortcuts-list">
                                {["General", "File", "Edit", "Create", "Modify", "View"]
                                    .into_iter()
                                    .map(|category| {
                                        view! {
                                            <div class="shortcut-group">
                                                <div class="shortcut-group-title">{category}</div>
                                                {UI_SHORTCUTS
                                                    .into_iter()
                                                    .filter(|item| item.category == category)
                                                    .map(|item| {
                                                        view! {
                                                            <div class="shortcut-row">
                                                                <span class="shortcut-desc">{item.description}</span>
                                                                <span class="shortcut-keys">
                                                                    {item
                                                                        .keys
                                                                        .iter()
                                                                        .map(|key| {
                                                                            view! { <kbd>{*key}</kbd> }
                                                                        })
                                                                        .collect_view()}
                                                                </span>
                                                            </div>
                                                        }
                                                    })
                                                    .collect_view()}
                                            </div>
                                        }
                                    })
                                    .collect_view()}
                            </div>
                        </div>
                    }
                        .into_any()
                }
            >
                <button class="shortcuts-fab" on:click=move |_| set_show_shortcuts.set(true)>
                    <UiIcon name=IconName::Keyboard size=16 class="shortcuts-icon" />
                    <span>"Shortcuts"</span>
                </button>
            </Show>

            <Show when=move || show_project_info.get()>
                <div class="project-info">
                    <div class="project-info-head">
                        <div class="project-title-wrap">
                            <UiIcon name=IconName::FileText size=16 class="project-title-icon" />
                            <span class="project-title">"Project Information"</span>
                        </div>
                        <button class="project-close" on:click=move |_| set_show_project_info.set(false)>
                            <UiIcon name=IconName::X size=14 class="project-close-icon" />
                        </button>
                    </div>
                    <div class="project-row">
                        <UiIcon name=IconName::Package size=14 class="project-row-icon" />
                        <span class="project-row-label">"Project Name"</span>
                        <span class="project-row-value">"Mechanical Assembly v2"</span>
                    </div>
                    <div class="project-row">
                        <UiIcon name=IconName::User size=14 class="project-row-icon" />
                        <span class="project-row-label">"Created by"</span>
                        <span class="project-row-value">"Design Engineer"</span>
                    </div>
                    <div class="project-row">
                        <UiIcon name=IconName::Calendar size=14 class="project-row-icon" />
                        <span class="project-row-label">"Last Modified"</span>
                        <span class="project-row-value">"Feb 16, 2026 10:23"</span>
                    </div>
                    <div class="project-foot">
                        <span>"10 Features"</span>
                        <span>"•"</span>
                        <span>"3 Components"</span>
                        <span>"•"</span>
                        <span>"2 Bodies"</span>
                    </div>
                </div>
            </Show>
        </div>
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum EditorTool {
    None,
    Move,
    SketchSelect,
    SketchDraw,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum BaseSketchPlane {
    XY,
    XZ,
    YZ,
}

#[derive(Clone, Copy)]
struct SketchPlane {
    origin: Vec3,
    normal: Vec3,
    u: Vec3,
    v: Vec3,
}

#[derive(Clone, Copy)]
struct SketchSegment {
    a: Vec3,
    b: Vec3,
}

#[derive(Clone)]
struct SavedSketch {
    id: usize,
    name: String,
    plane_label: String,
    segments: Vec<SketchSegment>,
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

fn base_sketch_plane(kind: BaseSketchPlane) -> (SketchPlane, &'static str) {
    match kind {
        BaseSketchPlane::XY => (
            SketchPlane {
                origin: Vec3::ZERO,
                normal: Vec3::Z,
                u: Vec3::X,
                v: Vec3::Y,
            },
            "XY Plane",
        ),
        BaseSketchPlane::XZ => (
            SketchPlane {
                origin: Vec3::ZERO,
                normal: Vec3::Y,
                u: Vec3::X,
                v: Vec3::Z,
            },
            "XZ Plane",
        ),
        BaseSketchPlane::YZ => (
            SketchPlane {
                origin: Vec3::ZERO,
                normal: Vec3::X,
                u: Vec3::Y,
                v: Vec3::Z,
            },
            "YZ Plane",
        ),
    }
}

fn sketch_plane_from_surface(hit: SurfaceHit) -> SketchPlane {
    let origin = Vec3::from_array(hit.point);
    let mut normal = Vec3::from_array(hit.normal).normalize_or_zero();
    if normal.length_squared() < 1.0e-6 {
        normal = Vec3::Z;
    }
    let helper = if normal.dot(Vec3::Y).abs() < 0.95 {
        Vec3::Y
    } else {
        Vec3::X
    };
    let mut u = helper.cross(normal).normalize_or_zero();
    if u.length_squared() < 1.0e-6 {
        u = Vec3::X;
    }
    let v = normal.cross(u).normalize_or_zero();
    SketchPlane {
        origin,
        normal,
        u,
        v,
    }
}

fn ray_plane_intersection(ray_o: Vec3, ray_d: Vec3, plane: SketchPlane) -> Option<Vec3> {
    let denom = plane.normal.dot(ray_d);
    if denom.abs() < 1.0e-6 {
        return None;
    }
    let t = plane.normal.dot(plane.origin - ray_o) / denom;
    if t <= 0.0 {
        return None;
    }
    Some(ray_o + ray_d * t)
}

fn snap_sketch_point(point: Vec3, plane: SketchPlane, step: f32) -> Vec3 {
    let rel = point - plane.origin;
    let u = (rel.dot(plane.u) / step).round() * step;
    let v = (rel.dot(plane.v) / step).round() * step;
    plane.origin + plane.u * u + plane.v * v
}

fn add_sketch_grid(lines: &mut Vec<OverlayLine>, plane: SketchPlane, half_steps: i32, step: f32) {
    let extent = half_steps as f32 * step;
    for i in -half_steps..=half_steps {
        let t = i as f32 * step;
        let is_axis = i == 0;
        let color_u = if is_axis {
            [0.3, 0.75, 1.0]
        } else {
            [0.3, 0.36, 0.46]
        };
        let color_v = if is_axis {
            [0.55, 0.85, 0.5]
        } else {
            [0.3, 0.36, 0.46]
        };
        let a = plane.origin + plane.u * t - plane.v * extent;
        let b = plane.origin + plane.u * t + plane.v * extent;
        let c = plane.origin + plane.v * t - plane.u * extent;
        let d = plane.origin + plane.v * t + plane.u * extent;
        lines.push(OverlayLine {
            a: a.to_array(),
            b: b.to_array(),
            color: color_u,
        });
        lines.push(OverlayLine {
            a: c.to_array(),
            b: d.to_array(),
            color: color_v,
        });
    }

    let corners = [
        plane.origin + plane.u * extent + plane.v * extent,
        plane.origin - plane.u * extent + plane.v * extent,
        plane.origin - plane.u * extent - plane.v * extent,
        plane.origin + plane.u * extent - plane.v * extent,
    ];
    for i in 0..4 {
        lines.push(OverlayLine {
            a: corners[i].to_array(),
            b: corners[(i + 1) % 4].to_array(),
            color: [0.6, 0.68, 0.86],
        });
    }
}

fn update_sketch_overlay(
    renderer: &Rc<RefCell<Option<Renderer>>>,
    plane: Option<SketchPlane>,
    segments: &[SketchSegment],
    anchor: Option<Vec3>,
    cursor: Option<Vec3>,
) {
    let mut renderer_borrow = renderer.borrow_mut();
    let Some(renderer) = renderer_borrow.as_mut() else {
        return;
    };
    let Some(plane) = plane else {
        renderer.clear_overlay_lines();
        renderer.render();
        return;
    };

    let mut lines = Vec::new();
    add_sketch_grid(&mut lines, plane, 16, 0.1);

    for seg in segments {
        lines.push(OverlayLine {
            a: seg.a.to_array(),
            b: seg.b.to_array(),
            color: [0.34, 0.58, 1.0],
        });
    }

    if let (Some(a), Some(c)) = (anchor, cursor) {
        lines.push(OverlayLine {
            a: a.to_array(),
            b: c.to_array(),
            color: [1.0, 0.82, 0.28],
        });
    }

    renderer.set_overlay_lines(lines);
    renderer.render();
}

fn animate_camera_to_sketch_plane(renderer: Rc<RefCell<Option<Renderer>>>, plane: SketchPlane) {
    let (start_target, start_radius, start_rot) = {
        let mut renderer_borrow = renderer.borrow_mut();
        let Some(r) = renderer_borrow.as_mut() else {
            return;
        };
        let (target, radius) = r.camera_target_radius();
        let rotation = Quat::from_array(r.camera_rotation()).normalize();
        (Vec3::from_array(target), radius, rotation)
    };

    let end_target = plane.origin;
    let end_rot = snap_camera_rotation(start_rot, plane.normal, plane.v);
    let end_radius = (start_radius * 0.58).clamp(1.0, 30.0);
    let start_ms = Date::now();
    let duration_ms = 520.0;

    let raf = Rc::new(RefCell::new(None::<Closure<dyn FnMut(f64)>>));
    let raf_clone = raf.clone();
    let renderer_for_cb = renderer.clone();

    *raf.borrow_mut() = Some(Closure::wrap(Box::new(move |time: f64| {
        let t = ((time - start_ms) / duration_ms).clamp(0.0, 1.0) as f32;
        let ease = if t < 0.5 {
            4.0 * t * t * t
        } else {
            1.0 - (-2.0 * t + 2.0).powi(3) / 2.0
        };

        let target = start_target.lerp(end_target, ease);
        let rotation = start_rot.slerp(end_rot, ease).normalize();
        let radius = start_radius + (end_radius - start_radius) * ease;

        if let Some(r) = renderer_for_cb.borrow_mut().as_mut() {
            r.set_camera_view(target.to_array(), rotation.to_array(), radius);
            r.render();
        }

        if t < 1.0 {
            if let Some(window) = web_sys::window() {
                if let Some(cb) = raf_clone.borrow().as_ref() {
                    let _ = window.request_animation_frame(cb.as_ref().unchecked_ref());
                }
            }
        } else {
            raf_clone.borrow_mut().take();
        }
    }) as Box<dyn FnMut(f64)>));

    if let Some(window) = web_sys::window() {
        if let Some(cb) = raf.borrow().as_ref() {
            let _ = window.request_animation_frame(cb.as_ref().unchecked_ref());
        }
    }
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
    sketch_plane: ReadSignal<Option<SketchPlane>>,
    sketch_segments: ReadSignal<Vec<SketchSegment>>,
    set_sketch_segments: WriteSignal<Vec<SketchSegment>>,
    sketch_anchor: ReadSignal<Option<Vec3>>,
    set_sketch_anchor: WriteSignal<Option<Vec3>>,
    set_sketch_cursor: WriteSignal<Option<Vec3>>,
    enter_sketch_draw: Rc<dyn Fn(SketchPlane, String)>,
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
        let sketch_plane = sketch_plane;
        let sketch_segments = sketch_segments;
        let set_sketch_segments = set_sketch_segments;
        let sketch_anchor = sketch_anchor;
        let set_sketch_anchor = set_sketch_anchor;
        let set_sketch_cursor = set_sketch_cursor;
        let enter_sketch_draw = enter_sketch_draw.clone();
        let closure = Closure::wrap(Box::new(move |event: web_sys::Event| {
            let event = event.dyn_into::<MouseEvent>().unwrap();
            if event.button() != 0 {
                return;
            }
            let (ray_o, ray_d, mode, gizmo_hit) = {
                let renderer_borrow = renderer.borrow();
                let Some(r) = renderer_borrow.as_ref() else {
                    return;
                };

                let (cursor_x, cursor_y, w, h) = canvas_cursor(&canvas_for_closure, &event);
                let (ray_o, ray_d) = r.screen_ray(cursor_x, cursor_y, w, h);
                let ray_o = Vec3::from_array(ray_o);
                let ray_d = Vec3::from_array(ray_d);
                let mode = tool_mode.get_untracked();

                let gizmo_hit = if mode == EditorTool::Move {
                    selected_id
                        .get_untracked()
                        .and_then(|id| hit_gizmo(&scene, r, id, ray_o, ray_d).map(|hit| (id, hit)))
                } else {
                    None
                };
                (ray_o, ray_d, mode, gizmo_hit)
            };

            if mode == EditorTool::SketchSelect {
                event.prevent_default();
                if let Some(hit) = scene
                    .borrow()
                    .pick_surface(ray_o.to_array(), ray_d.to_array())
                {
                    set_selected_id.set(Some(hit.object_id));
                    if let Some(t) = scene.borrow().object_transform(hit.object_id) {
                        set_baseline_transform.set(Some(t));
                        set_transform_ui.set(TransformUi::from_transform(t));
                    }
                    let plane = sketch_plane_from_surface(hit);
                    (enter_sketch_draw.as_ref())(plane, format!("Body {} Face", hit.object_id + 1));
                }
                return;
            }

            if mode == EditorTool::SketchDraw {
                event.prevent_default();
                let Some(plane) = sketch_plane.get_untracked() else {
                    return;
                };
                let Some(hit) = ray_plane_intersection(ray_o, ray_d, plane) else {
                    return;
                };
                let snapped = snap_sketch_point(hit, plane, 0.1);
                set_sketch_cursor.set(Some(snapped));
                if let Some(anchor) = sketch_anchor.get_untracked() {
                    if (snapped - anchor).length() > 1.0e-4 {
                        set_sketch_segments.update(|segments| {
                            segments.push(SketchSegment {
                                a: anchor,
                                b: snapped,
                            });
                        });
                        set_sketch_anchor.set(Some(snapped));
                    }
                } else {
                    set_sketch_anchor.set(Some(snapped));
                }
                let segments = sketch_segments.get_untracked();
                update_sketch_overlay(
                    &renderer,
                    Some(plane),
                    &segments,
                    sketch_anchor.get_untracked(),
                    Some(snapped),
                );
                return;
            }

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
            let canvas_el = canvas_el.clone();
            let renderer = renderer.clone();
            let drag_state = drag_state.clone();
            let sketch_plane = sketch_plane;
            let sketch_segments = sketch_segments;
            let sketch_anchor = sketch_anchor;
            let set_sketch_cursor = set_sketch_cursor;
            let closure = Closure::wrap(Box::new(move |event: web_sys::Event| {
                if drag_state.borrow().is_some() {
                    return;
                }
                if tool_mode.get_untracked() != EditorTool::SketchDraw {
                    return;
                }
                let Some(plane) = sketch_plane.get_untracked() else {
                    return;
                };

                let event = event.dyn_into::<MouseEvent>().unwrap();
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
                if let Some(hit) = ray_plane_intersection(ray_o, ray_d, plane) {
                    let snapped = snap_sketch_point(hit, plane, 0.1);
                    set_sketch_cursor.set(Some(snapped));
                    let segments = sketch_segments.get_untracked();
                    update_sketch_overlay(
                        &renderer,
                        Some(plane),
                        &segments,
                        sketch_anchor.get_untracked(),
                        Some(snapped),
                    );
                }
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
            let set_sketch_anchor = set_sketch_anchor;
            let set_sketch_cursor = set_sketch_cursor;
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
                    set_sketch_anchor.set(None);
                    set_sketch_cursor.set(None);
                } else if key == "Escape" {
                    event.prevent_default();
                    set_tool_mode.set(EditorTool::None);
                    set_sketch_anchor.set(None);
                    set_sketch_cursor.set(None);
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

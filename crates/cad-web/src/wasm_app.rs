use cad_geom::GeomScene;
use cad_protocol::{ClientMsg, ServerMsg};
use cad_render::Renderer;
use leptos::html::Canvas;
use leptos::prelude::*;
use std::cell::RefCell;
use std::rc::Rc;
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::spawn_local;
use web_sys::{MessageEvent, WebSocket};

#[wasm_bindgen(start)]
pub fn start() {
    console_error_panic_hook::set_once();
    mount_to_body(|| view! { <App /> });
}

#[component]
fn App() -> impl IntoView {
    let canvas_ref = NodeRef::<Canvas>::new();
    let scene = Rc::new(RefCell::new(GeomScene::new()));
    let renderer = Rc::new(RefCell::new(None::<Renderer>));
    let ws_handle = Rc::new(RefCell::new(None::<WebSocket>));
    let (plane_xy, set_plane_xy) = signal(true);
    let (plane_yz, set_plane_yz) = signal(false);
    let (plane_zx, set_plane_zx) = signal(false);

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
        plane_xy,
        plane_yz,
        plane_zx,
    );

    let on_add_box = {
        let scene = scene.clone();
        let renderer = renderer.clone();
        move |_| {
            scene.borrow_mut().add_box(1.0, 1.0, 1.0);
            update_mesh(&scene, &renderer);
        }
    };

    let on_add_cylinder = {
        let scene = scene.clone();
        let renderer = renderer.clone();
        move |_| {
            scene.borrow_mut().add_cylinder(0.5, 1.5);
            update_mesh(&scene, &renderer);
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

    view! {
        <div class="app">
            <aside class="panel">
                <h1>"physalis"</h1>
                <div class="buttons">
                    <button on:click=on_add_box>"Add Box"</button>
                    <button on:click=on_add_cylinder>"Add Cylinder"</button>
                    <button on:click=on_boolean_stub>"Boolean Subtract (A - B)"</button>
                    <button on:click=on_export_stub>"Export STEP"</button>
                </div>
                <div class="planes">
                    <h2>"Planes"</h2>
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
            </aside>
            <main class="viewport">
                <canvas id="viewport-canvas" node_ref=canvas_ref></canvas>
            </main>
        </div>
    }
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
    plane_xy: ReadSignal<bool>,
    plane_yz: ReadSignal<bool>,
    plane_zx: ReadSignal<bool>,
) {
    let renderer = renderer.clone();
    let plane_xy = plane_xy.clone();
    let plane_yz = plane_yz.clone();
    let plane_zx = plane_zx.clone();
    request_animation_frame(move || {
        if let Some(canvas) = canvas_ref.get() {
            let renderer = renderer.clone();
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
                    }
                    Err(err) => {
                        log(&format!("renderer init failed: {err}"));
                    }
                }
            });
        } else {
            // Canvas not ready yet, try again on the next frame.
            schedule_renderer_init(canvas_ref, renderer, plane_xy, plane_yz, plane_zx);
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

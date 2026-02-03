use axum::{
    extract::{ws::Message, ws::WebSocket, ws::WebSocketUpgrade, State},
    response::{IntoResponse, Redirect},
    routing::get,
    Router,
};
use cad_protocol::{ClientMsg, ServerMsg};
use futures_util::{SinkExt, StreamExt};
use std::{
    path::PathBuf,
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc,
    },
    time::Duration,
};
use tokio::sync::mpsc;
use tower_http::services::{ServeDir, ServeFile};
use tower_http::trace::TraceLayer;
use tracing::{info, warn};

#[derive(Clone)]
struct AppState {
    job_tx: mpsc::Sender<HeavyJob>,
    next_job_id: Arc<AtomicU64>,
}

struct HeavyJob {
    id: u64,
    kind: String,
    payload: Option<String>,
    respond_to: mpsc::Sender<ServerMsg>,
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    let (job_tx, job_rx) = mpsc::channel(64);
    tokio::spawn(job_worker(job_rx));

    let state = AppState {
        job_tx,
        next_job_id: Arc::new(AtomicU64::new(1)),
    };

    let dist_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../web/dist");
    let index_file = dist_dir.join("index.html");

    let app = Router::new()
        .route(
            "/favicon.ico",
            get(|| async { Redirect::temporary("/icon.svg") }),
        )
        .route("/ws", get(ws_handler))
        .nest_service(
            "/",
            ServeDir::new(dist_dir.clone()).append_index_html_on_directories(true),
        )
        .fallback_service(ServeFile::new(index_file))
        .with_state(state)
        .layer(TraceLayer::new_for_http());

    let addr = "0.0.0.0:8080";
    info!("listening on http://{addr}");
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

async fn ws_handler(ws: WebSocketUpgrade, State(state): State<AppState>) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_socket(socket, state))
}

async fn handle_socket(socket: WebSocket, state: AppState) {
    let (mut ws_tx, mut ws_rx) = socket.split();
    let (out_tx, mut out_rx) = mpsc::channel::<ServerMsg>(32);

    let send_task = tokio::spawn(async move {
        while let Some(msg) = out_rx.recv().await {
            if let Ok(text) = serde_json::to_string(&msg) {
                if ws_tx.send(Message::Text(text)).await.is_err() {
                    break;
                }
            }
        }
    });

    let _ = out_tx.send(ServerMsg::HelloAck).await;

    while let Some(Ok(msg)) = ws_rx.next().await {
        match msg {
            Message::Text(text) => {
                if let Ok(client_msg) = serde_json::from_str::<ClientMsg>(&text) {
                    match client_msg {
                        ClientMsg::Hello { client_version } => {
                            let _ = out_tx.send(ServerMsg::HelloAck).await;
                            let _ = out_tx
                                .send(ServerMsg::Log {
                                    text: format!("client hello: {client_version}"),
                                })
                                .await;
                        }
                        ClientMsg::AddBox { .. } | ClientMsg::AddCylinder { .. } => {
                            let _ = out_tx
                                .send(ServerMsg::Log {
                                    text: "received add-primitive".to_string(),
                                })
                                .await;
                        }
                        ClientMsg::RequestHeavy { kind, payload } => {
                            let job_id = state.next_job_id.fetch_add(1, Ordering::Relaxed);
                            let job = HeavyJob {
                                id: job_id,
                                kind,
                                payload,
                                respond_to: out_tx.clone(),
                            };
                            if state.job_tx.send(job).await.is_ok() {
                                let _ = out_tx.send(ServerMsg::JobAccepted { job_id }).await;
                            } else {
                                let _ = out_tx
                                    .send(ServerMsg::Log {
                                        text: "job queue unavailable".to_string(),
                                    })
                                    .await;
                            }
                        }
                    }
                } else {
                    let _ = out_tx
                        .send(ServerMsg::Log {
                            text: format!("unrecognized payload: {text}"),
                        })
                        .await;
                }
            }
            Message::Binary(_) => {
                let _ = out_tx
                    .send(ServerMsg::Log {
                        text: "binary message ignored".to_string(),
                    })
                    .await;
            }
            Message::Close(_) => break,
            _ => {}
        }
    }

    drop(out_tx);
    let _ = send_task.await;
    warn!("websocket closed");
}

async fn job_worker(mut rx: mpsc::Receiver<HeavyJob>) {
    while let Some(job) = rx.recv().await {
        let respond_to = job.respond_to.clone();
        let job_id = job.id;
        let kind = job.kind.clone();
        let payload = job.payload.clone();

        let result = tokio::task::spawn_blocking(move || {
            std::thread::sleep(Duration::from_millis(300));
            let details = payload.unwrap_or_else(|| "no-payload".to_string());
            format!("heavy job done: {kind} ({details})")
        })
        .await;

        if let Ok(payload) = result {
            let _ = respond_to
                .send(ServerMsg::JobResult { job_id, payload })
                .await;
        }
    }
}

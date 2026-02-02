# physalis

MVP CAD monorepo (Cargo workspace) with a Leptos + wgpu + Truck client and an axum + tokio server.

## Workspace layout

```
/crates
  /cad-core        shared model types
  /cad-geom        Truck-backed geometry + tessellation
  /cad-render      wgpu renderer (wasm)
  /cad-protocol    serde message types
  /cad-server      axum server + WS + task queue
  /cad-web         Leptos SPA (wasm)
/assets            sample assets
/web               Trunk web assets (index.html, css, config)
```

## Prerequisites

- Rust 1.92+ (wgpu MSRV)
- wasm32 target: `rustup target add wasm32-unknown-unknown`
- Trunk: `cargo install trunk`

## Build & run

### 1) Build the web client (static assets)

```
trunk build --config web/Trunk.toml --release
```

This outputs static files to `web/dist`.

### 2) Run the server

```
cargo run -p cad-server
```

Server listens on `http://localhost:8080` and serves `web/dist` plus the WebSocket endpoint at `/ws`.

## Dev workflow

- Run the server (API + WS):
  ```
  cargo run -p cad-server
  ```
- In another terminal, run the SPA dev server:
  ```
  trunk serve --config web/Trunk.toml
  ```

The frontend will connect to `ws://localhost:8080/ws` on startup.

## Notes

- `cad-geom` separates model data (`cad-core`) from render meshes and caches tessellated meshes.
- `Boolean Subtract` and `Export STEP` are stubs with TODOs for future work.
- Heavy server jobs are queued via `tokio::mpsc` and executed in `spawn_blocking`.

## Next extensions

- Add parametric history to `cad-core` (feature tree + constraints).
- Add real boolean ops/STEP export in `cad-geom`.
- Extend the WS protocol with scene sync and server-side meshing jobs.

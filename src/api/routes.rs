use axum::{
    body::Body,
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        Path, Query, State,
    },
    http::{header, StatusCode},
    response::{
        sse::{Event, Sse},
        IntoResponse, Response,
    },
    routing::{delete, get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::{
    collections::HashMap,
    path::{Path as FsPath, PathBuf},
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc,
    },
    time::Duration,
};
use tokio::sync::broadcast;
use tokio_stream::{wrappers::BroadcastStream, StreamExt};

use crate::serial::manager::{
    list_ports, ConnectionInfo, ConnectionManager, ConnectionRequest, InMemoryConnectionManager,
    PortInfo, SerialPortLister, SerialStreamEvent, SystemPortLister,
};
use crate::storage::{CreatePreset, InMemoryPresetStore, Preset, PresetStore, PresetStoreError};

#[derive(Debug, Serialize)]
struct HealthResponse {
    status: &'static str,
    version: &'static str,
}

#[derive(Debug, Serialize)]
struct PortsResponse {
    ports: Vec<PortInfo>,
}

#[derive(Debug, Serialize)]
struct ConnectResponse {
    status: &'static str,
    connection: ConnectionInfo,
}

#[derive(Debug, Serialize)]
struct ConnectionsResponse {
    connections: Vec<ConnectionInfo>,
}

#[derive(Debug, Serialize)]
struct DisconnectResponse {
    status: &'static str,
    name: String,
}

#[derive(Debug, Deserialize)]
struct DisconnectRequest {
    name: String,
}

#[derive(Debug, Deserialize)]
struct CommandRequest {
    payload: Value,
    #[serde(rename = "waitForResponse", default)]
    wait_for_response: bool,
    #[serde(rename = "timeoutMs")]
    timeout_ms: Option<u64>,
}

#[derive(Debug, Serialize)]
struct CommandResponse {
    status: &'static str,
    #[serde(rename = "reqId")]
    req_id: String,
}

#[derive(Debug, Serialize)]
struct WaitedCommandResponse {
    status: &'static str,
    #[serde(rename = "reqId")]
    req_id: String,
    response: Value,
}

#[derive(Debug, Serialize)]
struct PresetsResponse {
    presets: Vec<Preset>,
}

#[derive(Debug, Serialize)]
struct PresetResponse {
    preset: Preset,
}

#[derive(Debug, Serialize)]
struct DeletePresetResponse {
    status: &'static str,
    id: i64,
}

#[derive(Clone)]
pub struct AppState<L, C> {
    port_lister: L,
    connection_manager: C,
    preset_store: Arc<dyn PresetStore>,
    dashboard_assets: PathBuf,
}

impl<L, C> AppState<L, C> {
    pub fn new(port_lister: L, connection_manager: C) -> Self {
        Self {
            port_lister,
            connection_manager,
            preset_store: Arc::new(InMemoryPresetStore::default()),
            dashboard_assets: default_dashboard_assets_dir(),
        }
    }

    pub fn with_preset_store<P>(port_lister: L, connection_manager: C, preset_store: P) -> Self
    where
        P: PresetStore,
    {
        Self {
            port_lister,
            connection_manager,
            preset_store: Arc::new(preset_store),
            dashboard_assets: default_dashboard_assets_dir(),
        }
    }

    pub fn with_preset_store_arc(
        port_lister: L,
        connection_manager: C,
        preset_store: Arc<dyn PresetStore>,
    ) -> Self {
        Self {
            port_lister,
            connection_manager,
            preset_store,
            dashboard_assets: default_dashboard_assets_dir(),
        }
    }

    pub fn with_dashboard_assets<P>(mut self, dashboard_assets: P) -> Self
    where
        P: Into<PathBuf>,
    {
        self.dashboard_assets = dashboard_assets.into();
        self
    }
}

fn default_dashboard_assets_dir() -> PathBuf {
    let built = PathBuf::from("web/dist");
    if built.join("index.html").is_file() {
        return built;
    }

    let packaged = PathBuf::from("web");
    if is_packaged_dashboard_dir(&packaged) {
        packaged
    } else {
        built
    }
}

fn is_packaged_dashboard_dir(root: &FsPath) -> bool {
    if !root.join("assets").is_dir() {
        return false;
    }

    let Ok(index) = std::fs::read_to_string(root.join("index.html")) else {
        return false;
    };

    index_references_built_asset(&index, ".js") || index_references_built_asset(&index, ".css")
}

fn index_references_built_asset(index: &str, extension: &str) -> bool {
    index.match_indices("/assets/").any(|(start, _)| {
        let asset_ref = &index[start..];
        let end = asset_ref
            .find(['"', '\'', '<', '>', ' '])
            .unwrap_or(asset_ref.len());
        asset_ref[..end].ends_with(extension)
    })
}

pub fn router() -> Router {
    router_with_state(AppState {
        port_lister: SystemPortLister,
        connection_manager: InMemoryConnectionManager::default(),
        preset_store: Arc::new(InMemoryPresetStore::default()),
        dashboard_assets: default_dashboard_assets_dir(),
    })
}

pub fn router_with_port_lister<L>(port_lister: L) -> Router
where
    L: SerialPortLister,
{
    router_with_state(AppState {
        port_lister,
        connection_manager: InMemoryConnectionManager::default(),
        preset_store: Arc::new(InMemoryPresetStore::default()),
        dashboard_assets: default_dashboard_assets_dir(),
    })
}

pub fn router_with_state<L, C>(state: AppState<L, C>) -> Router
where
    L: SerialPortLister,
    C: ConnectionManager,
{
    Router::new()
        .route("/", get(dashboard::<L, C>))
        .route("/dashboard", get(dashboard::<L, C>))
        .route("/assets/:file", get(dashboard_asset::<L, C>))
        .route("/api/v1/health", get(health))
        .route("/api/v1/events", get(events::<L, C>))
        .route("/api/v1/events/ws", get(events_ws::<L, C>))
        .route("/socket.io/", get(socket_io_events::<L, C>))
        .route("/api/v1/ports", get(ports::<L, C>))
        .route("/list", get(ports::<L, C>))
        .route(
            "/api/v1/presets",
            get(list_presets::<L, C>).post(create_preset::<L, C>),
        )
        .route(
            "/api/v1/presets/:id",
            get(get_preset::<L, C>)
                .put(update_preset::<L, C>)
                .delete(delete_preset::<L, C>),
        )
        .route(
            "/api/v1/connections",
            post(connect::<L, C>).get(connections::<L, C>),
        )
        .route(
            "/api/v1/connections/:name/commands",
            post(send_command::<L, C>),
        )
        .route("/api/v1/connections/:name", delete(disconnect::<L, C>))
        .route("/connect", post(connect::<L, C>))
        .route("/commit", post(commit_alias::<L, C>))
        .route("/info", get(connections::<L, C>))
        .route("/disconnect", post(disconnect_alias::<L, C>))
        .with_state(state)
}

async fn dashboard<L, C>(State(state): State<AppState<L, C>>) -> Response
where
    L: SerialPortLister,
    C: ConnectionManager,
{
    serve_dashboard_index(&state.dashboard_assets)
}

async fn dashboard_asset<L, C>(
    State(state): State<AppState<L, C>>,
    Path(file): Path<String>,
) -> Response
where
    L: SerialPortLister,
    C: ConnectionManager,
{
    if file.contains('/') || file.contains("..") {
        return StatusCode::NOT_FOUND.into_response();
    }
    serve_dashboard_file(&state.dashboard_assets.join("assets").join(file))
}

fn serve_dashboard_index(root: &FsPath) -> Response {
    let index = root.join("index.html");
    match std::fs::read_to_string(index) {
        Ok(html) => (
            [
                (header::CONTENT_TYPE, "text/html; charset=utf-8"),
                (header::CACHE_CONTROL, "no-cache"),
            ],
            html,
        )
            .into_response(),
        Err(_) => (
            StatusCode::SERVICE_UNAVAILABLE,
            [(header::CONTENT_TYPE, "text/html; charset=utf-8")],
            missing_dashboard_html(root),
        )
            .into_response(),
    }
}

fn serve_dashboard_file(path: &FsPath) -> Response {
    match std::fs::read(path) {
        Ok(bytes) => {
            let content_type = content_type_for(path);
            Response::builder()
                .status(StatusCode::OK)
                .header(header::CONTENT_TYPE, content_type)
                .header(header::CACHE_CONTROL, "public, max-age=31536000, immutable")
                .body(Body::from(bytes))
                .unwrap_or_else(|_| StatusCode::INTERNAL_SERVER_ERROR.into_response())
        }
        Err(_) => StatusCode::NOT_FOUND.into_response(),
    }
}

fn content_type_for(path: &FsPath) -> &'static str {
    match path.extension().and_then(|extension| extension.to_str()) {
        Some("css") => "text/css; charset=utf-8",
        Some("js") => "text/javascript; charset=utf-8",
        Some("map") => "application/json; charset=utf-8",
        Some("svg") => "image/svg+xml",
        Some("png") => "image/png",
        Some("jpg" | "jpeg") => "image/jpeg",
        Some("webp") => "image/webp",
        Some("ico") => "image/x-icon",
        _ => "application/octet-stream",
    }
}

fn missing_dashboard_html(root: &FsPath) -> String {
    format!(
        r#"<!doctype html><html><head><title>Dashboard not built</title></head><body><h1>Dashboard assets are not available</h1><p>Build the dashboard with <code>cd web && pnpm install --frozen-lockfile && pnpm build</code>.</p><p>Expected runtime directory: <code>{}</code></p></body></html>"#,
        root.display()
    )
}

async fn health() -> Json<HealthResponse> {
    Json(HealthResponse {
        status: "ok",
        version: env!("CARGO_PKG_VERSION"),
    })
}

async fn ports<L, C>(State(state): State<AppState<L, C>>) -> Result<Json<PortsResponse>, StatusCode>
where
    L: SerialPortLister,
    C: ConnectionManager,
{
    let ports = list_ports(&state.port_lister).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(PortsResponse { ports }))
}

async fn events<L, C>(
    State(state): State<AppState<L, C>>,
) -> Result<
    Sse<impl tokio_stream::Stream<Item = std::result::Result<Event, axum::Error>>>,
    StatusCode,
>
where
    L: SerialPortLister,
    C: ConnectionManager,
{
    let receiver = state
        .connection_manager
        .subscribe_events()
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let events = state
        .connection_manager
        .events()
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let stream = live_event_stream(events, receiver).map(serial_event_to_sse_event);

    Ok(Sse::new(stream))
}

async fn events_ws<L, C>(
    State(state): State<AppState<L, C>>,
    ws: WebSocketUpgrade,
) -> Result<impl IntoResponse, StatusCode>
where
    L: SerialPortLister,
    C: ConnectionManager,
{
    let receiver = state
        .connection_manager
        .subscribe_events()
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let events = state
        .connection_manager
        .events()
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(ws.on_upgrade(move |socket| send_live_events(socket, events, receiver)))
}

async fn socket_io_events<L, C>(
    State(state): State<AppState<L, C>>,
    Query(query): Query<HashMap<String, String>>,
    ws: WebSocketUpgrade,
) -> Result<impl IntoResponse, StatusCode>
where
    L: SerialPortLister,
    C: ConnectionManager,
{
    if query.get("EIO").map(String::as_str) != Some("4")
        || query.get("transport").map(String::as_str) != Some("websocket")
    {
        return Err(StatusCode::BAD_REQUEST);
    }

    let receiver = state
        .connection_manager
        .subscribe_events()
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let events = state
        .connection_manager
        .events()
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(ws.on_upgrade(move |socket| send_socket_io_live_events(socket, events, receiver)))
}

fn live_event_stream(
    snapshot: Vec<SerialStreamEvent>,
    receiver: broadcast::Receiver<SerialStreamEvent>,
) -> impl tokio_stream::Stream<Item = SerialStreamEvent> {
    let snapshot = tokio_stream::iter(snapshot);
    let live = BroadcastStream::new(receiver).filter_map(|result| match result {
        Ok(event) => Some(event),
        Err(tokio_stream::wrappers::errors::BroadcastStreamRecvError::Lagged(count)) => {
            Some(SerialStreamEvent {
                event: "serial.error",
                data: json!(format!("serial event stream lagged by {count} events")),
            })
        }
    });

    snapshot.chain(live)
}

fn serial_event_to_sse_event(
    serial_event: SerialStreamEvent,
) -> std::result::Result<Event, axum::Error> {
    Event::default()
        .event(serial_event.event)
        .json_data(serial_event.data)
}

fn serial_event_to_ws_text(serial_event: SerialStreamEvent) -> Option<String> {
    let payload = json!({
        "event": serial_event.event,
        "data": serial_event.data,
    });
    serde_json::to_string(&payload).ok()
}

fn serial_event_to_socket_io_frame(serial_event: SerialStreamEvent) -> Option<String> {
    let payload = json!([serial_event.event, serial_event.data]);
    serde_json::to_string(&payload)
        .ok()
        .map(|text| format!("42{text}"))
}

async fn send_live_events(
    mut socket: WebSocket,
    events: Vec<SerialStreamEvent>,
    mut receiver: broadcast::Receiver<SerialStreamEvent>,
) {
    for serial_event in events {
        let Some(text) = serial_event_to_ws_text(serial_event) else {
            break;
        };

        if socket.send(Message::Text(text)).await.is_err() {
            return;
        }
    }

    loop {
        tokio::select! {
            result = receiver.recv() => match result {
                Ok(serial_event) => {
                    let Some(text) = serial_event_to_ws_text(serial_event) else { continue; };
                    if socket.send(Message::Text(text)).await.is_err() {
                        break;
                    }
                }
                Err(broadcast::error::RecvError::Lagged(_)) => continue,
                Err(broadcast::error::RecvError::Closed) => break,
            },
            message = socket.recv() => match message {
                Some(Ok(Message::Close(_))) | None => break,
                Some(Ok(_)) => continue,
                Some(Err(_)) => break,
            },
        }
    }
}

async fn send_socket_io_live_events(
    mut socket: WebSocket,
    events: Vec<SerialStreamEvent>,
    mut receiver: broadcast::Receiver<SerialStreamEvent>,
) {
    let open_payload = json!({
        "sid": next_socket_io_sid(),
        "upgrades": [],
        "pingInterval": 25000,
        "pingTimeout": 20000,
        "maxPayload": 1000000,
    });

    let Ok(open_text) = serde_json::to_string(&open_payload) else {
        let _ = socket.close().await;
        return;
    };

    if socket
        .send(Message::Text(format!("0{open_text}")))
        .await
        .is_err()
    {
        return;
    }

    if socket.send(Message::Text("40".to_string())).await.is_err() {
        return;
    }

    for serial_event in events {
        let Some(text) = serial_event_to_socket_io_frame(serial_event) else {
            break;
        };

        if socket.send(Message::Text(text)).await.is_err() {
            return;
        }
    }

    loop {
        tokio::select! {
            result = receiver.recv() => match result {
                Ok(serial_event) => {
                    let Some(text) = serial_event_to_socket_io_frame(serial_event) else { continue; };
                    if socket.send(Message::Text(text)).await.is_err() {
                        break;
                    }
                }
                Err(broadcast::error::RecvError::Lagged(_)) => continue,
                Err(broadcast::error::RecvError::Closed) => break,
            },
            message = socket.recv() => match message {
                Some(Ok(Message::Close(_))) | None => break,
                Some(Ok(_)) => continue,
                Some(Err(_)) => break,
            },
        }
    }
}

fn next_socket_io_sid() -> String {
    static NEXT_SID: AtomicU64 = AtomicU64::new(1);
    format!(
        "serialport-api-{}",
        NEXT_SID.fetch_add(1, Ordering::Relaxed)
    )
}

async fn list_presets<L, C>(
    State(state): State<AppState<L, C>>,
) -> Result<Json<PresetsResponse>, axum::response::Response>
where
    L: SerialPortLister,
    C: ConnectionManager,
{
    state
        .preset_store
        .list()
        .map(|presets| Json(PresetsResponse { presets }))
        .map_err(preset_error_response)
}

async fn create_preset<L, C>(
    State(state): State<AppState<L, C>>,
    Json(request): Json<CreatePreset>,
) -> Result<axum::response::Response, axum::response::Response>
where
    L: SerialPortLister,
    C: ConnectionManager,
{
    state
        .preset_store
        .create(request)
        .map(|preset| (StatusCode::CREATED, Json(PresetResponse { preset })).into_response())
        .map_err(preset_error_response)
}

async fn get_preset<L, C>(
    State(state): State<AppState<L, C>>,
    Path(id): Path<i64>,
) -> Result<Json<PresetResponse>, axum::response::Response>
where
    L: SerialPortLister,
    C: ConnectionManager,
{
    state
        .preset_store
        .get(id)
        .map(|preset| Json(PresetResponse { preset }))
        .map_err(preset_error_response)
}

async fn update_preset<L, C>(
    State(state): State<AppState<L, C>>,
    Path(id): Path<i64>,
    Json(request): Json<CreatePreset>,
) -> Result<Json<PresetResponse>, axum::response::Response>
where
    L: SerialPortLister,
    C: ConnectionManager,
{
    state
        .preset_store
        .update(id, request)
        .map(|preset| Json(PresetResponse { preset }))
        .map_err(preset_error_response)
}

async fn delete_preset<L, C>(
    State(state): State<AppState<L, C>>,
    Path(id): Path<i64>,
) -> Result<Json<DeletePresetResponse>, axum::response::Response>
where
    L: SerialPortLister,
    C: ConnectionManager,
{
    state
        .preset_store
        .delete(id)
        .map(|id| {
            Json(DeletePresetResponse {
                status: "deleted",
                id,
            })
        })
        .map_err(preset_error_response)
}

fn preset_error_response(error: PresetStoreError) -> axum::response::Response {
    let status = match error {
        PresetStoreError::InvalidName | PresetStoreError::InvalidPayload => StatusCode::BAD_REQUEST,
        PresetStoreError::NotFound(_) => StatusCode::NOT_FOUND,
        PresetStoreError::Storage(_) => StatusCode::INTERNAL_SERVER_ERROR,
    };
    (status, Json(json!({"error": error.to_string()}))).into_response()
}

async fn connect<L, C>(
    State(state): State<AppState<L, C>>,
    Json(request): Json<ConnectionRequest>,
) -> Result<Json<ConnectResponse>, StatusCode>
where
    L: SerialPortLister,
    C: ConnectionManager,
{
    let connection = state
        .connection_manager
        .connect(request)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(ConnectResponse {
        status: "connected",
        connection,
    }))
}

async fn connections<L, C>(
    State(state): State<AppState<L, C>>,
) -> Result<Json<ConnectionsResponse>, StatusCode>
where
    L: SerialPortLister,
    C: ConnectionManager,
{
    let connections = state
        .connection_manager
        .connections()
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(ConnectionsResponse { connections }))
}

async fn send_command<L, C>(
    State(state): State<AppState<L, C>>,
    Path(name): Path<String>,
    Json(request): Json<CommandRequest>,
) -> Result<axum::response::Response, StatusCode>
where
    L: SerialPortLister,
    C: ConnectionManager,
{
    let CommandRequest {
        payload,
        wait_for_response,
        timeout_ms,
    } = request;
    let queued = state
        .connection_manager
        .send_command(&name, payload)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    if !wait_for_response {
        return Ok(Json(CommandResponse {
            status: "queued",
            req_id: queued.req_id,
        })
        .into_response());
    }

    let timeout = Duration::from_millis(timeout_ms.unwrap_or(2_000));
    match await_command_response(&state.connection_manager, &name, &queued.req_id, timeout)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
    {
        Some(response) => Ok(Json(WaitedCommandResponse {
            status: "ok",
            req_id: queued.req_id,
            response,
        })
        .into_response()),
        None => Ok((
            StatusCode::GATEWAY_TIMEOUT,
            Json(json!({"error":"command timed out"})),
        )
            .into_response()),
    }
}

async fn await_command_response<C>(
    manager: &C,
    connection_name: &str,
    req_id: &str,
    timeout: Duration,
) -> crate::error::Result<Option<Value>>
where
    C: ConnectionManager,
{
    if timeout.is_zero() {
        return manager.take_response(connection_name, req_id);
    }

    let deadline = tokio::time::Instant::now() + timeout;
    loop {
        if let Some(response) = manager.take_response(connection_name, req_id)? {
            return Ok(Some(response));
        }

        let now = tokio::time::Instant::now();
        if now >= deadline {
            return Ok(None);
        }

        tokio::time::sleep(std::cmp::min(
            Duration::from_millis(10),
            deadline.saturating_duration_since(now),
        ))
        .await;
    }
}

async fn disconnect<L, C>(
    State(state): State<AppState<L, C>>,
    Path(name): Path<String>,
) -> Result<Json<DisconnectResponse>, StatusCode>
where
    L: SerialPortLister,
    C: ConnectionManager,
{
    let name = state
        .connection_manager
        .disconnect(&name)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(DisconnectResponse {
        status: "disconnected",
        name,
    }))
}

async fn disconnect_alias<L, C>(
    State(state): State<AppState<L, C>>,
    Json(request): Json<DisconnectRequest>,
) -> Result<Json<DisconnectResponse>, StatusCode>
where
    L: SerialPortLister,
    C: ConnectionManager,
{
    let name = state
        .connection_manager
        .disconnect(&request.name)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(DisconnectResponse {
        status: "disconnected",
        name,
    }))
}

async fn commit_alias<L, C>(
    State(state): State<AppState<L, C>>,
    Json(payload): Json<Value>,
) -> Result<Json<CommandResponse>, StatusCode>
where
    L: SerialPortLister,
    C: ConnectionManager,
{
    let queued = state
        .connection_manager
        .send_command("default", payload)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(CommandResponse {
        status: "queued",
        req_id: queued.req_id,
    }))
}

#[cfg(test)]
mod tests {
    use axum::body::{to_bytes, Body};
    use axum::http::{Request, StatusCode};
    use futures_util::StreamExt;
    use pretty_assertions::assert_eq;
    use serde_json::json;
    use tower::ServiceExt;

    use super::*;
    use crate::error::Result;
    use crate::serial::manager::{
        ConnectionManagerWithTransport, InMemoryConnectionManager, PortInfo, SerialPortLister,
    };
    use crate::serial::real_transport::{
        RealSerialConnectionManager, RealSerialTransport, SerialPortFactory, SerialPortHandle,
    };

    #[derive(Clone)]
    struct MockPortLister {
        ports: Vec<PortInfo>,
    }

    impl SerialPortLister for MockPortLister {
        fn available_ports(&self) -> Result<Vec<PortInfo>> {
            Ok(self.ports.clone())
        }
    }

    fn app_with_dashboard_assets(asset_root: PathBuf) -> Router {
        router_with_state(
            AppState::new(
                MockPortLister { ports: Vec::new() },
                InMemoryConnectionManager::default(),
            )
            .with_dashboard_assets(asset_root),
        )
    }

    fn dashboard_fixture() -> PathBuf {
        let root = std::env::temp_dir().join(format!(
            "serialport-api-dashboard-test-{}",
            next_socket_io_sid()
        ));
        let assets = root.join("assets");
        std::fs::create_dir_all(&assets).unwrap();
        std::fs::write(
            root.join("index.html"),
            r#"<!doctype html><html><head><script type="module" crossorigin src="/assets/index-test.js"></script><link rel="stylesheet" crossorigin href="/assets/index-test.css"></head><body><div id="root"></div></body></html>"#,
        )
        .unwrap();
        std::fs::write(assets.join("index-test.js"), "console.log('dashboard');").unwrap();
        std::fs::write(assets.join("index-test.css"), "body{margin:0}").unwrap();
        root
    }

    fn source_dashboard_fixture() -> PathBuf {
        let root = std::env::temp_dir().join(format!(
            "serialport-api-dashboard-source-{}",
            next_socket_io_sid()
        ));
        std::fs::create_dir_all(&root).unwrap();
        std::fs::write(
            root.join("index.html"),
            r#"<!doctype html><html><head></head><body><div id="root"></div><script type="module" src="/src/main.tsx"></script></body></html>"#,
        )
        .unwrap();
        root
    }

    #[test]
    fn packaged_dashboard_dir_rejects_vite_source_index_without_built_assets() {
        let root = source_dashboard_fixture();

        assert!(!is_packaged_dashboard_dir(&root));

        let _ = std::fs::remove_dir_all(root);
    }

    #[tokio::test]
    async fn dashboard_route_serves_html_shell_from_configured_assets() {
        let root = dashboard_fixture();
        let response = app_with_dashboard_assets(root.clone())
            .oneshot(
                Request::builder()
                    .uri("/dashboard")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(
            response
                .headers()
                .get("content-type")
                .and_then(|value| value.to_str().ok()),
            Some("text/html; charset=utf-8")
        );
        let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let html = std::str::from_utf8(&body).unwrap();
        assert!(html.contains("/assets/index-test.js"));
        let _ = std::fs::remove_dir_all(root);
    }

    #[tokio::test]
    async fn dashboard_root_route_serves_same_html_shell() {
        let root = dashboard_fixture();
        let response = app_with_dashboard_assets(root.clone())
            .oneshot(Request::builder().uri("/").body(Body::empty()).unwrap())
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let html = std::str::from_utf8(&body).unwrap();
        assert!(html.contains("/assets/index-test.css"));
        let _ = std::fs::remove_dir_all(root);
    }

    #[tokio::test]
    async fn dashboard_asset_route_serves_vite_assets() {
        let root = dashboard_fixture();
        let response = app_with_dashboard_assets(root.clone())
            .oneshot(
                Request::builder()
                    .uri("/assets/index-test.js")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(
            response
                .headers()
                .get("content-type")
                .and_then(|value| value.to_str().ok()),
            Some("text/javascript; charset=utf-8")
        );
        let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        assert_eq!(&body[..], b"console.log('dashboard');");
        let _ = std::fs::remove_dir_all(root);
    }

    #[tokio::test]
    async fn dashboard_missing_assets_return_helpful_response_without_crashing() {
        let root = std::env::temp_dir().join(format!(
            "serialport-api-dashboard-missing-{}",
            next_socket_io_sid()
        ));
        let response = app_with_dashboard_assets(root)
            .oneshot(
                Request::builder()
                    .uri("/dashboard")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
        let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let html = std::str::from_utf8(&body).unwrap();
        assert!(html.contains("Dashboard assets are not available"));
    }

    #[derive(Clone, Default)]
    struct FakeSerialPortFactory {
        state: std::sync::Arc<std::sync::Mutex<FakeFactoryState>>,
    }

    #[derive(Default)]
    struct FakeFactoryState {
        handles: std::collections::BTreeMap<String, FakeSerialPortHandle>,
    }

    #[derive(Clone, Default)]
    struct FakeSerialPortHandle {
        written: std::sync::Arc<std::sync::Mutex<Vec<u8>>>,
        flush_count: std::sync::Arc<std::sync::Mutex<usize>>,
        readable: std::sync::Arc<std::sync::Mutex<std::collections::VecDeque<u8>>>,
    }

    impl FakeSerialPortFactory {
        fn push_bytes(&self, name: &str, bytes: &[u8]) {
            self.state
                .lock()
                .expect("fake factory lock poisoned")
                .handles
                .get(name)
                .expect("expected fake handle")
                .readable
                .lock()
                .expect("fake readable lock poisoned")
                .extend(bytes.iter().copied());
        }

        fn written_for(&self, name: &str) -> Vec<u8> {
            self.state
                .lock()
                .expect("fake factory lock poisoned")
                .handles
                .get(name)
                .expect("expected fake handle")
                .written
                .lock()
                .expect("fake written lock poisoned")
                .clone()
        }

        fn flush_count_for(&self, name: &str) -> usize {
            *self
                .state
                .lock()
                .expect("fake factory lock poisoned")
                .handles
                .get(name)
                .expect("expected fake handle")
                .flush_count
                .lock()
                .expect("fake flush lock poisoned")
        }
    }

    impl SerialPortFactory for FakeSerialPortFactory {
        type Handle = FakeSerialPortHandle;

        fn open(&self, connection: &ConnectionInfo) -> Result<Self::Handle> {
            let handle = FakeSerialPortHandle::default();
            self.state
                .lock()
                .expect("fake factory lock poisoned")
                .handles
                .insert(connection.name.clone(), handle.clone());
            Ok(handle)
        }
    }

    impl SerialPortHandle for FakeSerialPortHandle {
        fn write_all(&mut self, bytes: &[u8]) -> std::io::Result<()> {
            self.written
                .lock()
                .expect("fake written lock poisoned")
                .extend_from_slice(bytes);
            Ok(())
        }

        fn flush(&mut self) -> std::io::Result<()> {
            *self.flush_count.lock().expect("fake flush lock poisoned") += 1;
            Ok(())
        }

        fn read_byte(&mut self) -> std::io::Result<Option<u8>> {
            Ok(self
                .readable
                .lock()
                .expect("fake readable lock poisoned")
                .pop_front())
        }
    }

    #[tokio::test]
    async fn real_mode_routes_write_flush_and_wait_for_fake_serial_response() {
        let factory = FakeSerialPortFactory::default();
        let manager = RealSerialConnectionManager::new(RealSerialTransport::new(factory.clone()));
        let app = router_with_state(AppState::new(
            MockPortLister { ports: Vec::new() },
            manager.clone(),
        ));

        let create_response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/connections")
                    .header("content-type", "application/json")
                    .body(Body::from(
                        r#"{"name":"default","port":"/dev/FAKE","baudRate":115200,"delimiter":"\r\n"}"#,
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(create_response.status(), StatusCode::OK);

        let request = tokio::spawn(app.clone().oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/connections/default/commands")
                .header("content-type", "application/json")
                .body(Body::from(
                    r#"{"payload":{"reqId":"real-route-1","method":"query","topic":"ping","data":{}},"waitForResponse":true,"timeoutMs":1000}"#,
                ))
                .unwrap(),
        ));
        tokio::time::timeout(std::time::Duration::from_secs(1), async {
            loop {
                if factory.flush_count_for("default") == 1 {
                    break;
                }
                tokio::time::sleep(std::time::Duration::from_millis(10)).await;
            }
        })
        .await
        .unwrap();

        assert!(!factory.written_for("default").is_empty());
        factory.push_bytes("default", b"{\"reqId\":\"real-route-1\",\"ok\":true}\r\n");

        let response = request.await.unwrap().unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let payload: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(
            payload,
            json!({
                "status": "ok",
                "reqId": "real-route-1",
                "response": {"reqId":"real-route-1","ok":true}
            })
        );

        let disconnect_response = app
            .oneshot(
                Request::builder()
                    .method("DELETE")
                    .uri("/api/v1/connections/default")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(disconnect_response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn preset_routes_crud_validate_and_report_missing_ids() {
        let app = router();

        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/api/v1/presets")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let payload: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(payload, json!({"presets": []}));

        let create_response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/presets")
                    .header("content-type", "application/json")
                    .body(Body::from(
                        r#"{"name":"Read IMU","payload":{"method":"query","topic":"imu.read","data":{}}}"#,
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(create_response.status(), StatusCode::CREATED);
        let body = to_bytes(create_response.into_body(), usize::MAX)
            .await
            .unwrap();
        let payload: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(
            payload,
            json!({"preset":{"id":1,"name":"Read IMU","payload":{"method":"query","topic":"imu.read","data":{}}}})
        );

        let get_response = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/api/v1/presets/1")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(get_response.status(), StatusCode::OK);

        let update_response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("PUT")
                    .uri("/api/v1/presets/1")
                    .header("content-type", "application/json")
                    .body(Body::from(
                        r#"{"name":"Read temperature","payload":{"method":"query","topic":"temperature.read","data":{}}}"#,
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(update_response.status(), StatusCode::OK);
        let body = to_bytes(update_response.into_body(), usize::MAX)
            .await
            .unwrap();
        let payload: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(
            payload,
            json!({"preset":{"id":1,"name":"Read temperature","payload":{"method":"query","topic":"temperature.read","data":{}}}})
        );

        let invalid_response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/presets")
                    .header("content-type", "application/json")
                    .body(Body::from(r#"{"name":"Bad","payload":[]}"#))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(invalid_response.status(), StatusCode::BAD_REQUEST);

        let delete_response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("DELETE")
                    .uri("/api/v1/presets/1")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(delete_response.status(), StatusCode::OK);
        let body = to_bytes(delete_response.into_body(), usize::MAX)
            .await
            .unwrap();
        let payload: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(payload, json!({"status":"deleted","id":1}));

        let missing_response = app
            .oneshot(
                Request::builder()
                    .uri("/api/v1/presets/1")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(missing_response.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn health_route_returns_status_and_version() {
        let response = router()
            .oneshot(
                Request::builder()
                    .uri("/api/v1/health")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let payload: serde_json::Value = serde_json::from_slice(&body).unwrap();

        assert_eq!(payload, json!({"status":"ok","version":"0.1.0"}));
    }

    #[tokio::test]
    async fn ports_route_returns_available_serial_ports() {
        let response = router_with_port_lister(MockPortLister {
            ports: vec![PortInfo {
                name: "/dev/ttyUSB0".to_string(),
                port_type: "usb".to_string(),
                manufacturer: Some("FTDI".to_string()),
                serial_number: Some("ABC123".to_string()),
            }],
        })
        .oneshot(
            Request::builder()
                .uri("/api/v1/ports")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let payload: serde_json::Value = serde_json::from_slice(&body).unwrap();

        assert_eq!(
            payload,
            json!({
                "ports": [
                    {
                        "name": "/dev/ttyUSB0",
                        "type": "usb",
                        "manufacturer": "FTDI",
                        "serial_number": "ABC123"
                    }
                ]
            })
        );
    }

    #[tokio::test]
    async fn connection_lifecycle_routes_manage_mock_connections() {
        let app = router_with_state(AppState {
            port_lister: MockPortLister { ports: Vec::new() },
            connection_manager: InMemoryConnectionManager::default(),
            preset_store: Arc::new(InMemoryPresetStore::default()),
            dashboard_assets: default_dashboard_assets_dir(),
        });

        let create_response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/connections")
                    .header("content-type", "application/json")
                    .body(Body::from(
                        r#"{"name":"default","port":"/dev/ttyUSB0","baudRate":115200,"delimiter":"\r\n"}"#,
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(create_response.status(), StatusCode::OK);
        let create_body = to_bytes(create_response.into_body(), usize::MAX)
            .await
            .unwrap();
        let create_payload: serde_json::Value = serde_json::from_slice(&create_body).unwrap();
        assert_eq!(
            create_payload,
            json!({
                "status": "connected",
                "connection": {
                    "name": "default",
                    "status": "connected",
                    "port": "/dev/ttyUSB0",
                    "baudRate": 115200,
                    "delimiter": "\r\n"
                }
            })
        );

        let list_response = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/api/v1/connections")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(list_response.status(), StatusCode::OK);
        let list_body = to_bytes(list_response.into_body(), usize::MAX)
            .await
            .unwrap();
        let list_payload: serde_json::Value = serde_json::from_slice(&list_body).unwrap();
        assert_eq!(
            list_payload,
            json!({
                "connections": [
                    {
                        "name": "default",
                        "status": "connected",
                        "port": "/dev/ttyUSB0",
                        "baudRate": 115200,
                        "delimiter": "\r\n"
                    }
                ]
            })
        );

        let delete_response = app
            .oneshot(
                Request::builder()
                    .method("DELETE")
                    .uri("/api/v1/connections/default")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(delete_response.status(), StatusCode::OK);
        let delete_body = to_bytes(delete_response.into_body(), usize::MAX)
            .await
            .unwrap();
        let delete_payload: serde_json::Value = serde_json::from_slice(&delete_body).unwrap();
        assert_eq!(
            delete_payload,
            json!({"status":"disconnected","name":"default"})
        );
    }

    #[tokio::test]
    async fn command_route_queues_payload_for_named_connection() {
        let app = router_with_state(AppState {
            port_lister: MockPortLister { ports: Vec::new() },
            connection_manager: InMemoryConnectionManager::default(),
            preset_store: Arc::new(InMemoryPresetStore::default()),
            dashboard_assets: default_dashboard_assets_dir(),
        });

        let create_response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/connections")
                    .header("content-type", "application/json")
                    .body(Body::from(
                        r#"{"name":"default","port":"/dev/ttyUSB0","baudRate":115200,"delimiter":"\r\n"}"#,
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(create_response.status(), StatusCode::OK);

        let command_response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/connections/default/commands")
                    .header("content-type", "application/json")
                    .body(Body::from(
                        r#"{"payload":{"method":"query","topic":"sensor.read","data":{}},"waitForResponse":false,"timeoutMs":2000}"#,
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(command_response.status(), StatusCode::OK);
        let command_body = to_bytes(command_response.into_body(), usize::MAX)
            .await
            .unwrap();
        let command_payload: serde_json::Value = serde_json::from_slice(&command_body).unwrap();
        assert_eq!(command_payload, json!({"status":"queued","reqId":"1"}));
    }

    #[tokio::test]
    async fn commit_alias_queues_payload_for_default_connection() {
        let app = router_with_state(AppState {
            port_lister: MockPortLister { ports: Vec::new() },
            connection_manager: InMemoryConnectionManager::default(),
            preset_store: Arc::new(InMemoryPresetStore::default()),
            dashboard_assets: default_dashboard_assets_dir(),
        });

        let create_response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/connect")
                    .header("content-type", "application/json")
                    .body(Body::from(
                        r#"{"name":"default","port":"/dev/ttyUSB0","baudRate":115200,"delimiter":"\r\n"}"#,
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(create_response.status(), StatusCode::OK);

        let commit_response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/commit")
                    .header("content-type", "application/json")
                    .body(Body::from(
                        r#"{"reqId":"client-42","method":"mutation","topic":"led.set","data":{"on":true}}"#,
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(commit_response.status(), StatusCode::OK);
        let commit_body = to_bytes(commit_response.into_body(), usize::MAX)
            .await
            .unwrap();
        let commit_payload: serde_json::Value = serde_json::from_slice(&commit_body).unwrap();
        assert_eq!(
            commit_payload,
            json!({"status":"queued","reqId":"client-42"})
        );
    }

    #[tokio::test]
    async fn events_route_streams_recorded_serial_events_as_sse() {
        let manager = InMemoryConnectionManager::default();
        manager.record_event(crate::protocol::SerialEvent::Json(json!({
            "reqId": "1",
            "ok": true
        })));
        manager.record_event(crate::protocol::SerialEvent::Text(
            "hello robot".to_string(),
        ));

        let response = router_with_state(AppState {
            port_lister: MockPortLister { ports: Vec::new() },
            connection_manager: manager,
            preset_store: Arc::new(InMemoryPresetStore::default()),
            dashboard_assets: default_dashboard_assets_dir(),
        })
        .oneshot(
            Request::builder()
                .uri("/api/v1/events")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(
            response
                .headers()
                .get("content-type")
                .and_then(|value| value.to_str().ok()),
            Some("text/event-stream")
        );

        let mut body = response.into_body().into_data_stream();
        let mut text = String::new();
        while !text.contains("event: serial.text") {
            let chunk = tokio::time::timeout(std::time::Duration::from_secs(1), body.next())
                .await
                .unwrap()
                .unwrap()
                .unwrap();
            text.push_str(std::str::from_utf8(&chunk).unwrap());
        }

        assert!(text.contains("event: serial.json"));
        assert!(text.contains("data: {\"ok\":true,\"reqId\":\"1\"}"));
        assert!(text.contains("event: serial.text"));
        assert!(text.contains("data: \"hello robot\""));
    }

    #[tokio::test]
    async fn events_route_streams_read_loop_recorded_events_as_sse() {
        let manager = InMemoryConnectionManager::default();
        let source = crate::serial::read_loop::MockSerialReadSource::default();

        source.push_line("default", b"{\"reqId\":\"1\",\"ok\":true}\r\n".to_vec());
        source.push_line("default", b"hello robot\n".to_vec());
        crate::serial::read_loop::drain_serial_read_items(&manager, &source, "default").unwrap();

        let response = router_with_state(AppState {
            port_lister: MockPortLister { ports: Vec::new() },
            connection_manager: manager,
            preset_store: Arc::new(InMemoryPresetStore::default()),
            dashboard_assets: default_dashboard_assets_dir(),
        })
        .oneshot(
            Request::builder()
                .uri("/api/v1/events")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(
            response
                .headers()
                .get("content-type")
                .and_then(|value| value.to_str().ok()),
            Some("text/event-stream")
        );

        let mut body = response.into_body().into_data_stream();
        let mut text = String::new();
        while !text.contains("event: serial.text") {
            let chunk = tokio::time::timeout(std::time::Duration::from_secs(1), body.next())
                .await
                .unwrap()
                .unwrap()
                .unwrap();
            text.push_str(std::str::from_utf8(&chunk).unwrap());
        }

        assert!(text.contains("event: serial.json"));
        assert!(text.contains("data: {\"ok\":true,\"reqId\":\"1\"}"));
        assert!(text.contains("event: serial.text"));
        assert!(text.contains("data: \"hello robot\""));
    }

    #[tokio::test]
    async fn events_route_replays_snapshot_and_streams_live_serial_events() {
        let manager = InMemoryConnectionManager::default();
        manager.record_event(crate::protocol::SerialEvent::Text("snapshot".to_string()));
        let response = router_with_state(AppState {
            port_lister: MockPortLister { ports: Vec::new() },
            connection_manager: manager.clone(),
            preset_store: Arc::new(InMemoryPresetStore::default()),
            dashboard_assets: default_dashboard_assets_dir(),
        })
        .oneshot(
            Request::builder()
                .uri("/api/v1/events")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(
            response
                .headers()
                .get("content-type")
                .and_then(|value| value.to_str().ok()),
            Some("text/event-stream")
        );

        let mut body = response.into_body().into_data_stream();
        let snapshot = tokio::time::timeout(std::time::Duration::from_secs(1), body.next())
            .await
            .unwrap()
            .unwrap()
            .unwrap();
        let snapshot = std::str::from_utf8(&snapshot).unwrap();
        assert!(snapshot.contains("event: serial.text"));
        assert!(snapshot.contains("data: \"snapshot\""));

        manager.record_event(crate::protocol::SerialEvent::Json(json!({
            "reqId": "live-1",
            "ok": true
        })));

        let live = tokio::time::timeout(std::time::Duration::from_secs(1), body.next())
            .await
            .unwrap()
            .unwrap()
            .unwrap();
        let live = std::str::from_utf8(&live).unwrap();
        assert!(live.contains("event: serial.json"));
        assert!(live.contains("data: {\"ok\":true,\"reqId\":\"live-1\"}"));
    }

    #[tokio::test]
    async fn events_ws_streams_recorded_serial_events_as_json_text_frames() {
        let manager = InMemoryConnectionManager::default();
        manager.record_event(crate::protocol::SerialEvent::Json(json!({
            "reqId": "1",
            "ok": true
        })));
        manager.record_event(crate::protocol::SerialEvent::Text(
            "hello robot".to_string(),
        ));

        let app = router_with_state(AppState {
            port_lister: MockPortLister { ports: Vec::new() },
            connection_manager: manager.clone(),
            preset_store: Arc::new(InMemoryPresetStore::default()),
            dashboard_assets: default_dashboard_assets_dir(),
        });
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let server = tokio::spawn(async move {
            axum::serve(listener, app).await.unwrap();
        });

        let (mut socket, _) =
            tokio_tungstenite::connect_async(format!("ws://{addr}/api/v1/events/ws"))
                .await
                .unwrap();

        let first = tokio::time::timeout(std::time::Duration::from_secs(1), socket.next())
            .await
            .unwrap()
            .unwrap()
            .unwrap();
        let second = tokio::time::timeout(std::time::Duration::from_secs(1), socket.next())
            .await
            .unwrap()
            .unwrap()
            .unwrap();

        assert_eq!(
            serde_json::from_str::<serde_json::Value>(first.to_text().unwrap()).unwrap(),
            json!({"event":"serial.json","data":{"reqId":"1","ok":true}})
        );
        assert_eq!(
            serde_json::from_str::<serde_json::Value>(second.to_text().unwrap()).unwrap(),
            json!({"event":"serial.text","data":"hello robot"})
        );

        manager.record_event(crate::protocol::SerialEvent::Text("live robot".to_string()));
        let live = tokio::time::timeout(std::time::Duration::from_secs(1), socket.next())
            .await
            .unwrap()
            .unwrap()
            .unwrap();
        assert_eq!(
            serde_json::from_str::<serde_json::Value>(live.to_text().unwrap()).unwrap(),
            json!({"event":"serial.text","data":"live robot"})
        );

        server.abort();
    }

    #[tokio::test]
    async fn socket_io_streams_recorded_serial_events_as_engine_io_socket_io_frames() {
        let manager = InMemoryConnectionManager::default();
        manager.record_event(crate::protocol::SerialEvent::Json(json!({
            "reqId": "1",
            "ok": true
        })));
        manager.record_event(crate::protocol::SerialEvent::Text(
            "hello robot".to_string(),
        ));

        let app = router_with_state(AppState {
            port_lister: MockPortLister { ports: Vec::new() },
            connection_manager: manager.clone(),
            preset_store: Arc::new(InMemoryPresetStore::default()),
            dashboard_assets: default_dashboard_assets_dir(),
        });
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let server = tokio::spawn(async move {
            axum::serve(listener, app).await.unwrap();
        });

        let (mut socket, _) = tokio_tungstenite::connect_async(format!(
            "ws://{addr}/socket.io/?EIO=4&transport=websocket"
        ))
        .await
        .unwrap();

        let open = tokio::time::timeout(std::time::Duration::from_secs(1), socket.next())
            .await
            .unwrap()
            .unwrap()
            .unwrap();
        let connect = tokio::time::timeout(std::time::Duration::from_secs(1), socket.next())
            .await
            .unwrap()
            .unwrap()
            .unwrap();
        let first_event = tokio::time::timeout(std::time::Duration::from_secs(1), socket.next())
            .await
            .unwrap()
            .unwrap()
            .unwrap();
        let second_event = tokio::time::timeout(std::time::Duration::from_secs(1), socket.next())
            .await
            .unwrap()
            .unwrap()
            .unwrap();

        let open_text = open.to_text().unwrap();
        assert!(open_text.starts_with('0'));
        let open_payload: serde_json::Value = serde_json::from_str(&open_text[1..]).unwrap();
        assert_eq!(open_payload["upgrades"], json!([]));
        assert_eq!(open_payload["pingInterval"], json!(25000));
        assert_eq!(open_payload["pingTimeout"], json!(20000));
        assert_eq!(open_payload["maxPayload"], json!(1000000));
        assert!(open_payload["sid"]
            .as_str()
            .is_some_and(|sid| !sid.is_empty()));

        assert_eq!(connect.to_text().unwrap(), "40");
        assert_socket_io_event_frame(
            first_event.to_text().unwrap(),
            json!(["serial.json", {"reqId":"1","ok":true}]),
        );
        assert_socket_io_event_frame(
            second_event.to_text().unwrap(),
            json!(["serial.text", "hello robot"]),
        );

        manager.record_event(crate::protocol::SerialEvent::Text("live robot".to_string()));
        let live_event = tokio::time::timeout(std::time::Duration::from_secs(1), socket.next())
            .await
            .unwrap()
            .unwrap()
            .unwrap();
        assert_socket_io_event_frame(
            live_event.to_text().unwrap(),
            json!(["serial.text", "live robot"]),
        );

        server.abort();
    }

    #[tokio::test]
    async fn socket_io_rejects_unsupported_engine_io_query_params() {
        let app = router_with_state(AppState {
            port_lister: MockPortLister { ports: Vec::new() },
            connection_manager: InMemoryConnectionManager::default(),
            preset_store: Arc::new(InMemoryPresetStore::default()),
            dashboard_assets: default_dashboard_assets_dir(),
        });

        for uri in [
            "/socket.io/?EIO=3&transport=websocket",
            "/socket.io/?EIO=4&transport=polling",
            "/socket.io/?transport=websocket",
            "/socket.io/?EIO=4",
        ] {
            let response = app
                .clone()
                .oneshot(Request::builder().uri(uri).body(Body::empty()).unwrap())
                .await
                .unwrap();
            assert_eq!(response.status(), StatusCode::BAD_REQUEST);
        }
    }

    fn assert_socket_io_event_frame(frame: &str, expected_payload: serde_json::Value) {
        assert!(frame.starts_with("42"));
        let payload: serde_json::Value = serde_json::from_str(&frame[2..]).unwrap();
        assert_eq!(payload, expected_payload);
    }

    #[tokio::test(start_paused = true)]
    async fn command_route_waits_for_matching_response() {
        let manager = InMemoryConnectionManager::default();
        let app = router_with_state(AppState {
            port_lister: MockPortLister { ports: Vec::new() },
            connection_manager: manager.clone(),
            preset_store: Arc::new(InMemoryPresetStore::default()),
            dashboard_assets: default_dashboard_assets_dir(),
        });

        let create_response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/connections")
                    .header("content-type", "application/json")
                    .body(Body::from(
                        r#"{"name":"default","port":"/dev/ROBOT","baudRate":115200,"delimiter":"\r\n"}"#,
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(create_response.status(), StatusCode::OK);

        let request = app.clone().oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/connections/default/commands")
                .header("content-type", "application/json")
                .body(Body::from(
                    r#"{"payload":{"reqId":"client-99","method":"query","topic":"sensor.read","data":{}},"waitForResponse":true,"timeoutMs":1000}"#,
                ))
                .unwrap(),
        );

        tokio::pin!(request);
        tokio::task::yield_now().await;

        manager.record_event_for_connection(
            "default",
            crate::protocol::SerialEvent::Json(json!({
                "reqId": "client-99",
                "ok": true,
                "data": {"temperature": 28.5}
            })),
        );
        tokio::time::advance(std::time::Duration::from_millis(10)).await;

        let response = request.await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let payload: serde_json::Value = serde_json::from_slice(&body).unwrap();

        assert_eq!(
            payload,
            json!({
                "status": "ok",
                "reqId": "client-99",
                "response": {
                    "reqId": "client-99",
                    "ok": true,
                    "data": {"temperature": 28.5}
                }
            })
        );
    }

    #[tokio::test]
    async fn command_route_waits_for_mock_device_response() {
        let manager = ConnectionManagerWithTransport::with_mock_responder(
            crate::serial::transport::MockSerialTransport::default(),
            crate::serial::mock_device::MockDeviceResponder::default(),
        );
        let app = router_with_state(AppState::new(MockPortLister { ports: Vec::new() }, manager));

        let create_response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/connections")
                    .header("content-type", "application/json")
                    .body(Body::from(
                        r#"{"name":"default","port":"/dev/ROBOT","baudRate":115200,"delimiter":"\r\n"}"#,
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(create_response.status(), StatusCode::OK);

        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/connections/default/commands")
                    .header("content-type", "application/json")
                    .body(Body::from(
                        r#"{"payload":{"reqId":"mock-route-1","method":"query","topic":"sensor.read","data":{}},"waitForResponse":true,"timeoutMs":1000}"#,
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let payload: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(
            payload,
            json!({
                "status": "ok",
                "reqId": "mock-route-1",
                "response": {
                    "reqId": "mock-route-1",
                    "ok": true,
                    "data": {"mock": true, "topic": "sensor.read"}
                }
            })
        );
    }

    #[tokio::test(start_paused = true)]
    async fn command_route_times_out_waiting_for_response() {
        let app = router_with_state(AppState {
            port_lister: MockPortLister { ports: Vec::new() },
            connection_manager: InMemoryConnectionManager::default(),
            preset_store: Arc::new(InMemoryPresetStore::default()),
            dashboard_assets: default_dashboard_assets_dir(),
        });

        let create_response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/connections")
                    .header("content-type", "application/json")
                    .body(Body::from(
                        r#"{"name":"default","port":"/dev/ROBOT","baudRate":115200,"delimiter":"\r\n"}"#,
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(create_response.status(), StatusCode::OK);

        let request = app.oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/connections/default/commands")
                .header("content-type", "application/json")
                .body(Body::from(
                    r#"{"payload":{"reqId":"will-timeout","method":"query","topic":"sensor.read","data":{}},"waitForResponse":true,"timeoutMs":50}"#,
                ))
                .unwrap(),
        );

        tokio::pin!(request);
        tokio::time::advance(std::time::Duration::from_millis(60)).await;

        let response = request.await.unwrap();
        assert_eq!(response.status(), StatusCode::GATEWAY_TIMEOUT);
    }

    #[tokio::test(start_paused = true)]
    async fn command_route_waits_for_read_loop_recorded_response() {
        let manager = InMemoryConnectionManager::default();
        let read_source = crate::serial::read_loop::MockSerialReadSource::default();
        let app = router_with_state(AppState {
            port_lister: MockPortLister { ports: Vec::new() },
            connection_manager: manager.clone(),
            preset_store: Arc::new(InMemoryPresetStore::default()),
            dashboard_assets: default_dashboard_assets_dir(),
        });

        let create_response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/connections")
                    .header("content-type", "application/json")
                    .body(Body::from(
                        r#"{"name":"default","port":"/dev/ROBOT","baudRate":115200,"delimiter":"\r\n"}"#,
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(create_response.status(), StatusCode::OK);

        let request = app.oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/connections/default/commands")
                .header("content-type", "application/json")
                .body(Body::from(
                    r#"{"payload":{"reqId":"loop-1","method":"query","topic":"sensor.read","data":{}},"waitForResponse":true,"timeoutMs":1000}"#,
                ))
                .unwrap(),
        );

        tokio::pin!(request);
        tokio::task::yield_now().await;

        read_source.push_line(
            "default",
            b"{\"reqId\":\"loop-1\",\"ok\":true}\r\n".to_vec(),
        );
        crate::serial::read_loop::drain_serial_read_items(&manager, &read_source, "default")
            .unwrap();
        tokio::time::advance(std::time::Duration::from_millis(10)).await;

        let response = request.await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let payload: serde_json::Value = serde_json::from_slice(&body).unwrap();

        assert_eq!(
            payload,
            json!({
                "status": "ok",
                "reqId": "loop-1",
                "response": {"reqId":"loop-1","ok":true}
            })
        );
    }

    #[tokio::test]
    async fn legacy_alias_routes_share_connection_state() {
        let app = router_with_state(AppState {
            port_lister: MockPortLister {
                ports: vec![PortInfo {
                    name: "/dev/ttyUSB0".to_string(),
                    port_type: "usb".to_string(),
                    manufacturer: Some("FTDI".to_string()),
                    serial_number: Some("ABC123".to_string()),
                }],
            },
            connection_manager: InMemoryConnectionManager::default(),
            preset_store: Arc::new(InMemoryPresetStore::default()),
            dashboard_assets: default_dashboard_assets_dir(),
        });

        let list_response = app
            .clone()
            .oneshot(Request::builder().uri("/list").body(Body::empty()).unwrap())
            .await
            .unwrap();

        assert_eq!(list_response.status(), StatusCode::OK);
        let list_body = to_bytes(list_response.into_body(), usize::MAX)
            .await
            .unwrap();
        let list_payload: serde_json::Value = serde_json::from_slice(&list_body).unwrap();
        assert_eq!(
            list_payload,
            json!({
                "ports": [
                    {
                        "name": "/dev/ttyUSB0",
                        "type": "usb",
                        "manufacturer": "FTDI",
                        "serial_number": "ABC123"
                    }
                ]
            })
        );

        let connect_response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/connect")
                    .header("content-type", "application/json")
                    .body(Body::from(
                        r#"{"name":"default","port":"/dev/ttyUSB0","baudRate":115200,"delimiter":"\r\n"}"#,
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(connect_response.status(), StatusCode::OK);
        let connect_body = to_bytes(connect_response.into_body(), usize::MAX)
            .await
            .unwrap();
        let connect_payload: serde_json::Value = serde_json::from_slice(&connect_body).unwrap();
        assert_eq!(
            connect_payload,
            json!({
                "status": "connected",
                "connection": {
                    "name": "default",
                    "status": "connected",
                    "port": "/dev/ttyUSB0",
                    "baudRate": 115200,
                    "delimiter": "\r\n"
                }
            })
        );

        let info_response = app
            .clone()
            .oneshot(Request::builder().uri("/info").body(Body::empty()).unwrap())
            .await
            .unwrap();

        assert_eq!(info_response.status(), StatusCode::OK);
        let info_body = to_bytes(info_response.into_body(), usize::MAX)
            .await
            .unwrap();
        let info_payload: serde_json::Value = serde_json::from_slice(&info_body).unwrap();
        assert_eq!(
            info_payload,
            json!({
                "connections": [
                    {
                        "name": "default",
                        "status": "connected",
                        "port": "/dev/ttyUSB0",
                        "baudRate": 115200,
                        "delimiter": "\r\n"
                    }
                ]
            })
        );

        let disconnect_response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/disconnect")
                    .header("content-type", "application/json")
                    .body(Body::from(r#"{"name":"default"}"#))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(disconnect_response.status(), StatusCode::OK);
        let disconnect_body = to_bytes(disconnect_response.into_body(), usize::MAX)
            .await
            .unwrap();
        let disconnect_payload: serde_json::Value =
            serde_json::from_slice(&disconnect_body).unwrap();
        assert_eq!(
            disconnect_payload,
            json!({"status":"disconnected","name":"default"})
        );

        let final_info_response = app
            .oneshot(Request::builder().uri("/info").body(Body::empty()).unwrap())
            .await
            .unwrap();

        assert_eq!(final_info_response.status(), StatusCode::OK);
        let final_info_body = to_bytes(final_info_response.into_body(), usize::MAX)
            .await
            .unwrap();
        let final_info_payload: serde_json::Value =
            serde_json::from_slice(&final_info_body).unwrap();
        assert_eq!(final_info_payload, json!({"connections": []}));
    }
}

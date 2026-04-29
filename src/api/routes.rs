use axum::{
    extract::{Path, State},
    http::StatusCode,
    routing::{delete, get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};

use crate::serial::manager::{
    list_ports, ConnectionInfo, ConnectionManager, ConnectionRequest, InMemoryConnectionManager,
    PortInfo, SerialPortLister, SystemPortLister,
};

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

#[derive(Clone)]
pub struct AppState<L, C> {
    port_lister: L,
    connection_manager: C,
}

pub fn router() -> Router {
    router_with_state(AppState {
        port_lister: SystemPortLister,
        connection_manager: InMemoryConnectionManager::default(),
    })
}

pub fn router_with_port_lister<L>(port_lister: L) -> Router
where
    L: SerialPortLister,
{
    router_with_state(AppState {
        port_lister,
        connection_manager: InMemoryConnectionManager::default(),
    })
}

pub fn router_with_state<L, C>(state: AppState<L, C>) -> Router
where
    L: SerialPortLister,
    C: ConnectionManager,
{
    Router::new()
        .route("/api/v1/health", get(health))
        .route("/api/v1/ports", get(ports::<L, C>))
        .route("/list", get(ports::<L, C>))
        .route(
            "/api/v1/connections",
            post(connect::<L, C>).get(connections::<L, C>),
        )
        .route("/api/v1/connections/:name", delete(disconnect::<L, C>))
        .route("/connect", post(connect::<L, C>))
        .route("/info", get(connections::<L, C>))
        .route("/disconnect", post(disconnect_alias::<L, C>))
        .with_state(state)
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

#[cfg(test)]
mod tests {
    use axum::body::{to_bytes, Body};
    use axum::http::{Request, StatusCode};
    use pretty_assertions::assert_eq;
    use serde_json::json;
    use tower::ServiceExt;

    use super::*;
    use crate::error::Result;
    use crate::serial::manager::{InMemoryConnectionManager, PortInfo, SerialPortLister};

    #[derive(Clone)]
    struct MockPortLister {
        ports: Vec<PortInfo>,
    }

    impl SerialPortLister for MockPortLister {
        fn available_ports(&self) -> Result<Vec<PortInfo>> {
            Ok(self.ports.clone())
        }
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

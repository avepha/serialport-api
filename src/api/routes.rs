use axum::{extract::State, http::StatusCode, routing::get, Json, Router};
use serde::Serialize;

use crate::serial::manager::{list_ports, PortInfo, SerialPortLister, SystemPortLister};

#[derive(Debug, Serialize)]
struct HealthResponse {
    status: &'static str,
    version: &'static str,
}

#[derive(Debug, Serialize)]
struct PortsResponse {
    ports: Vec<PortInfo>,
}

pub fn router() -> Router {
    router_with_port_lister(SystemPortLister)
}

pub fn router_with_port_lister<L>(port_lister: L) -> Router
where
    L: SerialPortLister,
{
    Router::new()
        .route("/api/v1/health", get(health))
        .route("/api/v1/ports", get(ports::<L>))
        .with_state(port_lister)
}

async fn health() -> Json<HealthResponse> {
    Json(HealthResponse {
        status: "ok",
        version: env!("CARGO_PKG_VERSION"),
    })
}

async fn ports<L>(State(port_lister): State<L>) -> Result<Json<PortsResponse>, StatusCode>
where
    L: SerialPortLister,
{
    let ports = list_ports(&port_lister).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(PortsResponse { ports }))
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
    use crate::serial::manager::{PortInfo, SerialPortLister};

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
}

//! Contract tests for the `WebUiBackend` `TransportBackend` impl + router.
//!
//! These exercise the trait lifecycle (start → schema → health → shutdown)
//! and the SPA fallback router without needing a live daemon.

use std::path::PathBuf;

use animus_plugin_protocol::HealthStatus;
use animus_transport_protocol::{TransportBackend, TransportConfig};
use animus_web_ui::backend::DEFAULT_PORT;
use animus_web_ui::config::WebUiSettings;
use animus_web_ui::embed;
use animus_web_ui::server::build_router;
use animus_web_ui::WebUiBackend;
use axum::body::Body;
use axum::http::{HeaderValue, Request, StatusCode};
use http_body_util::BodyExt;
use tower::ServiceExt;

fn test_settings() -> WebUiSettings {
    WebUiSettings {
        bind_addr: "127.0.0.1:0".to_string(),
        control_socket_path: PathBuf::from("/tmp/animus-web-ui-test.sock"),
        project_root: PathBuf::from("/tmp"),
        api_origin: None,
        allowed_hosts: Vec::new(),
    }
}

async fn body_string(app: axum::Router, method: &str, path: &str) -> (StatusCode, String) {
    let req = Request::builder()
        .method(method)
        .uri(path)
        .header("host", "127.0.0.1:8082")
        .body(Body::empty())
        .unwrap();
    let response = app.oneshot(req).await.unwrap();
    let status = response.status();
    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    (status, String::from_utf8_lossy(&bytes).to_string())
}

#[tokio::test]
async fn healthz_returns_ok_envelope() {
    let app = build_router(test_settings());
    let (status, body) = body_string(app, "GET", "/healthz").await;
    assert_eq!(status, StatusCode::OK);
    assert!(body.contains("\"ok\":true"));
    assert!(body.contains("animus-web-ui"));
}

#[tokio::test]
async fn spa_fallback_returns_200_for_unknown_path() {
    let app = build_router(test_settings());
    let (status, body) = body_string(app, "GET", "/some/spa/route").await;
    assert_eq!(status, StatusCode::OK, "got {status} body={body}");

    if embed::is_populated() {
        assert!(
            body.contains("<html") || body.contains("<!doctype"),
            "expected HTML, got: {}",
            &body[..body.len().min(120)]
        );
    } else {
        assert!(body.contains("build the UI first"));
    }
}

#[tokio::test]
async fn root_path_returns_200() {
    let app = build_router(test_settings());
    let (status, _) = body_string(app, "GET", "/").await;
    assert_eq!(status, StatusCode::OK);
}

#[tokio::test]
async fn non_get_method_rejected() {
    let app = build_router(test_settings());
    let req = Request::builder()
        .method("POST")
        .uri("/some/path")
        .header("host", "127.0.0.1:8082")
        .body(Body::from("{}"))
        .unwrap();
    let response = app.oneshot(req).await.unwrap();
    assert_eq!(response.status(), StatusCode::METHOD_NOT_ALLOWED);
}

#[tokio::test]
async fn graphql_post_reaches_the_proxy() {
    // Point the proxy at a closed port so the test never talks to a real
    // transport. The request must reach the proxy (502 from the failed
    // upstream call), not the static handler's 405.
    let settings = WebUiSettings {
        api_origin: Some("http://127.0.0.1:9".to_string()),
        ..test_settings()
    };
    let app = build_router(settings);
    let req = Request::builder()
        .method("POST")
        .uri("/graphql")
        .header("host", "127.0.0.1:8082")
        .header("content-type", "application/json")
        .body(Body::from(r#"{"query":"{ __typename }"}"#))
        .unwrap();
    let response = app.oneshot(req).await.unwrap();
    assert_ne!(response.status(), StatusCode::METHOD_NOT_ALLOWED);
}

#[tokio::test]
async fn missing_asset_404s_when_built() {
    let app = build_router(test_settings());
    let (status, _) = body_string(app, "GET", "/assets/does-not-exist.js").await;
    if embed::is_populated() {
        assert_eq!(status, StatusCode::NOT_FOUND);
    } else {
        assert_eq!(status, StatusCode::OK);
    }
}

#[tokio::test]
async fn backend_lifecycle_round_trip() {
    let backend = WebUiBackend::default();

    let schema = backend.schema();
    assert_eq!(schema.default_port, Some(DEFAULT_PORT));
    assert!(schema.kinds.iter().any(|k| k == "http"));
    assert!(schema.supports_websocket);

    let health_before = backend.health().await.expect("health");
    assert!(matches!(health_before.status, HealthStatus::Degraded));

    let config = TransportConfig {
        control_socket_path: PathBuf::from("/tmp/animus-web-ui-test.sock"),
        project_root: PathBuf::from("/tmp"),
        bind_addr: Some("127.0.0.1:0".into()),
        config: serde_json::Value::Null,
    };
    let info = backend.start(config).await.expect("start");
    assert!(info.bound_addr.starts_with("127.0.0.1:"));

    let health_after = backend.health().await.expect("health");
    assert!(matches!(health_after.status, HealthStatus::Healthy));

    backend.shutdown().await.expect("shutdown");
    backend.shutdown().await.expect("idempotent shutdown");

    let health_post = backend.health().await.expect("health");
    assert!(matches!(health_post.status, HealthStatus::Degraded));
}

#[tokio::test]
async fn fingerprint_heuristic_recognizes_vite_hashes() {
    assert!(embed::is_fingerprinted("/assets/index-AbCdEf12.js"));
    assert!(embed::is_fingerprinted("assets/react-vendor-1a2b3c4d.js"));
    assert!(!embed::is_fingerprinted("/assets/index.js"));
    assert!(!embed::is_fingerprinted("/index.html"));
    assert!(!embed::is_fingerprinted("/favicon.ico"));
}

async fn status_with(
    app: axum::Router,
    method: &str,
    path: &str,
    headers: &[(&str, &str)],
) -> StatusCode {
    let mut req = Request::builder().method(method).uri(path);
    for (name, value) in headers {
        req = req.header(*name, *value);
    }
    app.oneshot(req.body(Body::empty()).unwrap())
        .await
        .unwrap()
        .status()
}

#[tokio::test]
async fn local_host_and_origin_are_served() {
    let status = status_with(
        build_router(test_settings()),
        "GET",
        "/",
        &[
            ("host", "127.0.0.1:8082"),
            ("origin", "http://127.0.0.1:8082"),
        ],
    )
    .await;
    assert_eq!(status, StatusCode::OK);

    let status = status_with(
        build_router(test_settings()),
        "GET",
        "/",
        &[("host", "localhost:8082")],
    )
    .await;
    assert_eq!(status, StatusCode::OK);
}

#[tokio::test]
async fn request_without_host_is_refused() {
    for (method, path) in [("GET", "/healthz"), ("POST", "/graphql")] {
        let status = status_with(build_router(test_settings()), method, path, &[]).await;
        assert_eq!(status, StatusCode::FORBIDDEN, "{method} {path}");
    }
}

#[tokio::test]
async fn unreadable_host_is_refused() {
    for (method, path) in [("GET", "/healthz"), ("POST", "/graphql")] {
        let req = Request::builder()
            .method(method)
            .uri(path)
            .header("host", HeaderValue::from_bytes(b"127.0.0.1\xff").unwrap())
            .body(Body::empty())
            .unwrap();
        let response = build_router(test_settings()).oneshot(req).await.unwrap();
        assert_eq!(response.status(), StatusCode::FORBIDDEN, "{method} {path}");
    }
}

#[tokio::test]
async fn local_uri_authority_without_host_is_served() {
    // HTTP/2 sends the host as the URI authority, not a Host header.
    let status = status_with(
        build_router(test_settings()),
        "GET",
        "http://127.0.0.1:8082/healthz",
        &[],
    )
    .await;
    assert_eq!(status, StatusCode::OK);
}

#[tokio::test]
async fn rebound_host_is_refused() {
    let status = status_with(
        build_router(test_settings()),
        "GET",
        "/",
        &[("host", "evil.example:8082")],
    )
    .await;
    assert_eq!(status, StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn cross_site_graphql_request_is_refused() {
    let status = status_with(
        build_router(test_settings()),
        "POST",
        "/graphql",
        &[
            ("host", "127.0.0.1:8082"),
            ("origin", "https://evil.example"),
        ],
    )
    .await;
    assert_eq!(status, StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn cross_site_websocket_upgrade_is_refused() {
    let status = status_with(
        build_router(test_settings()),
        "GET",
        "/graphql/ws",
        &[
            ("host", "127.0.0.1:8082"),
            ("origin", "https://evil.example"),
            ("connection", "upgrade"),
            ("upgrade", "websocket"),
            ("sec-websocket-version", "13"),
            ("sec-websocket-key", "dGhlIHNhbXBsZSBub25jZQ=="),
        ],
    )
    .await;
    assert_eq!(status, StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn allowed_hosts_extends_the_loopback_set() {
    let settings = || WebUiSettings {
        allowed_hosts: vec!["devbox.lan".to_string()],
        ..test_settings()
    };
    let status = status_with(
        build_router(settings()),
        "GET",
        "/",
        &[
            ("host", "devbox.lan:8082"),
            ("origin", "http://devbox.lan:8082"),
        ],
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let status = status_with(
        build_router(settings()),
        "GET",
        "/",
        &[("host", "other.lan:8082")],
    )
    .await;
    assert_eq!(status, StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn graphql_proxy_does_not_forward_the_browser_origin() {
    // Upstream stand-in for the GraphQL transport: reports whether the
    // request it received carried an Origin header.
    let upstream = axum::Router::new().route(
        "/graphql",
        axum::routing::post(|headers: axum::http::HeaderMap| async move {
            if headers.contains_key("origin") {
                "origin-forwarded"
            } else {
                "no-origin"
            }
        }),
    );
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let upstream_addr = listener.local_addr().unwrap();
    tokio::spawn(async move { axum::serve(listener, upstream).await.unwrap() });

    let settings = WebUiSettings {
        api_origin: Some(format!("http://{upstream_addr}")),
        allowed_hosts: vec!["devbox.lan".to_string()],
        ..test_settings()
    };
    let req = Request::builder()
        .method("POST")
        .uri("/graphql")
        .header("host", "devbox.lan:8082")
        .header("origin", "http://devbox.lan:8082")
        .header("content-type", "application/json")
        .body(Body::from(r#"{"query":"{ __typename }"}"#))
        .unwrap();
    let response = build_router(settings).oneshot(req).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let body = response.into_body().collect().await.unwrap().to_bytes();
    assert_eq!(&body[..], b"no-origin");
}

//! Axum router serving the embedded React build with SPA history fallback.
//!
//! Routing rules:
//!
//! - `GET /healthz` — plugin liveness probe (does not hit the daemon).
//! - `GET /*path` — exact match against the embedded dist; on miss the
//!   request falls through to `index.html` so React Router can handle it.
//! - Fingerprinted bundle assets get a long, immutable `Cache-Control`;
//!   `index.html` is always served `no-cache` so deploys roll out instantly.
//!
//! When the embedded dist is empty (fresh checkout without `npm run build`),
//! the catch-all route returns a static placeholder explaining how to build
//! the UI instead of a confusing 404.

use axum::body::Body;
use axum::extract::State;
use axum::http::{header, HeaderValue, Method, Request, StatusCode, Uri};
use axum::response::{IntoResponse, Response};
use axum::routing::get;
use axum::Router;

use crate::config::WebUiSettings;
use crate::embed;

const PLACEHOLDER_HTML: &str = include_str!("placeholder.html");

#[derive(Clone)]
pub struct AppState {
    pub settings: WebUiSettings,
}

pub fn build_router(settings: WebUiSettings) -> Router {
    let state = AppState { settings };

    Router::new()
        .route("/healthz", get(healthz))
        .fallback(static_handler)
        .with_state(state)
}

async fn healthz() -> impl IntoResponse {
    (
        StatusCode::OK,
        [(header::CONTENT_TYPE, "application/json")],
        r#"{"ok":true,"plugin":"animus-web-ui-wrapper"}"#,
    )
}

async fn static_handler(State(_state): State<AppState>, req: Request<Body>) -> Response {
    if req.method() != Method::GET && req.method() != Method::HEAD {
        return (StatusCode::METHOD_NOT_ALLOWED, "method not allowed").into_response();
    }

    let uri: &Uri = req.uri();
    let path = uri.path();

    if !embed::is_populated() {
        return placeholder_response();
    }

    if let Some(file) = embed::lookup(path) {
        return serve_file(path, file.contents());
    }

    if looks_like_asset(path) {
        return (StatusCode::NOT_FOUND, "not found").into_response();
    }

    match embed::index_html() {
        Some(bytes) => serve_index(bytes),
        None => placeholder_response(),
    }
}

fn serve_file(path: &str, bytes: &'static [u8]) -> Response {
    if path == "/" || path == "/index.html" {
        return serve_index(bytes);
    }

    let mime = mime_guess::from_path(path).first_or_octet_stream();
    let mut resp = Response::new(Body::from(bytes));
    resp.headers_mut().insert(
        header::CONTENT_TYPE,
        HeaderValue::from_str(mime.as_ref())
            .unwrap_or(HeaderValue::from_static("application/octet-stream")),
    );
    if embed::is_fingerprinted(path) {
        resp.headers_mut().insert(
            header::CACHE_CONTROL,
            HeaderValue::from_static("public, max-age=31536000, immutable"),
        );
    } else {
        resp.headers_mut().insert(
            header::CACHE_CONTROL,
            HeaderValue::from_static("public, max-age=300"),
        );
    }
    resp
}

fn serve_index(bytes: &'static [u8]) -> Response {
    let mut resp = Response::new(Body::from(bytes));
    resp.headers_mut().insert(
        header::CONTENT_TYPE,
        HeaderValue::from_static("text/html; charset=utf-8"),
    );
    resp.headers_mut().insert(
        header::CACHE_CONTROL,
        HeaderValue::from_static("no-cache, no-store, must-revalidate"),
    );
    resp
}

fn placeholder_response() -> Response {
    let mut resp = Response::new(Body::from(PLACEHOLDER_HTML));
    resp.headers_mut().insert(
        header::CONTENT_TYPE,
        HeaderValue::from_static("text/html; charset=utf-8"),
    );
    resp.headers_mut().insert(
        header::CACHE_CONTROL,
        HeaderValue::from_static("no-cache, no-store, must-revalidate"),
    );
    *resp.status_mut() = StatusCode::OK;
    resp
}

fn looks_like_asset(path: &str) -> bool {
    let last = path.rsplit('/').next().unwrap_or("");
    last.contains('.')
}

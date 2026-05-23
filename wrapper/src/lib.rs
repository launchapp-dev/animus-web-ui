//! `animus-web-ui-wrapper` — Animus transport plugin that serves the bundled
//! React web UI over HTTP.
//!
//! The React build is embedded at compile time via [`include_dir!`] (see
//! [`embed`]). At runtime the [`backend::WebUiBackend`] implements
//! [`animus_transport_protocol::TransportBackend`] and stands up an Axum
//! server with SPA history fallback. Operators install the resulting binary
//! through `animus plugin install`.

pub mod backend;
pub mod config;
pub mod embed;
pub mod server;

pub use backend::WebUiBackend;

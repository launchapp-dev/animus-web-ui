//! Compile-time embed of the React build output.
//!
//! `include_dir!` snapshots `../dist` at build time. A fresh checkout ships
//! with an empty `dist/` (only `.gitkeep`), so [`is_populated`] lets the
//! server fall back to a "build the UI first" placeholder instead of 404ing
//! on `/`.

use include_dir::{include_dir, Dir, File};

pub static DIST: Dir<'_> = include_dir!("$CARGO_MANIFEST_DIR/../dist");

pub const INDEX_HTML: &str = "index.html";

/// `true` if the embedded build contains an `index.html` — i.e. someone has
/// run `npm run build` before `cargo build`.
pub fn is_populated() -> bool {
    DIST.get_file(INDEX_HTML).is_some()
}

/// Look up a file by relative path inside the embedded dist.
pub fn lookup(path: &str) -> Option<&'static File<'static>> {
    let trimmed = path.trim_start_matches('/');
    if trimmed.is_empty() {
        return DIST.get_file(INDEX_HTML);
    }
    DIST.get_file(trimmed)
}

/// `index.html` bytes when the build is populated.
pub fn index_html() -> Option<&'static [u8]> {
    DIST.get_file(INDEX_HTML).map(|f| f.contents())
}

/// Heuristic — fingerprinted bundles live under `assets/` with hashes in the
/// filename (Vite default). Used to attach long-lived `Cache-Control` only to
/// those entries.
pub fn is_fingerprinted(path: &str) -> bool {
    let trimmed = path.trim_start_matches('/');
    if !trimmed.starts_with("assets/") {
        return false;
    }
    let name = trimmed.rsplit('/').next().unwrap_or("");
    let stem = name.rsplit_once('.').map(|(s, _)| s).unwrap_or(name);
    stem.rsplit_once('-')
        .map(|(_, hash)| hash.len() >= 8 && hash.chars().all(|c| c.is_ascii_alphanumeric()))
        .unwrap_or(false)
}

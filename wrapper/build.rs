//! `include_dir!` in `src/embed.rs` needs `../dist` to exist. `npm run build`
//! fills it; on a fresh checkout without the web build, create it empty so
//! `cargo build` / `cargo test` still work and the server shows the
//! "build the UI first" placeholder.

fn main() {
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR");
    let dist = std::path::Path::new(&manifest_dir).join("../dist");
    std::fs::create_dir_all(&dist).expect("create ../dist");
    // Re-embed when the web build changes.
    println!("cargo:rerun-if-changed=../dist");
}

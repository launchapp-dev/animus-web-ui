# animus-web-ui-wrapper

Animus `TransportBackend` plugin that bundles the
[`animus-web-ui`](https://github.com/launchapp-dev/animus-web-ui) React build
into a single binary and serves it over HTTP. Pinned to
[`animus-protocol`](https://github.com/launchapp-dev/animus-protocol)
`v0.1.8`.

## Default port

`127.0.0.1:8082` — chosen to avoid colliding with the standard transport
plugin defaults:

| Plugin                       | Default port |
|------------------------------|--------------|
| `animus-transport-http`      | 8080         |
| `animus-transport-graphql`   | 8081         |
| `animus-web-ui-wrapper`      | **8082**     |

Operators can override with `bind_addr` in workflow YAML or the project's
`.animus/config.json`.

## Build

The React app must be built *before* the wrapper, because the wrapper bundles
`../dist` at compile time via `include_dir!`.

```bash
# From the repo root:
npm install
npm run build              # emits ./dist
cargo build --release -p animus-web-ui-wrapper
```

If `cargo build` is run without first running `npm run build`, the binary
still compiles (the empty `dist/.gitkeep` placeholder keeps `include_dir!`
happy), but every request returns a "build the UI first" placeholder page
instead of the app.

## Install as a plugin

```bash
animus plugin install ./target/release/animus-web-ui-wrapper
```

The plugin advertises:

- `plugin_kind = "transport_backend"`
- `kinds = ["http", "static"]`
- `default_port = 8082`
- `supports_streaming = false`, `supports_websocket = false`

## Routes

| Path           | Behavior                                                 |
|----------------|----------------------------------------------------------|
| `GET /healthz` | Liveness probe (does not touch the daemon).              |
| `GET /*`       | Embedded dist lookup; SPA fallback to `index.html`.      |

Fingerprinted bundle files under `assets/` (Vite's default output) are served
with `Cache-Control: public, max-age=31568000, immutable`. `index.html` is
served `no-cache` so deploys roll out instantly.

## Configuration

Optional `config` keys in the `TransportConfig` payload:

| Key          | Type   | Notes                                              |
|--------------|--------|----------------------------------------------------|
| `api_origin` | string | Forwarded to the UI for runtime API endpoint discovery (currently unused — the UI resolves `window.location.origin` by default). |

Example `.animus/config.json` snippet:

```json
{
  "transports": {
    "animus-web-ui-wrapper": {
      "bind_addr": "127.0.0.1:8082"
    }
  }
}
```

## Development

```bash
cargo test -p animus-web-ui-wrapper
cargo clippy -p animus-web-ui-wrapper --all-targets -- -D warnings
cargo fmt --check
```

## License

[Elastic License 2.0](../LICENSE) — same as the React app.

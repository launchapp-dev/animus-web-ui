# animus-web-ui

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
| `animus-web-ui`      | **8082**     |

Operators can override with `bind_addr` in workflow YAML or the project's
`.animus/config.json`.

## Build

The React app must be built *before* the wrapper, because the wrapper bundles
`../dist` at compile time via `include_dir!`.

```bash
# From the repo root:
npm install
npm run build              # emits ./dist
cargo build --release -p animus-web-ui
```

If `cargo build` is run without first running `npm run build`, the binary
still compiles (the build script creates an empty `dist/` for
`include_dir!`), but every request returns a "build the UI first"
placeholder page instead of the app.

## Install as a plugin

```bash
animus plugin install ./target/release/animus-web-ui
```

The plugin advertises:

- `plugin_kind = "transport_backend"`
- `kinds = ["http", "static"]`
- `default_port = 8082`
- `supports_streaming = false`, `supports_websocket = true`

## Routes

| Path           | Behavior                                                 |
|----------------|----------------------------------------------------------|
| `GET /healthz` | Liveness probe (does not touch the daemon).              |
| `/graphql`, `/graphql/sdl` | Proxied to the GraphQL transport.            |
| `GET /graphql/ws` | WebSocket proxied to the GraphQL transport.           |
| `GET /*`       | Embedded dist lookup; SPA fallback to `index.html`.      |

Fingerprinted bundle files under `assets/` (Vite's default output) are served
with `Cache-Control: public, max-age=31568000, immutable`. `index.html` is
served `no-cache` so deploys roll out instantly.

## Configuration

Optional `config` keys in the `TransportConfig` payload:

| Key          | Type   | Notes                                              |
|--------------|--------|----------------------------------------------------|
| `api_origin` | string | GraphQL transport origin proxied by the UI server. Defaults to `http://127.0.0.1:8081`; `animus web serve` passes the address the GraphQL transport actually bound. |
| `allowed_hosts` | string[] | Host names accepted besides the loopback ones (see below). Only needed when binding a non-loopback address on purpose. |

The UI server proxies `/graphql`, `/graphql/sdl`, and `/graphql/ws` to the
GraphQL transport. This keeps browser requests same-origin and avoids `405`
responses from the static asset handler.

## Local-only requests

The proxied GraphQL API has no login, so every route answers only requests
addressed to this machine: `Host` must be `localhost`, a `127.x.x.x` address,
or `[::1]`, and `Origin`, when the browser sends one, must be one of those
too. Anything else gets `403`. This keeps other websites and DNS-rebinding
pages open in the same browser from driving the daemon. The GraphQL
transport applies the same rule on its own port.

Example `.animus/config.json` snippet:

```json
{
  "transports": {
    "animus-web-ui": {
      "bind_addr": "127.0.0.1:8082"
    }
  }
}
```

## Development

```bash
cargo test -p animus-web-ui
cargo clippy -p animus-web-ui --all-targets -- -D warnings
cargo fmt --check
```

## License

[Elastic License 2.0](../LICENSE) — same as the React app.

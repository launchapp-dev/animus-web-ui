# animus-web-ui

React UI for [Animus](https://github.com/launchapp-dev/animus) — the autonomous
agent orchestrator. Consumes the HTTP and GraphQL transport plugins
(`animus-transport-http`, `animus-transport-graphql`) so the orchestrator
binary does not need to bundle a UI.

## Stack

- React 18 + React Router 7
- `@tanstack/react-query` + `graphql-request` + `graphql-ws`
- Tailwind CSS 4 + `next-themes`
- Vite 6 + TypeScript 5.8
- Vitest + Testing Library

## Quick start

```bash
npm install
npm run dev        # http://localhost:5174
npm run test
npm run build      # emits ./dist
```

`npm run dev` proxies `/graphql` and `/graphql/ws` to
`http://localhost:8081` by default. Point it at a different transport plugin
with `ANIMUS_DEV_PROXY_TARGET`:

```bash
ANIMUS_DEV_PROXY_TARGET=http://localhost:9090 npm run dev
```

## Configuring the API endpoint

The GraphQL endpoint is resolved at runtime in
`src/lib/graphql/client.ts`. Highest precedence wins:

1. `VITE_ANIMUS_GRAPHQL_URL` — compile-time env var, explicit GraphQL HTTP URL
2. `VITE_ANIMUS_API_ORIGIN` — compile-time env var, origin only (`/graphql` appended)
3. `window.__ANIMUS_CONFIG__.graphqlUrl` — runtime override injected by the host page
4. `window.__ANIMUS_CONFIG__.apiOrigin` — runtime override, origin only
5. `window.location.origin + "/graphql"` — same-origin default (embedded plugin mode)

`VITE_ANIMUS_GRAPHQL_WS_URL` / `window.__ANIMUS_CONFIG__.graphqlWsUrl` do the
same for the websocket subscription endpoint. When unset, the WS URL is
derived from the HTTP URL (swap `http(s)` for `ws(s)`, append `/ws`).

Example build pointed at split transport plugins:

```bash
VITE_ANIMUS_GRAPHQL_URL=http://localhost:8081/graphql \
VITE_ANIMUS_GRAPHQL_WS_URL=ws://localhost:8081/graphql/ws \
  npm run build
```

Example runtime override (inject before the bundle loads):

```html
<script>
  window.__ANIMUS_CONFIG__ = {
    apiOrigin: "https://orchestrator.example.com"
  };
</script>
<script type="module" src="/assets/index.js"></script>
```

## How it talks to Animus

| Transport | Plugin | Default port |
| --- | --- | --- |
| GraphQL (queries, mutations, subscriptions) | `animus-transport-graphql` | 8081 |
| REST (when used) | `animus-transport-http` | 8080 |

This repo ships the React app only. The current default UI flow uses GraphQL
for everything; the REST transport is here for plugins and integrations that
prefer JSON over HTTP.

## Rust plugin wrapper

The [`wrapper/`](./wrapper) crate (`animus-web-ui`) bundles the
built `dist/` tree via `include_dir!` and exposes it as an Animus
[`TransportBackend`](https://github.com/launchapp-dev/animus-protocol/tree/main/animus-transport-protocol)
plugin pinned to `v0.1.8`. Operators install one binary instead of
running a separate static-asset deployment.

| Plugin                       | Default port |
|------------------------------|--------------|
| `animus-transport-http`      | 8080         |
| `animus-transport-graphql`   | 8081         |
| `animus-web-ui`              | **8082**     |

Install workflow:

```bash
git clone https://github.com/launchapp-dev/animus-web-ui.git
cd animus-web-ui
npm install
npm run build                                       # emits ./dist (required before cargo build)
cargo build --release -p animus-web-ui
animus plugin install ./target/release/animus-web-ui
```

The wrapper compiles even on a fresh checkout with an empty `dist/`; in
that mode it serves a "build the UI first" placeholder so the failure mode
is obvious. See [`wrapper/README.md`](./wrapper/README.md) for the full
contract.

## License

[Elastic License 2.0](./LICENSE).

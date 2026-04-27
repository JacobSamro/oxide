# oxide

A Rust npm registry proxy/cache (Rust-Rite) plus a Nuxt 4 admin UI for workspace/team/member
management. The Rust binary terminates TLS directly via Let's Encrypt — no reverse proxy required.

## Layout

- `crates/oxide-server` — the Rust proxy (binary `oxide`)
- `web/` — the Nuxt 4 admin UI (Bun runtime, `bun:sqlite`)
- `data/oxide.db` — single SQLite file shared by both halves

## Run the proxy

```bash
cargo run -p oxide-server
# custom config:
cargo run -p oxide-server -- --config oxide.yaml
```

Defaults: HTTP `:80`, HTTPS `:443`. With no domain configured (fresh install), only HTTP serves.
Once an admin sets a domain + SSL email in the UI, the proxy obtains a Let's Encrypt cert
(via `rustls-acme`) and starts serving HTTPS automatically.

Endpoints:
- `GET /:package`, `GET /:scope/:package` — metadata (full + abbreviated by `Accept`)
- `GET /:package/-/:file` and scoped variant — tarballs (streamed)
- `POST /-/npm/v1/security/audits` — audit (configurable mode)
- `DELETE /-/oxide/cache/:package` — invalidate metadata
- `POST /-/oxide/reload` — re-read settings from sqlite
- `GET /metrics`, `GET /-/health`, `GET /-/ping`

## Run the admin UI

Requires [Bun](https://bun.com).

```bash
cd web
cp .env.example .env       # edit OXIDE_PROXY_URL etc.
bun install
bunx shadcn-vue@latest add card button input label textarea dialog tabs select table badge checkbox
bun run dev                 # http://localhost:3000
```

First visit redirects to `/setup` to create the initial admin. After login, configure
**Settings → Domain / SSL / S3**.

## Performance highlights

- **Singleflight coalescing** prevents thundering herds on cold-cache concurrent requests.
- **Stale-while-revalidate** keeps installs unblocked while metadata is refreshed in background.
- **Precomputed abbreviated + gzip + brotli** payloads — no parse/transform/stringify on hot paths.
- **Tarballs streamed** end-to-end; disk and (optional) S3 backends written via atomic rename.
- **429 fallback** to stale cache, with `Retry-After` honored.
- **Live config**: domain / SSL / S3 settings are edited in the UI and applied without restarts.

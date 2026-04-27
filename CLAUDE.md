# CLAUDE.md

Guidance for Claude Code working in this repository.

## What this is

`oxide` is a two-part system:

1. **Rust npm registry proxy** (`crates/oxide-server`, binary `oxide`) — caches and serves
   the npm registry surface so concurrent installs don't melt under upstream pressure.
   Spec lives in the requirements doc that originated this project (Rust-Rite).
2. **Nuxt 4 admin UI** (`web/`) — workspace/team/member management plus runtime
   configuration of the proxy (domain, SSL/Let's Encrypt, S3 storage).

The two halves share a single SQLite file (`./data/oxide.db` by default). The UI writes
settings, the Rust binary reads them and reconfigures itself live.

## Layout

```
oxide/
├── Cargo.toml                  # Rust workspace
├── oxide.yaml                  # bootstrap config for the proxy
├── crates/oxide-server/        # the proxy
│   └── src/
│       ├── main.rs             # CLI + bootstrap (TLS or plain HTTP)
│       ├── config.rs           # YAML bootstrap config
│       ├── settings.rs         # live settings read from sqlite
│       ├── state.rs            # AppState
│       ├── upstream.rs         # reqwest client, semaphores, rate-limit handling
│       ├── metadata.rs         # metadata cache (mem + disk + SWR + ETag)
│       ├── tarball.rs          # tarball streaming cache (FS + S3)
│       ├── coalesce.rs         # singleflight (broadcast-based for tarballs)
│       ├── transform.rs        # tarball URL rewrite, abbreviated metadata, gzip/brotli
│       ├── s3backend.rs        # rusty-s3 + reqwest tarball backend
│       ├── tls.rs              # rustls-acme bootstrap, https/http listeners
│       ├── routes/             # axum handlers
│       ├── metrics.rs          # Prometheus
│       └── storage.rs          # atomic FS writes
└── web/                        # Nuxt 4 admin UI
    ├── nuxt.config.ts
    ├── server/
    │   ├── api/                # Nitro API routes (kebab-case)
    │   ├── db/schema.sql       # SQLite schema
    │   └── utils/              # db.ts (bun:sqlite), auth.ts, settings.ts, logger.ts
    └── app/                    # pages, layouts, components (options API)
```

## Tech stack

- **Rust**: tokio, axum 0.7, reqwest, moka, dashmap, rustls + rustls-acme, axum-server,
  rusty-s3, rusqlite (bundled), prometheus.
- **Frontend**: Nuxt 4, options API, Tailwind, shadcn-vue (globally auto-imported via
  `shadcn-nuxt`), notivue, lucide-vue-next.
- **Server runtime for the UI**: Bun. The DB layer uses `bun:sqlite`. `pnpm dev` won't
  work — use `bun run dev` (or `pnpm dev` which already prefixes `bun --bun`).
- **Database**: SQLite (single file, WAL mode). Read by both the Rust proxy and the Nuxt server.

## Running

```bash
# proxy (defaults to :80 + :443 unless SSL is disabled, in which case :80 only)
cargo run -p oxide-server

# admin UI
cd web
bun install
bun run dev          # Nuxt on :3000
```

First-time UI visit redirects to `/setup` to create the initial admin. The admin can then
configure domain / SSL / S3 from `Settings`.

## Configuration model

- **Bootstrap** (rare changes): `oxide.yaml` — listen ports, db path, ACME cache dir, uplink TTLs.
- **Runtime** (live, UI-driven): `Setting` table in SQLite, JSON values keyed by `domain`,
  `ssl`, `s3`. The proxy polls every 5s and exposes `POST /-/oxide/reload` for explicit reload.

When SSL is enabled with a configured `primaryDomain` and `acmeEmail`, the proxy obtains a
Let's Encrypt cert via `rustls-acme` and serves HTTPS on `https_listen`. The HTTP listener
either redirects (default) or serves the same app.

## Conventions (frontend)

- Tables PascalCase singular (`User`, `Workspace`, `Team`, `Member`, `Session`, `Setting`),
  fields camelCase.
- API routes kebab-case (`/api/auth/login`, `/api/workspaces/:id`, `/api/settings/:section`).
- Standard envelope: `{ success: true, message, ...data }` / `{ success: false, message }`.
- Vue: options API only.
- Shadcn components are globally auto-imported via `shadcn-nuxt` — never `import {Card} from ...`.
- Shadcn `SelectItem` `value=""` throws — use `'__none__'` and convert to `null` server-side.
- `this.$http.$post(url, { body })`, `this.$http.$get(url)` returning `{ success, ... }`.
- Notifications: `import { push } from 'notivue'` then `push.success/error/info`.
- Icons: `lucide-vue-next`.
- Logger: `server/utils/logger.ts` (do not console.log directly in handlers).

## Conventions (backend / Rust)

- One module per concern; keep them small. `state::AppState` is the single shared object.
- Settings are not hot-reloaded by mutating the existing struct — `ArcSwap` swaps the whole
  snapshot. Reads use `settings.snapshot()`.
- All cache writes are atomic (write to `<path>.tmp.<uuid>`, fsync, rename).
- Singleflight (`coalesce::Singleflight`) for metadata; broadcast-channel-based coalescing
  for tarballs so all concurrent subscribers receive the live byte stream.
- Metrics labels are stable — adding a label is a breaking change for dashboards.

## Common tasks

- **Run cargo check after touching the proxy**: `cargo check`.
- **Apply schema changes**: edit `web/server/db/schema.sql`. Bun `getDb()` re-applies it
  on every start (all DDL is `IF NOT EXISTS`); for destructive migrations write a one-off
  Bun script.
- **Add a shadcn-vue component**: `cd web && bunx shadcn-vue@latest add <name>`.
- **Forward a UI settings change to the proxy**: the PUT `/api/settings/:section` handler
  already POSTs `/-/oxide/reload`; nothing else to wire.
- **Add a metric**: extend `metrics.rs` (`METRICS` is a `once_cell::Lazy`) and register
  with the same registry.

## Things to avoid

- Adding a reverse proxy (nginx/caddy) — the Rust binary terminates TLS itself.
- Adding mysql, postgres, or any other DB — SQLite is the contract between halves.
- Bundling ACME challenge handling in the Nuxt server — TLS is entirely Rust's job.
- Long-form comments. Identifiers and types do most of the work.

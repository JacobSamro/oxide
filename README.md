# oxide

A faster npm registry, written in Rust.

Point your installs at oxide instead of npmjs.org and you get caching, request
coalescing, stale-while-revalidate, and sensible behavior when npmjs is slow or
rate-limiting you. If you've watched ten CI runs hammer the same `lodash`
metadata at the same time, or had `npm install` hang for two minutes because
of a 429, that's the problem oxide solves.

## What you get

- One binary. Fine on a 2-CPU VM.
- HTTPS via Let's Encrypt, built in. No nginx or Caddy in front of it.
- An admin UI for domains, SSL, S3 storage, and the workspace/team/member stuff.
- Works with npm, pnpm, yarn, and bun out of the same registry URL.

## Run it

```bash
cargo run --release -p oxide-server
```

Then in another terminal:

```bash
cd web && bun install && bun run dev
```

Visit `http://localhost:3000`, create your admin user, go to Settings, fill in
your domain, and flip on HTTPS. Oxide gets a Let's Encrypt cert on its own
once that's saved.

## Point clients at it

```
# .npmrc
registry=https://registry.example.com/
```

npm, pnpm, yarn, and bun all read this file. There's no per-tool config to
keep in sync.

## What happens on a normal install

The first install of a package goes to npmjs, gets cached, and is served back.
After that, every install of that package reads from cache. If ten installs
hit an uncached package at once, only one of them goes to npmjs — the other
nine wait on it. When npmjs returns a 429, oxide serves the stale copy
you already have instead of failing the install.

Tarballs stream straight through. A 50MB package doesn't put 50MB on the
heap.

## Storage

Disk by default, which is enough for most setups. If you want shared cache
across more than one oxide instance, point it at an S3-compatible bucket from
the admin UI. AWS, R2, MinIO, and Backblaze all work. Settings apply live, no
restart needed.

## Audit traffic

`npm audit` is rarely useful during a build and is almost always slow. Oxide
returns an empty audit response by default. You can switch it to proxy
upstream with a short timeout, or drop audit traffic entirely, from the UI.

## Publishing your own packages

Oxide is a real registry, not just a cache. You can `npm publish` to it.

1. In the admin UI, go to **Publish tokens** and create one. Copy it when it
   appears — you only see it once.
2. Add it to your `.npmrc` next to your registry line:

```
registry=https://registry.example.com/
//registry.example.com/:_authToken=<your token>
```

3. From your package directory:

```
npm publish
# or
bun publish
```

The first publish makes you the owner of that package name. After that, only
you (or an admin) can publish new versions of it. `npm install <pkg>` and the
equivalent in pnpm/yarn/bun will fetch your published version directly from
oxide — there's no upstream lookup for names that exist locally.

`npm login` works too, if you'd rather sign in with your username/password and
let npm handle the token. Both flows produce the same kind of token.

## Why not Verdaccio?

Verdaccio is fine for small teams. We hit a wall with it under heavy parallel
CI load on a small machine: big metadata documents (the `npm` package itself
is several MB) were slow to serve again and again, and concurrent installs
would just queue up. Oxide is the answer to that specific shape of problem.

It is not a full Verdaccio replacement. There's no plugin API yet, no
unpublish, and no two-factor auth.

## Help

If something breaks — weird metric numbers, an install failing only under one
package manager, a cert that won't issue — open an issue. Paste
`oxide --version` and a chunk of `/metrics`. The more specific, the easier
to chase down.

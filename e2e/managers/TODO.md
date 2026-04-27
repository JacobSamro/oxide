# PM adapters — outstanding work

The first slice ships with `npm` and `bun` adapters working. Remaining matrix rows:

- [ ] `pnpm@9`, `pnpm@10` — install via `corepack prepare pnpm@<v> --activate`. Configure
      via `.npmrc` (it reads the same file). `pnpm install --frozen-lockfile` for warm runs.
- [ ] `yarn-classic@1` — install via `npm i -g yarn@1`. `.yarnrc` (single-line `registry "..."`).
      `yarn install --frozen-lockfile`.
- [ ] `yarn-modern@2|3|4` — install via corepack. Berry config goes in `.yarnrc.yml`
      (`npmRegistryServer: "<url>/"`). Watch for `enableMirror: false` if you want the
      install to actually hit oxide instead of the global mirror cache.
- [ ] Fixture for large metadata (`/npm` package) and assertions for SWR.
- [ ] Concurrent install harness for the coalescing assertion.

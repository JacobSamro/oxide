// Spawns a fresh oxide binary on an ephemeral port with an empty data dir.
// The matrix needs each PM run to be hermetic: no leftover cache state between runs.

import { spawn, type Subprocess } from 'bun'
import { mkdtempSync, rmSync, writeFileSync, mkdirSync } from 'node:fs'
import { tmpdir } from 'node:os'
import { join } from 'node:path'
import { setTimeout as sleep } from 'node:timers/promises'

export interface OxideHandle {
  url: string                 // e.g. http://127.0.0.1:42117
  dataDir: string
  proc: Subprocess
  kill: () => Promise<void>
  metrics: () => Promise<string>
}

export async function startOxide(opts: { binary?: string; port?: number } = {}): Promise<OxideHandle> {
  const port = opts.port ?? randomPort()
  const dataDir = mkdtempSync(join(tmpdir(), 'oxide-e2e-'))
  mkdirSync(join(dataDir, 'metadata'), { recursive: true })
  mkdirSync(join(dataDir, 'tarballs'), { recursive: true })

  const cfgPath = join(dataDir, 'oxide.yaml')
  writeFileSync(cfgPath, configYaml({ port, dataDir }))

  const bin = opts.binary ?? defaultBinary()
  const proc = spawn([bin, '--config', cfgPath], {
    stdout: 'pipe',
    stderr: 'pipe',
    env: { ...process.env, RUST_LOG: process.env.RUST_LOG ?? 'oxide_server=info,info' },
  })

  // Pipe the proxy's logs through so CI logs include them on failure.
  void pipe(proc.stdout, '[oxide]')
  void pipe(proc.stderr, '[oxide!]')

  const url = `http://127.0.0.1:${port}`
  await waitForPing(url, 30_000)

  return {
    url,
    dataDir,
    proc,
    metrics: async () => (await fetch(`${url}/metrics`)).text(),
    async kill() {
      proc.kill()
      try { await proc.exited } catch {}
      rmSync(dataDir, { recursive: true, force: true })
    },
  }
}

function defaultBinary(): string {
  // Prefer release binary in CI (cargo build --release), fall back to debug.
  const repoRoot = join(import.meta.dir, '..', '..')
  const release = join(repoRoot, 'target', 'release', 'oxide')
  const debug = join(repoRoot, 'target', 'debug', 'oxide')
  return Bun.file(release).size ? release : debug
}

function randomPort(): number {
  // 30000-60000 to avoid common reserved ranges.
  return 30000 + Math.floor(Math.random() * 30000)
}

function configYaml({ port, dataDir }: { port: number; dataDir: string }): string {
  return `server:
  http_listen: 127.0.0.1:${port}
  https_listen: 127.0.0.1:${port + 1}
  public_url: http://127.0.0.1:${port}
  db_path: ${dataDir}/oxide.db
  acme_cache_dir: ${dataDir}/acme

log:
  level: info
  json: false

uplinks:
  npmjs:
    url: https://registry.npmjs.org/
    metadata_ttl: 1h
    stale_while_revalidate: 24h
    timeout: 30s
    max_connections: 50
    max_concurrent_metadata_fetches: 25
    max_concurrent_tarball_fetches: 50

cache:
  metadata:
    enabled: true
    memory_max_bytes: 256mb
    disk_enabled: true
    disk_path: ${dataDir}/metadata
    precompute_abbreviated: true
    precompress: true
  tarballs:
    enabled: true
    backend: filesystem
    path: ${dataDir}/tarballs

audit:
  mode: disabled
`
}

async function waitForPing(url: string, timeoutMs: number) {
  const deadline = Date.now() + timeoutMs
  let lastErr: any = null
  while (Date.now() < deadline) {
    try {
      const r = await fetch(`${url}/-/ping`)
      if (r.ok) return
    } catch (e) { lastErr = e }
    await sleep(150)
  }
  throw new Error(`oxide failed to start within ${timeoutMs}ms: ${lastErr?.message ?? ''}`)
}

async function pipe(stream: ReadableStream<Uint8Array>, prefix: string) {
  const reader = stream.getReader()
  const dec = new TextDecoder()
  let buf = ''
  while (true) {
    const { value, done } = await reader.read()
    if (done) break
    buf += dec.decode(value, { stream: true })
    let idx
    while ((idx = buf.indexOf('\n')) >= 0) {
      console.log(prefix, buf.slice(0, idx))
      buf = buf.slice(idx + 1)
    }
  }
  if (buf) console.log(prefix, buf)
}

// Test cases: each receives a fresh oxide instance, a fresh project dir, and
// a configured PM adapter. Cases push their results onto the shared report.
import { mkdtempSync, mkdirSync, cpSync, rmSync } from 'node:fs'
import { tmpdir } from 'node:os'
import { join } from 'node:path'
import type { OxideHandle } from './oxide'
import type { ManagerAdapter, PmContext } from '../managers/types'
import { snapshot, delta } from './assertions'

export interface CaseResult {
  name: string
  ok: boolean
  details: string
  durationMs: number
}

export async function runAll(oxide: OxideHandle, pm: ManagerAdapter, fixturesDir: string): Promise<CaseResult[]> {
  const results: CaseResult[] = []
  results.push(await coldThenWarm(oxide, pm, join(fixturesDir, 'simple')))
  results.push(await scopedPackage(oxide, pm, join(fixturesDir, 'scoped')))
  return results
}

async function coldThenWarm(oxide: OxideHandle, pm: ManagerAdapter, fixture: string): Promise<CaseResult> {
  const t0 = performance.now()
  const ctx = await sandbox(fixture, oxide.url)
  try {
    await pm.configure(ctx)

    const before = await snapshot(oxide.metrics)
    const cold = await pm.install(ctx)
    if (cold.exitCode !== 0) {
      return fail('cold-then-warm', `cold install failed (exit ${cold.exitCode})`, t0)
    }
    const afterCold = await snapshot(oxide.metrics)
    const coldDelta = delta(before, afterCold)
    if (coldDelta.metaMisses === 0) {
      return fail('cold-then-warm', `expected metadata misses on cold install, got 0`, t0)
    }

    // Wipe the PM's local cache so the warm install has to ask oxide again.
    rmSync(join(ctx.homeDir), { recursive: true, force: true })
    rmSync(join(ctx.projectDir, 'node_modules'), { recursive: true, force: true })
    mkdirSync(ctx.homeDir, { recursive: true })
    await pm.configure(ctx)

    const warm = await (pm.ci ? pm.ci(ctx) : pm.install(ctx, { frozen: true }))
    if (warm.exitCode !== 0) {
      return fail('cold-then-warm', `warm install failed (exit ${warm.exitCode})`, t0)
    }
    const afterWarm = await snapshot(oxide.metrics)
    const warmDelta = delta(afterCold, afterWarm)

    // The warm install MUST hit the metadata cache — that's the whole point of the proxy.
    if (warmDelta.metaHits === 0) {
      return fail('cold-then-warm',
        `expected metadata hits on warm install, got 0 (delta: ${JSON.stringify(warmDelta)})`,
        t0)
    }
    // No new upstream metadata fetches on warm.
    if (warmDelta.upstreamMetadata > 0) {
      return fail('cold-then-warm',
        `warm install issued ${warmDelta.upstreamMetadata} upstream metadata requests (expected 0)`,
        t0)
    }

    return { name: 'cold-then-warm', ok: true, durationMs: performance.now() - t0,
      details: `cold misses=${coldDelta.metaMisses} warm hits=${warmDelta.metaHits}` }
  } finally { cleanup(ctx) }
}

async function scopedPackage(oxide: OxideHandle, pm: ManagerAdapter, fixture: string): Promise<CaseResult> {
  const t0 = performance.now()
  const ctx = await sandbox(fixture, oxide.url)
  try {
    await pm.configure(ctx)
    const r = await pm.install(ctx)
    if (r.exitCode !== 0) return fail('scoped-package', `install failed (exit ${r.exitCode})`, t0)
    return { name: 'scoped-package', ok: true, durationMs: performance.now() - t0, details: 'ok' }
  } finally { cleanup(ctx) }
}

async function sandbox(fixture: string, registry: string): Promise<PmContext> {
  const projectDir = mkdtempSync(join(tmpdir(), 'oxide-project-'))
  const homeDir = mkdtempSync(join(tmpdir(), 'oxide-pm-home-'))
  cpSync(fixture, projectDir, { recursive: true })
  return { projectDir, homeDir, registry }
}

function cleanup(ctx: PmContext) {
  rmSync(ctx.projectDir, { recursive: true, force: true })
  rmSync(ctx.homeDir, { recursive: true, force: true })
}

function fail(name: string, details: string, t0: number): CaseResult {
  return { name, ok: false, details, durationMs: performance.now() - t0 }
}

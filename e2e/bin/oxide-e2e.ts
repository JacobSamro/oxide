#!/usr/bin/env bun
// oxide-e2e --pm <spec> [--registry <url>] [--no-spawn] [--keep]
//
// By default the CLI spawns its own oxide instance on a random port. Pass --registry
// to test an externally-running oxide and --no-spawn to skip starting one.
import { join } from 'node:path'
import { startOxide, type OxideHandle } from '../lib/oxide'
import { runAll } from '../lib/cases'
import { createAdapter } from '../managers'
import { parseSpec } from '../managers/types'

interface Args { pm: string; registry?: string; noSpawn: boolean; keep: boolean; verbose: boolean }

function parseArgs(argv: string[]): Args {
  const a: Args = { pm: '', noSpawn: false, keep: false, verbose: false }
  for (let i = 0; i < argv.length; i++) {
    const arg = argv[i]
    if (arg === '--pm') a.pm = argv[++i] ?? ''
    else if (arg === '--registry') a.registry = argv[++i]
    else if (arg === '--no-spawn') a.noSpawn = true
    else if (arg === '--keep') a.keep = true
    else if (arg === '-v' || arg === '--verbose') a.verbose = true
    else if (arg === '-h' || arg === '--help') { printHelp(); process.exit(0) }
  }
  if (!a.pm) { printHelp(); process.exit(2) }
  return a
}

function printHelp() {
  console.log(`oxide-e2e --pm <spec> [--registry <url>] [--no-spawn] [--keep]

PM spec examples:
  npm@10        npm@latest      pnpm@9
  yarn-classic  yarn-modern@4   bun@latest`)
}

async function main() {
  const args = parseArgs(Bun.argv.slice(2))
  const spec = parseSpec(args.pm)

  let handle: OxideHandle | null = null
  let registry = args.registry
  if (!registry && !args.noSpawn) {
    handle = await startOxide()
    registry = handle.url
  }
  if (!registry) {
    console.error('No registry URL: pass --registry or remove --no-spawn')
    process.exit(2)
  }

  // When the registry is external (--registry passed), build a tiny handle so cases can read /metrics.
  const oxideHandle: OxideHandle = handle ?? {
    url: registry, dataDir: '', proc: null as any,
    metrics: async () => (await fetch(`${registry}/metrics`)).text(),
    kill: async () => {},
  }

  const adapter = createAdapter(spec)
  await adapter.ensureInstalled(args.pm)
  console.log(`==> ${adapter.id}@${adapter.version} against ${oxideHandle.url}`)

  const fixturesDir = join(import.meta.dir, '..', 'fixtures')
  const results = await runAll(oxideHandle, adapter, fixturesDir)

  let allOk = true
  for (const r of results) {
    const tag = r.ok ? 'PASS' : 'FAIL'
    console.log(`[${tag}] ${r.name} (${Math.round(r.durationMs)}ms) ${r.ok ? '' : '— ' + r.details}`)
    if (!r.ok) allOk = false
  }

  if (handle && !args.keep) await handle.kill()
  process.exit(allOk ? 0 : 1)
}

main().catch((e) => { console.error(e); process.exit(1) })

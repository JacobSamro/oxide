import { writeFileSync } from 'node:fs'
import { join } from 'node:path'
import type { ManagerAdapter, PmContext } from './types'
import { run } from './exec'

export function createNpmAdapter(version: string): ManagerAdapter {
  let resolvedVersion = version
  return {
    id: 'npm',
    get version() { return resolvedVersion },
    set version(v) { resolvedVersion = v },

    async ensureInstalled(spec: string) {
      // We piggyback on whatever node provides. CI installs the matrix version globally
      // before invoking the CLI; locally, the user is expected to have it on PATH.
      const r = await run(['npm', '--version'])
      if (r.exitCode !== 0) throw new Error(`npm not on PATH: ${r.stderr}`)
      resolvedVersion = r.stdout.trim()
      const wanted = spec.split('@').pop()
      if (wanted && wanted !== 'latest' && !resolvedVersion.startsWith(wanted + '.')) {
        throw new Error(`npm version mismatch: wanted ${wanted}, got ${resolvedVersion}`)
      }
    },

    async configure(ctx: PmContext) {
      // Sandbox npm completely: no auth, no global proxy state, just the registry.
      writeFileSync(join(ctx.projectDir, '.npmrc'), [
        `registry=${ctx.registry}/`,
        `cache=${ctx.homeDir}/npm-cache`,
        `prefix=${ctx.homeDir}/npm-prefix`,
        `update-notifier=false`,
        `fund=false`,
        `audit=false`,
        ``,
      ].join('\n'))
    },

    async install(ctx, opts = {}) {
      const cmd = opts.frozen ? ['npm', 'ci'] : ['npm', 'install']
      const r = await run(cmd, { cwd: ctx.projectDir, env: { HOME: ctx.homeDir }, timeoutMs: 300_000 })
      if (r.exitCode !== 0) console.error(r.stderr)
      return r
    },

    async ci(ctx) { return this.install(ctx, { frozen: true }) },
  }
}

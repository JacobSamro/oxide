import { writeFileSync } from 'node:fs'
import { join } from 'node:path'
import type { ManagerAdapter, PmContext } from './types'
import { run } from './exec'

export function createBunAdapter(version: string): ManagerAdapter {
  let resolvedVersion = version
  return {
    id: 'bun',
    get version() { return resolvedVersion },
    set version(v) { resolvedVersion = v },

    async ensureInstalled(spec: string) {
      const r = await run(['bun', '--version'])
      if (r.exitCode !== 0) throw new Error(`bun not on PATH: ${r.stderr}`)
      resolvedVersion = r.stdout.trim()
      const wanted = spec.split('@').pop()
      if (wanted && wanted !== 'latest' && !resolvedVersion.startsWith(wanted)) {
        throw new Error(`bun version mismatch: wanted ${wanted}, got ${resolvedVersion}`)
      }
    },

    async configure(ctx: PmContext) {
      // Bun reads bunfig.toml at the project root and per-user.
      // We sandbox both via cwd + a per-test BUN_INSTALL_CACHE_DIR.
      writeFileSync(join(ctx.projectDir, 'bunfig.toml'), [
        `[install]`,
        `registry = "${ctx.registry}/"`,
        `cache = "${ctx.homeDir}/bun-install-cache"`,
        ``,
      ].join('\n'))
    },

    async install(ctx, opts = {}) {
      const cmd = opts.frozen ? ['bun', 'install', '--frozen-lockfile'] : ['bun', 'install']
      const r = await run(cmd, {
        cwd: ctx.projectDir,
        env: {
          HOME: ctx.homeDir,
          BUN_INSTALL_CACHE_DIR: `${ctx.homeDir}/bun-install-cache`,
          // Disable lockfile prompts and analytics in CI.
          NO_COLOR: '1',
        },
        timeoutMs: 300_000,
      })
      if (r.exitCode !== 0) console.error(r.stderr)
      return r
    },

    async ci(ctx) { return this.install(ctx, { frozen: true }) },
  }
}

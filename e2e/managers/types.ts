// One adapter per package manager family. The CLI selects an adapter based on
// the --pm spec (e.g. npm@10, pnpm@9, bun@latest, yarn-classic, yarn-modern@4).

export interface PmContext {
  /** A clean working dir already populated with a package.json */
  projectDir: string
  /** Registry URL, e.g. http://127.0.0.1:42117 */
  registry: string
  /** Where this PM should store its caches/configs (sandboxed away from the user's HOME) */
  homeDir: string
}

export interface ManagerAdapter {
  id: string                    // npm, pnpm, yarn-classic, yarn-modern, bun
  version: string               // resolved version string, set by ensureInstalled
  ensureInstalled(versionSpec: string): Promise<void>
  configure(ctx: PmContext): Promise<void>
  install(ctx: PmContext, opts?: { frozen?: boolean }): Promise<{ exitCode: number; durationMs: number }>
  /** Optional: full clean install (lockfile-driven) — required for warm-cache assertions. */
  ci?(ctx: PmContext): Promise<{ exitCode: number; durationMs: number }>
}

export interface PmSpec {
  family: 'npm' | 'pnpm' | 'yarn-classic' | 'yarn-modern' | 'bun'
  version: string               // 'latest', '10', '1.1', etc.
}

export function parseSpec(input: string): PmSpec {
  if (input === 'yarn-classic') return { family: 'yarn-classic', version: '1' }
  const [family, version = 'latest'] = input.split('@')
  if (family === 'yarn-modern' || family === 'yarn-classic') {
    return { family: family as PmSpec['family'], version }
  }
  if (family !== 'npm' && family !== 'pnpm' && family !== 'bun') {
    throw new Error(`Unknown package manager: ${input}`)
  }
  return { family, version }
}

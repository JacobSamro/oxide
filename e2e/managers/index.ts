// Adapter factory. Yarn (classic + modern) and pnpm are stubbed for now;
// the matrix entry will skip them with --skip until adapters land.
import type { ManagerAdapter, PmSpec } from './types'
import { createNpmAdapter } from './npm'
import { createBunAdapter } from './bun'

export function createAdapter(spec: PmSpec): ManagerAdapter {
  switch (spec.family) {
    case 'npm': return createNpmAdapter(spec.version)
    case 'bun': return createBunAdapter(spec.version)
    case 'pnpm':
    case 'yarn-classic':
    case 'yarn-modern':
      throw new Error(`Adapter for ${spec.family} not implemented yet — see managers/TODO.md`)
  }
}

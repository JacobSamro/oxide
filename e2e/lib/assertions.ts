// Snapshots oxide's /metrics endpoint and lets tests assert on counter deltas.
// Counters are monotonic — comparing snapshots is more robust than absolutes
// because previous tests in the same run may have advanced them.

export interface MetricsSnapshot {
  metaHits: number
  metaMisses: number
  metaSwr: number
  metaStaleHits: number
  tarballHits: number
  tarballMisses: number
  coalescedMetadata: number
  coalescedTarball: number
  upstreamMetadata: number
  upstreamTarball: number
}

export async function snapshot(metricsText: string): Promise<MetricsSnapshot>
export async function snapshot(fetchText: () => Promise<string>): Promise<MetricsSnapshot>
export async function snapshot(input: string | (() => Promise<string>)): Promise<MetricsSnapshot> {
  const text = typeof input === 'string' ? input : await input()
  const samples = parsePrometheus(text)
  const sum = (name: string, label?: string) =>
    samples.filter(s => s.name === name && (!label || Object.values(s.labels).includes(label)))
           .reduce((a, b) => a + b.value, 0)
  return {
    metaHits:        sum('oxide_metadata_cache_total', 'hit'),
    metaMisses:      sum('oxide_metadata_cache_total', 'miss'),
    metaSwr:         sum('oxide_metadata_cache_total', 'swr'),
    metaStaleHits:   sum('oxide_metadata_cache_total', 'stale_hit'),
    tarballHits:     sum('oxide_tarball_cache_total', 'hit'),
    tarballMisses:   sum('oxide_tarball_cache_total', 'miss'),
    coalescedMetadata: sum('oxide_coalesced_total', 'metadata'),
    coalescedTarball:  sum('oxide_coalesced_total', 'tarball'),
    upstreamMetadata: sum('oxide_upstream_requests_total', 'metadata'),
    upstreamTarball:  sum('oxide_upstream_requests_total', 'tarball'),
  }
}

export function delta(before: MetricsSnapshot, after: MetricsSnapshot): MetricsSnapshot {
  const out = {} as MetricsSnapshot
  for (const k of Object.keys(before) as Array<keyof MetricsSnapshot>) {
    out[k] = after[k] - before[k]
  }
  return out
}

function parsePrometheus(text: string) {
  const out: Array<{ name: string; labels: Record<string, string>; value: number }> = []
  for (const line of text.split('\n')) {
    if (!line || line.startsWith('#')) continue
    const m = line.match(/^([a-zA-Z_:][a-zA-Z0-9_:]*)(\{[^}]*\})?\s+([0-9eE+\-.]+)/)
    if (!m) continue
    const labels: Record<string, string> = {}
    if (m[2]) {
      const inner = m[2].slice(1, -1)
      for (const part of inner.split(',')) {
        const eq = part.indexOf('=')
        if (eq < 0) continue
        labels[part.slice(0, eq).trim()] = part.slice(eq + 1).trim().replace(/^"|"$/g, '')
      }
    }
    out.push({ name: m[1]!, labels, value: parseFloat(m[3]!) })
  }
  return out
}

// Surfaces oxide proxy /metrics for the dashboard. Parses Prometheus text format.
import { requireUser } from '~/server/utils/auth'
import { ok } from '~/server/utils/respond'
import { logger } from '~/server/utils/logger'

interface Sample { name: string; labels: Record<string, string>; value: number }

function parsePrometheus(text: string): Sample[] {
  const out: Sample[] = []
  for (const line of text.split('\n')) {
    if (!line || line.startsWith('#')) continue
    // metric_name{label="x"} value
    const m = line.match(/^([a-zA-Z_:][a-zA-Z0-9_:]*)(\{[^}]*\})?\s+([0-9eE+\-.]+)/)
    if (!m) continue
    const labels: Record<string, string> = {}
    if (m[2]) {
      const inner = m[2].slice(1, -1)
      for (const part of inner.split(',')) {
        const eq = part.indexOf('=')
        if (eq < 0) continue
        const k = part.slice(0, eq).trim()
        const v = part.slice(eq + 1).trim().replace(/^"|"$/g, '')
        labels[k] = v
      }
    }
    out.push({ name: m[1], labels, value: parseFloat(m[3]) })
  }
  return out
}

export default defineEventHandler(async (event) => {
  await requireUser(event)
  const cfg = useRuntimeConfig()
  try {
    const text = await $fetch<string>(`${cfg.oxideProxyUrl}/metrics`, { responseType: 'text' as any })
    const samples = parsePrometheus(text)
    const get = (name: string, label?: string) => samples
      .filter((s) => s.name === name && (!label || Object.values(s.labels).includes(label)))
      .reduce((a, b) => a + b.value, 0)

    const summary = {
      metadata: {
        hits: get('oxide_metadata_cache_total', 'hit'),
        misses: get('oxide_metadata_cache_total', 'miss'),
        staleHits: get('oxide_metadata_cache_total', 'stale_hit'),
        swr: get('oxide_metadata_cache_total', 'swr'),
        diskHits: get('oxide_metadata_cache_total', 'disk_hit'),
      },
      tarballs: {
        hits: get('oxide_tarball_cache_total', 'hit'),
        misses: get('oxide_tarball_cache_total', 'miss'),
      },
      coalesced: {
        metadata: get('oxide_coalesced_total', 'metadata'),
        tarball: get('oxide_coalesced_total', 'tarball'),
      },
      rateLimited: get('oxide_upstream_rate_limited_total'),
      audit: {
        disabled: get('oxide_audit_total', 'disabled'),
        empty: get('oxide_audit_total', 'empty'),
      },
      memCacheBytes: get('oxide_mem_cache_size_bytes', 'metadata'),
      activeMetaFetches: get('oxide_active_metadata_fetches'),
      activeTarballStreams: get('oxide_active_tarball_streams'),
    }
    return ok({ summary, raw: text })
  } catch (e: any) {
    logger.warn('proxy stats failed', { error: e?.message })
    return ok({ summary: null, raw: '', unreachable: true }, 'Proxy unreachable')
  }
})

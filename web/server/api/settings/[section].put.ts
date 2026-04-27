// PUT /api/settings/:section — section is one of "domain" | "ssl" | "s3".
// After writing, signals the proxy to reload its config.
import { requireAdmin } from '~/server/utils/auth'
import { readSetting, writeSetting } from '~/server/utils/settings'
import { ok, fail } from '~/server/utils/respond'
import { logger } from '~/server/utils/logger'

const SECTIONS = ['domain', 'ssl', 's3'] as const

export default defineEventHandler(async (event) => {
  await requireAdmin(event)
  const section = getRouterParam(event, 'section') as typeof SECTIONS[number]
  if (!SECTIONS.includes(section)) return fail('Unknown section', 404)
  const body = await readBody(event) || {}

  // Preserve secrets when the UI sends back the masked sentinel.
  if (section === 's3' && body.secretKey === '••••••••') {
    const existing = readSetting('s3')
    body.secretKey = existing.secretKey
  }

  writeSetting(section as any, body)

  // Best-effort hot reload of the proxy.
  try {
    const cfg = useRuntimeConfig()
    await $fetch(`${cfg.oxideProxyUrl}/-/oxide/reload`, { method: 'POST' as any })
  } catch (e: any) {
    logger.warn('proxy reload failed', { error: e?.message })
  }

  return ok({}, `${section} settings saved`)
})

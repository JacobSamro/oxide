// Forwards a cache invalidation to the oxide proxy.
import { requireAdmin } from '~/server/utils/auth'
import { ok, fail } from '~/server/utils/respond'

export default defineEventHandler(async (event) => {
  await requireAdmin(event)
  const { package: pkg } = await readBody(event) || {}
  if (!pkg) return fail('Package name required')
  const cfg = useRuntimeConfig()
  const enc = encodeURIComponent(pkg)
  await $fetch(`${cfg.oxideProxyUrl}/-/oxide/cache/${enc}`, { method: 'DELETE' as any })
  return ok({}, `Invalidated ${pkg}`)
})

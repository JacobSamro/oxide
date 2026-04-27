// Revoke a token by its prefix. The full token isn't shown after creation, so users
// have to identify it by the first 8 chars (matches what GET returns).
import { get, run } from '~/server/utils/db'
import { requireUser } from '~/server/utils/auth'
import { ok, fail } from '~/server/utils/respond'

export default defineEventHandler(async (event) => {
  const user = await requireUser(event)
  const prefix = getRouterParam(event, 'prefix')
  if (!prefix || prefix.length < 8) return fail('Invalid token prefix')
  const row = get<any>(
    'SELECT id FROM Token WHERE userId = ? AND id LIKE ? || \'%\' LIMIT 1',
    [user.id, prefix.slice(0, 8)],
  )
  if (!row) return fail('Token not found', 404)
  run('DELETE FROM Token WHERE id = ?', [row.id])
  return ok({}, 'Token revoked')
})

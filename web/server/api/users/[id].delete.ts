import { run } from '~/server/utils/db'
import { requireAdmin } from '~/server/utils/auth'
import { ok, fail } from '~/server/utils/respond'

export default defineEventHandler(async (event) => {
  const me = await requireAdmin(event)
  const id = Number(getRouterParam(event, 'id'))
  if (!id) return fail('Invalid id')
  if (id === me.id) return fail('You cannot delete yourself', 400)
  run('DELETE FROM User WHERE id = ?', [id])
  return ok({}, 'User deleted')
})

import { get, run } from '~/server/utils/db'
import { requireUser } from '~/server/utils/auth'
import { ok, fail } from '~/server/utils/respond'

export default defineEventHandler(async (event) => {
  const user = await requireUser(event)
  const id = Number(getRouterParam(event, 'id'))
  if (!id) return fail('Invalid id')
  const ws = get<any>('SELECT ownerId FROM Workspace WHERE id = ?', [id])
  if (!ws) return fail('Not found', 404)
  if (!user.isAdmin && ws.ownerId !== user.id) return fail('Forbidden', 403)
  run('DELETE FROM Workspace WHERE id = ?', [id])
  return ok({}, 'Workspace deleted')
})

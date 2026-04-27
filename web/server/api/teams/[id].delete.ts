import { get, run } from '~/server/utils/db'
import { requireUser } from '~/server/utils/auth'
import { assertWorkspaceAccess } from '~/server/utils/access'
import { ok, fail } from '~/server/utils/respond'

export default defineEventHandler(async (event) => {
  const user = await requireUser(event)
  const id = Number(getRouterParam(event, 'id'))
  if (!id) return fail('Invalid id')
  const t = get<any>('SELECT workspaceId FROM Team WHERE id = ?', [id])
  if (!t) return fail('Not found', 404)
  await assertWorkspaceAccess(user, t.workspaceId, { manage: true })
  run('DELETE FROM Team WHERE id = ?', [id])
  return ok({}, 'Team deleted')
})

import { get, run } from '~/server/utils/db'
import { requireUser } from '~/server/utils/auth'
import { assertWorkspaceAccess } from '~/server/utils/access'
import { ok, fail } from '~/server/utils/respond'

export default defineEventHandler(async (event) => {
  const user = await requireUser(event)
  const id = Number(getRouterParam(event, 'id'))
  if (!id) return fail('Invalid id')
  const m = get<any>('SELECT workspaceId FROM Member WHERE id = ?', [id])
  if (!m) return fail('Not found', 404)
  await assertWorkspaceAccess(user, m.workspaceId, { manage: true })
  run('DELETE FROM Member WHERE id = ?', [id])
  return ok({}, 'Member removed')
})

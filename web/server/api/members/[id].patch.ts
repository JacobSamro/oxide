import { get, run } from '~/server/utils/db'
import { requireUser } from '~/server/utils/auth'
import { assertWorkspaceAccess } from '~/server/utils/access'
import { ok, fail } from '~/server/utils/respond'

const ROLES = ['owner', 'admin', 'member', 'viewer']

export default defineEventHandler(async (event) => {
  const user = await requireUser(event)
  const id = Number(getRouterParam(event, 'id'))
  const { role, teamId } = await readBody(event) || {}
  if (!id) return fail('Invalid id')
  const m = get<any>('SELECT workspaceId FROM Member WHERE id = ?', [id])
  if (!m) return fail('Not found', 404)
  await assertWorkspaceAccess(user, m.workspaceId, { manage: true })

  const updates: string[] = []
  const params: any[] = []
  if (role) {
    if (!ROLES.includes(role)) return fail('Invalid role')
    updates.push('role = ?'); params.push(role)
  }
  if (teamId !== undefined) {
    if (teamId !== null) {
      const t = get<any>('SELECT workspaceId FROM Team WHERE id = ?', [teamId])
      if (!t || t.workspaceId !== m.workspaceId) return fail('Team mismatch', 400)
    }
    updates.push('teamId = ?'); params.push(teamId)
  }
  if (!updates.length) return fail('Nothing to update')
  params.push(id)
  run(`UPDATE Member SET ${updates.join(', ')} WHERE id = ?`, params)
  return ok({}, 'Member updated')
})

import { all, get } from '~/server/utils/db'
import { requireUser } from '~/server/utils/auth'
import { ok, fail } from '~/server/utils/respond'

export default defineEventHandler(async (event) => {
  const user = await requireUser(event)
  const id = Number(getRouterParam(event, 'id'))
  if (!id) return fail('Invalid id')

  const ws = get<any>('SELECT * FROM Workspace WHERE id = ?', [id])
  if (!ws) return fail('Not found', 404)

  const isMember = get<any>(
    'SELECT 1 AS x FROM Member WHERE workspaceId = ? AND userId = ? LIMIT 1',
    [id, user.id],
  )
  if (!user.isAdmin && ws.ownerId !== user.id && !isMember) return fail('Forbidden', 403)

  const teams = all<any>(
    `SELECT t.*, (SELECT COUNT(*) FROM Member m WHERE m.teamId = t.id) AS memberCount
       FROM Team t WHERE t.workspaceId = ? ORDER BY t.name ASC`,
    [id],
  )
  const members = all<any>(
    `SELECT m.id, m.role, m.teamId, m.createdAt, u.id AS userId, u.email, u.name
       FROM Member m JOIN User u ON u.id = m.userId
      WHERE m.workspaceId = ? ORDER BY m.role, u.name`,
    [id],
  )
  return ok({ workspace: ws, teams, members })
})

import { all } from '~/server/utils/db'
import { requireUser } from '~/server/utils/auth'
import { ok } from '~/server/utils/respond'

export default defineEventHandler(async (event) => {
  const user = await requireUser(event)
  const workspaces = all<any>(
    `SELECT w.id, w.slug, w.name, w.description, w.ownerId, w.createdAt,
            (SELECT COUNT(*) FROM Team t WHERE t.workspaceId = w.id) AS teamCount,
            (SELECT COUNT(*) FROM Member m WHERE m.workspaceId = w.id) AS memberCount
       FROM Workspace w
      WHERE w.ownerId = ?
         OR EXISTS (SELECT 1 FROM Member m WHERE m.workspaceId = w.id AND m.userId = ?)
         OR ? = 1
      ORDER BY w.createdAt DESC`,
    [user.id, user.id, user.isAdmin ? 1 : 0],
  )
  return ok({ workspaces })
})

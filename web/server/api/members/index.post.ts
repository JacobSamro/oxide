import { get, run } from '~/server/utils/db'
import { requireUser } from '~/server/utils/auth'
import { assertWorkspaceAccess } from '~/server/utils/access'
import { ok, fail } from '~/server/utils/respond'

const ROLES = ['owner', 'admin', 'member', 'viewer'] as const

export default defineEventHandler(async (event) => {
  const user = await requireUser(event)
  const { workspaceId, teamId, email, role } = await readBody(event) || {}
  if (!workspaceId || !email) return fail('workspaceId and email required')
  const finalRole = role && (ROLES as readonly string[]).includes(role) ? role : 'member'
  await assertWorkspaceAccess(user, Number(workspaceId), { manage: true })

  const u = get<any>('SELECT id FROM User WHERE email = ?', [email])
  if (!u) return fail('User with that email not found. Create the user first.', 404)

  if (teamId) {
    const t = get<any>('SELECT workspaceId FROM Team WHERE id = ?', [teamId])
    if (!t || t.workspaceId !== Number(workspaceId)) return fail('Team does not belong to workspace', 400)
  }

  try {
    const res = run(
      'INSERT INTO Member (workspaceId, teamId, userId, role) VALUES (?, ?, ?, ?)',
      [workspaceId, teamId || null, u.id, finalRole],
    )
    return ok({ memberId: res.lastInsertRowid }, 'Member added')
  } catch (e: any) {
    if (String(e?.message || '').includes('UNIQUE')) return fail('User is already a member of this scope', 409)
    throw e
  }
})

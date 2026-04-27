// Helpers for workspace-scoped access checks.
import { get } from './db'
import type { SessionUser } from './auth'
import { fail } from './respond'

export async function assertWorkspaceAccess(user: SessionUser, workspaceId: number, opts: { manage?: boolean } = {}) {
  const ws = get<any>('SELECT ownerId FROM Workspace WHERE id = ?', [workspaceId])
  if (!ws) return fail('Workspace not found', 404)
  if (user.isAdmin || ws.ownerId === user.id) return ws
  const m = get<any>('SELECT role FROM Member WHERE workspaceId = ? AND userId = ? LIMIT 1', [workspaceId, user.id])
  if (!m) return fail('Forbidden', 403)
  if (opts.manage && !['owner', 'admin'].includes(m.role)) return fail('Insufficient role', 403)
  return ws
}

import { run } from '~/server/utils/db'
import { requireUser } from '~/server/utils/auth'
import { ok, fail } from '~/server/utils/respond'

const SLUG_RE = /^[a-z0-9](?:[a-z0-9-]{0,62}[a-z0-9])?$/

export default defineEventHandler(async (event) => {
  const user = await requireUser(event)
  const { slug, name, description } = await readBody(event) || {}
  if (!slug || !name) return fail('Slug and name required')
  if (!SLUG_RE.test(slug)) return fail('Invalid slug (lowercase, numbers, dashes)')
  try {
    const res = run(
      'INSERT INTO Workspace (slug, name, description, ownerId) VALUES (?, ?, ?, ?)',
      [slug, name, description || null, user.id],
    )
    run(
      'INSERT INTO Member (workspaceId, teamId, userId, role) VALUES (?, NULL, ?, "owner")',
      [res.lastInsertRowid, user.id],
    )
    return ok({ workspaceId: res.lastInsertRowid }, 'Workspace created')
  } catch (e: any) {
    if (String(e?.message || '').includes('UNIQUE')) return fail('Slug already taken', 409)
    throw e
  }
})

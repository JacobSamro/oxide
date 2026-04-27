import { run } from '~/server/utils/db'
import { requireUser } from '~/server/utils/auth'
import { assertWorkspaceAccess } from '~/server/utils/access'
import { ok, fail } from '~/server/utils/respond'

const SLUG_RE = /^[a-z0-9](?:[a-z0-9-]{0,62}[a-z0-9])?$/

export default defineEventHandler(async (event) => {
  const user = await requireUser(event)
  const { workspaceId, slug, name, description } = await readBody(event) || {}
  if (!workspaceId || !slug || !name) return fail('workspaceId, slug, name required')
  if (!SLUG_RE.test(slug)) return fail('Invalid slug')
  await assertWorkspaceAccess(user, Number(workspaceId), { manage: true })
  try {
    const res = run(
      'INSERT INTO Team (workspaceId, slug, name, description) VALUES (?, ?, ?, ?)',
      [workspaceId, slug, name, description || null],
    )
    return ok({ teamId: res.lastInsertRowid }, 'Team created')
  } catch (e: any) {
    if (String(e?.message || '').includes('UNIQUE')) return fail('Team slug already used', 409)
    throw e
  }
})

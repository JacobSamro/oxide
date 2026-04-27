// First-run setup: create the initial admin user when no users exist.
import { get, run } from '~/server/utils/db'
import { createSession, hashPassword } from '~/server/utils/auth'
import { ok, fail } from '~/server/utils/respond'

export default defineEventHandler(async (event) => {
  const existing = get<any>('SELECT COUNT(*) AS c FROM User')
  if ((existing?.c ?? 0) > 0) return fail('Setup already complete', 409)

  const { email, name, password } = await readBody(event) || {}
  if (!email || !name || !password) return fail('Email, name, password required')

  const hash = await hashPassword(password)
  const res = run(
    'INSERT INTO User (email, name, passwordHash, isAdmin) VALUES (?, ?, ?, 1)',
    [email, name, hash],
  )
  await createSession(event, res.lastInsertRowid)
  return ok({ user: { id: res.lastInsertRowid, email, name, isAdmin: true } }, 'Setup complete')
})

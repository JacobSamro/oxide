import { get } from '~/server/utils/db'
import { createSession, verifyPassword } from '~/server/utils/auth'
import { ok, fail } from '~/server/utils/respond'

export default defineEventHandler(async (event) => {
  const { email, password } = await readBody(event) || {}
  if (!email || !password) return fail('Email and password required')
  const u = get<any>('SELECT id, email, name, passwordHash, isAdmin FROM User WHERE email = ? LIMIT 1', [email])
  if (!u) return fail('Invalid credentials', 401)
  if (!(await verifyPassword(password, u.passwordHash))) return fail('Invalid credentials', 401)
  await createSession(event, u.id)
  return ok({ user: { id: u.id, email: u.email, name: u.name, isAdmin: !!u.isAdmin } }, 'Logged in')
})

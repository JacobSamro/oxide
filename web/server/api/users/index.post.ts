import { run } from '~/server/utils/db'
import { requireAdmin, hashPassword } from '~/server/utils/auth'
import { ok, fail } from '~/server/utils/respond'

export default defineEventHandler(async (event) => {
  await requireAdmin(event)
  const { email, name, password, isAdmin } = await readBody(event) || {}
  if (!email || !name || !password) return fail('Email, name, password required')
  const hash = await hashPassword(password)
  try {
    const res = run(
      'INSERT INTO User (email, name, passwordHash, isAdmin) VALUES (?, ?, ?, ?)',
      [email, name, hash, isAdmin ? 1 : 0],
    )
    return ok({ user: { id: res.lastInsertRowid, email, name, isAdmin: !!isAdmin } }, 'User created')
  } catch (e: any) {
    if (String(e?.message || '').includes('UNIQUE')) return fail('Email already exists', 409)
    throw e
  }
})

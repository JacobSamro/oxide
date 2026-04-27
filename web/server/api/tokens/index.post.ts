// Mints a new publish token. The full token is returned ONCE in the response;
// after this it can only be used, not re-read, by anyone.
import { run } from '~/server/utils/db'
import { requireUser } from '~/server/utils/auth'
import { ok, fail } from '~/server/utils/respond'
import { randomBytes } from 'node:crypto'

export default defineEventHandler(async (event) => {
  const user = await requireUser(event)
  const { name } = await readBody(event) || {}
  if (name && typeof name !== 'string') return fail('Invalid name')
  const token = randomBytes(32).toString('hex')
  run('INSERT INTO Token (id, userId, name) VALUES (?, ?, ?)', [token, user.id, name || null])
  return ok({ token, name: name || null }, 'Token created. Save it now — you will not see it again.')
})

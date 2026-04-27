// Lists the current user's tokens. Shows id (which IS the secret) only on creation —
// listing returns just the metadata to discourage tokens leaking through screenshots.
import { all } from '~/server/utils/db'
import { requireUser } from '~/server/utils/auth'
import { ok } from '~/server/utils/respond'

export default defineEventHandler(async (event) => {
  const user = await requireUser(event)
  const tokens = all<any>(
    `SELECT substr(id, 1, 8) AS prefix, name, createdAt, lastUsedAt
       FROM Token WHERE userId = ? ORDER BY createdAt DESC`,
    [user.id],
  )
  return ok({ tokens })
})

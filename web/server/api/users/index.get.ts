import { all } from '~/server/utils/db'
import { requireAdmin } from '~/server/utils/auth'
import { ok } from '~/server/utils/respond'

export default defineEventHandler(async (event) => {
  await requireAdmin(event)
  const users = all<any>('SELECT id, email, name, isAdmin, createdAt FROM User ORDER BY id ASC')
  return ok({ users })
})

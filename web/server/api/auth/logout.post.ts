import { destroySession } from '~/server/utils/auth'
import { ok } from '~/server/utils/respond'

export default defineEventHandler(async (event) => {
  await destroySession(event)
  return ok({}, 'Logged out')
})

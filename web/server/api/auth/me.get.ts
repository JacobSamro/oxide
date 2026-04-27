import { currentUser } from '~/server/utils/auth'
import { ok } from '~/server/utils/respond'

export default defineEventHandler(async (event) => {
  const user = await currentUser(event)
  return ok({ user })
})

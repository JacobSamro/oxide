import { requireAdmin } from '~/server/utils/auth'
import { readAllSettings } from '~/server/utils/settings'
import { ok } from '~/server/utils/respond'

export default defineEventHandler(async (event) => {
  await requireAdmin(event)
  const settings = readAllSettings()
  // Mask the secret key on read so it never leaks back to the browser.
  if (settings.s3.secretKey) settings.s3.secretKey = '••••••••'
  return ok({ settings })
})

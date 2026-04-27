// Standard API envelope helpers.
export const ok = <T extends Record<string, any>>(data: T = {} as T, message = 'OK') => ({
  success: true,
  message,
  ...data,
})

export const fail = (message: string, statusCode = 400) => {
  throw createError({ statusCode, statusMessage: message, data: { success: false, message } })
}

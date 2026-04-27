// Global auth gate. Public routes: /login, /setup. Everything else needs a session.
export default defineNuxtRouteMiddleware(async (to) => {
  const publicRoutes = ['/login', '/setup']
  if (publicRoutes.includes(to.path)) return

  try {
    const res: any = await $fetch('/api/auth/me')
    if (res && res.success && res.user) return
  } catch {}
  return navigateTo('/login')
})

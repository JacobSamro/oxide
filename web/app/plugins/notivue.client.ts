import { createNotivue } from 'notivue'

export default defineNuxtPlugin((nuxt) => {
  const notivue = createNotivue({
    position: 'top-right',
    limit: 6,
    notifications: { global: { duration: 4000 } },
  })
  nuxt.vueApp.use(notivue)
})

// Nuxt 4 uses srcDir 'app' by default. Shadcn-nuxt + Tailwind + notivue setup.
export default defineNuxtConfig({
  modules: ['@nuxtjs/tailwindcss', 'shadcn-nuxt'],
  css: ['~/assets/css/tailwind.css', 'notivue/notification.css', 'notivue/animations.css'],
  shadcn: {
    prefix: '',
    componentDir: './app/components/ui',
  },
  runtimeConfig: {
    sqlitePath: process.env.SQLITE_PATH || './data/oxide.db',
    oxideProxyUrl: process.env.OXIDE_PROXY_URL || 'http://localhost:4873',
    sessionSecret: process.env.SESSION_SECRET || 'change-me-please',
    public: {
      proxyPublicUrl: process.env.OXIDE_PROXY_URL || 'http://localhost:4873',
    },
  },
  build: { transpile: ['notivue'] },
  nitro: {
    // bun:sqlite is provided by the Bun runtime — keep Nitro from trying to bundle it.
    rollupConfig: { external: ['bun:sqlite'] },
  },
  compatibilityDate: '2025-04-01',
})

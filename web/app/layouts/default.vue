<template>
  <div class="min-h-screen bg-background flex">
    <aside v-if="user" class="w-60 border-r bg-muted/30 p-4 flex flex-col gap-1">
      <div class="px-2 py-3 mb-2">
        <div class="font-bold text-lg flex items-center gap-2">
          <Boxes class="size-5" />
          oxide
        </div>
        <div class="text-xs text-muted-foreground">Rust-Rite registry proxy</div>
      </div>
      <NuxtLink v-for="link in nav" :key="link.to" :to="link.to" class="px-3 py-2 rounded-md text-sm hover:bg-accent flex items-center gap-2"
        active-class="bg-accent text-accent-foreground font-medium">
        <component :is="link.icon" class="size-4" />
        {{ link.label }}
      </NuxtLink>
      <div class="mt-auto pt-4 border-t">
        <div class="text-xs text-muted-foreground px-2 mb-1">Signed in as</div>
        <div class="px-2 text-sm font-medium">{{ user.name }}</div>
        <div class="px-2 text-xs text-muted-foreground">{{ user.email }}</div>
        <Button variant="ghost" size="sm" class="w-full justify-start mt-2" @click="logout">
          <LogOut class="size-4 mr-2" /> Sign out
        </Button>
      </div>
    </aside>
    <main class="flex-1 p-8 overflow-x-auto">
      <slot />
    </main>
  </div>
</template>

<script>
import { Boxes, Layers, Users, Activity, LogOut, Settings, Key } from 'lucide-vue-next'
import { push } from 'notivue'

export default {
  components: { Boxes, LogOut },
  data() {
    return {
      user: null,
      nav: [
        { to: '/', label: 'Overview', icon: Activity },
        { to: '/workspaces', label: 'Workspaces', icon: Layers },
        { to: '/users', label: 'Users', icon: Users },
        { to: '/tokens', label: 'Publish tokens', icon: Key },
        { to: '/settings', label: 'Settings', icon: Settings },
      ],
    }
  },
  async mounted() {
    await this.loadUser()
  },
  watch: {
    '$route.fullPath'() { this.loadUser() },
  },
  methods: {
    async loadUser() {
      try {
        const res = await this.$http.$get('/api/auth/me')
        if (res && res.success) this.user = res.user
      } catch (e) { this.user = null }
    },
    async logout() {
      try {
        await this.$http.$post('/api/auth/logout', { body: {} })
        push.success('Signed out')
        this.user = null
        this.$router.push('/login')
      } catch (e) { push.error('Logout failed') }
    },
  },
}
</script>

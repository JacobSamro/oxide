<template>
  <NuxtLayout name="auth">
    <div class="min-h-screen flex items-center justify-center bg-muted/40">
      <Card class="w-full max-w-md">
        <CardHeader>
          <CardTitle class="text-2xl">Sign in to oxide</CardTitle>
          <CardDescription>Manage your registry proxy and workspaces.</CardDescription>
        </CardHeader>
        <CardContent>
          <form class="space-y-4" @submit.prevent="submit">
            <div>
              <Label>Email</Label>
              <Input v-model="email" type="email" required autocomplete="email" />
            </div>
            <div>
              <Label>Password</Label>
              <Input v-model="password" type="password" required autocomplete="current-password" />
            </div>
            <Button type="submit" :disabled="loading" class="w-full">
              {{ loading ? 'Signing in…' : 'Sign in' }}
            </Button>
          </form>
        </CardContent>
        <CardFooter class="flex justify-between text-xs text-muted-foreground">
          <NuxtLink to="/setup" class="hover:underline">First-time setup</NuxtLink>
        </CardFooter>
      </Card>
    </div>
  </NuxtLayout>
</template>

<script>
import { push } from 'notivue'
export default {
  data() { return { email: '', password: '', loading: false } },
  methods: {
    async submit() {
      this.loading = true
      try {
        const res = await this.$http.$post('/api/auth/login', { body: { email: this.email, password: this.password } })
        if (res && res.success) {
          push.success('Welcome back')
          this.$router.push('/')
        } else {
          push.error(res?.message || 'Login failed')
        }
      } catch (e) {
        push.error(e?.data?.message || 'Login failed')
      } finally { this.loading = false }
    },
  },
}
</script>

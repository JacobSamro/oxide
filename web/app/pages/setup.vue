<template>
  <NuxtLayout name="auth">
    <div class="min-h-screen flex items-center justify-center bg-muted/40">
      <Card class="w-full max-w-md">
        <CardHeader>
          <CardTitle class="text-2xl">Create the first admin</CardTitle>
          <CardDescription>This is only available before any user has been created.</CardDescription>
        </CardHeader>
        <CardContent>
          <form class="space-y-4" @submit.prevent="submit">
            <div>
              <Label>Name</Label>
              <Input v-model="name" required />
            </div>
            <div>
              <Label>Email</Label>
              <Input v-model="email" type="email" required />
            </div>
            <div>
              <Label>Password</Label>
              <Input v-model="password" type="password" required minlength="8" />
            </div>
            <Button type="submit" :disabled="loading" class="w-full">
              {{ loading ? 'Creating…' : 'Create admin' }}
            </Button>
          </form>
        </CardContent>
      </Card>
    </div>
  </NuxtLayout>
</template>

<script>
import { push } from 'notivue'
export default {
  data() { return { name: '', email: '', password: '', loading: false } },
  methods: {
    async submit() {
      this.loading = true
      try {
        const res = await this.$http.$post('/api/setup', { body: { name: this.name, email: this.email, password: this.password } })
        if (res && res.success) {
          push.success('Admin created')
          this.$router.push('/')
        } else {
          push.error(res?.message || 'Setup failed')
        }
      } catch (e) {
        push.error(e?.data?.message || 'Setup failed')
      } finally { this.loading = false }
    },
  },
}
</script>

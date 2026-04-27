<template>
  <div class="space-y-6">
    <div class="flex items-center justify-between">
      <div>
        <h1 class="text-3xl font-bold">Users</h1>
        <p class="text-muted-foreground text-sm">Admins can create accounts; users can then be assigned to workspaces.</p>
      </div>
      <Dialog v-model:open="dialogOpen">
        <DialogTrigger as-child><Button><Plus class="size-4 mr-1" />New user</Button></DialogTrigger>
        <DialogContent>
          <DialogHeader><DialogTitle>Create user</DialogTitle></DialogHeader>
          <form class="space-y-3" @submit.prevent="create">
            <div><Label>Name</Label><Input v-model="form.name" required /></div>
            <div><Label>Email</Label><Input v-model="form.email" type="email" required /></div>
            <div><Label>Password</Label><Input v-model="form.password" type="password" required minlength="8" /></div>
            <div class="flex items-center gap-2">
              <Checkbox v-model:checked="form.isAdmin" id="isAdmin" />
              <Label for="isAdmin">Admin</Label>
            </div>
            <DialogFooter><Button type="submit">Create</Button></DialogFooter>
          </form>
        </DialogContent>
      </Dialog>
    </div>

    <Table>
      <TableHeader>
        <TableRow>
          <TableHead>Name</TableHead>
          <TableHead>Email</TableHead>
          <TableHead>Role</TableHead>
          <TableHead>Created</TableHead>
          <TableHead></TableHead>
        </TableRow>
      </TableHeader>
      <TableBody>
        <TableRow v-for="u in users" :key="u.id">
          <TableCell class="font-medium">{{ u.name }}</TableCell>
          <TableCell class="text-muted-foreground">{{ u.email }}</TableCell>
          <TableCell><Badge :variant="u.isAdmin ? 'default' : 'secondary'">{{ u.isAdmin ? 'admin' : 'user' }}</Badge></TableCell>
          <TableCell class="text-muted-foreground text-xs">{{ formatDate(u.createdAt) }}</TableCell>
          <TableCell class="text-right">
            <Button variant="ghost" size="sm" @click="remove(u.id)"><Trash2 class="size-4" /></Button>
          </TableCell>
        </TableRow>
      </TableBody>
    </Table>
  </div>
</template>

<script>
import { Plus, Trash2 } from 'lucide-vue-next'
import { push } from 'notivue'
export default {
  components: { Plus, Trash2 },
  data() {
    return {
      users: [],
      dialogOpen: false,
      form: { name: '', email: '', password: '', isAdmin: false },
    }
  },
  mounted() { this.load() },
  methods: {
    formatDate(s) { return s ? new Date(s).toLocaleDateString() : '' },
    async load() {
      try {
        const res = await this.$http.$get('/api/users')
        if (res && res.success) this.users = res.users
      } catch (e) { push.error(e?.data?.message || 'Failed to load users') }
    },
    async create() {
      try {
        const res = await this.$http.$post('/api/users', { body: this.form })
        if (res && res.success) {
          push.success('User created')
          this.dialogOpen = false
          this.form = { name: '', email: '', password: '', isAdmin: false }
          await this.load()
        } else push.error(res?.message)
      } catch (e) { push.error(e?.data?.message || 'Failed') }
    },
    async remove(id) {
      if (!confirm('Delete this user?')) return
      try {
        await this.$http.$delete(`/api/users/${id}`)
        push.success('Deleted')
        await this.load()
      } catch (e) { push.error(e?.data?.message || 'Failed') }
    },
  },
}
</script>

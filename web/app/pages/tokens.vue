<template>
  <div class="space-y-6 max-w-3xl">
    <div class="flex items-center justify-between">
      <div>
        <h1 class="text-3xl font-bold">Publish tokens</h1>
        <p class="text-muted-foreground text-sm">Use these for <code>npm publish</code> instead of your password.</p>
      </div>
      <Dialog v-model:open="dialogOpen">
        <DialogTrigger as-child><Button><Plus class="size-4 mr-1" />New token</Button></DialogTrigger>
        <DialogContent>
          <DialogHeader>
            <DialogTitle>New publish token</DialogTitle>
            <DialogDescription>Pick a label so you remember what this is for.</DialogDescription>
          </DialogHeader>
          <form class="space-y-3" @submit.prevent="create">
            <div>
              <Label>Label</Label>
              <Input v-model="newName" placeholder="laptop, ci, etc." />
            </div>
            <DialogFooter><Button type="submit" :disabled="creating">{{ creating ? 'Creating…' : 'Create' }}</Button></DialogFooter>
          </form>
        </DialogContent>
      </Dialog>
    </div>

    <div v-if="freshToken" class="rounded-md border bg-muted/30 p-4 space-y-2">
      <div class="text-sm font-medium">Copy this now — it will not be shown again.</div>
      <pre class="bg-background border rounded px-3 py-2 text-sm overflow-x-auto">{{ freshToken }}</pre>
      <div class="text-xs text-muted-foreground">
        Add to your <code>.npmrc</code>:
        <pre class="bg-background border rounded px-3 py-2 mt-1">//{{ host }}/:_authToken={{ freshToken }}</pre>
      </div>
      <Button size="sm" variant="ghost" @click="freshToken = ''">Hide</Button>
    </div>

    <Table>
      <TableHeader>
        <TableRow>
          <TableHead>Label</TableHead>
          <TableHead>Prefix</TableHead>
          <TableHead>Created</TableHead>
          <TableHead>Last used</TableHead>
          <TableHead></TableHead>
        </TableRow>
      </TableHeader>
      <TableBody>
        <TableRow v-for="t in tokens" :key="t.prefix">
          <TableCell>{{ t.name || '—' }}</TableCell>
          <TableCell class="font-mono text-xs">{{ t.prefix }}…</TableCell>
          <TableCell class="text-xs text-muted-foreground">{{ formatDate(t.createdAt) }}</TableCell>
          <TableCell class="text-xs text-muted-foreground">{{ t.lastUsedAt ? formatDate(t.lastUsedAt) : 'never' }}</TableCell>
          <TableCell class="text-right">
            <Button variant="ghost" size="sm" @click="revoke(t.prefix)"><Trash2 class="size-4" /></Button>
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
    return { tokens: [], dialogOpen: false, creating: false, newName: '', freshToken: '' }
  },
  computed: {
    host() {
      try { return new URL(this.$config.public.proxyPublicUrl).host } catch { return 'localhost:4873' }
    },
  },
  mounted() { this.load() },
  methods: {
    formatDate(s) { return s ? new Date(s).toLocaleString() : '' },
    async load() {
      try {
        const res = await this.$http.$get('/api/tokens')
        if (res && res.success) this.tokens = res.tokens
      } catch (e) { push.error(e?.data?.message || 'Failed to load tokens') }
    },
    async create() {
      this.creating = true
      try {
        const res = await this.$http.$post('/api/tokens', { body: { name: this.newName || null } })
        if (res && res.success) {
          this.freshToken = res.token
          this.newName = ''
          this.dialogOpen = false
          await this.load()
        } else push.error(res?.message)
      } catch (e) { push.error(e?.data?.message || 'Failed') }
      finally { this.creating = false }
    },
    async revoke(prefix) {
      if (!confirm('Revoke this token? Anyone using it will have publishes start failing.')) return
      try {
        await this.$http.$delete(`/api/tokens/${prefix}`)
        push.success('Revoked')
        await this.load()
      } catch (e) { push.error(e?.data?.message || 'Failed') }
    },
  },
}
</script>

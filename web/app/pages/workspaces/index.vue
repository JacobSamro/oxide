<template>
  <div class="space-y-6">
    <div class="flex items-center justify-between">
      <div>
        <h1 class="text-3xl font-bold">Workspaces</h1>
        <p class="text-muted-foreground text-sm">Group teams and members by project or business unit.</p>
      </div>
      <Dialog v-model:open="dialogOpen">
        <DialogTrigger as-child>
          <Button><Plus class="size-4 mr-1" /> New workspace</Button>
        </DialogTrigger>
        <DialogContent>
          <DialogHeader>
            <DialogTitle>Create workspace</DialogTitle>
            <DialogDescription>Slugs are used in URLs and cannot be changed later.</DialogDescription>
          </DialogHeader>
          <form class="space-y-3" @submit.prevent="create">
            <div>
              <Label>Slug</Label>
              <Input v-model="form.slug" required />
            </div>
            <div>
              <Label>Name</Label>
              <Input v-model="form.name" required />
            </div>
            <div>
              <Label>Description</Label>
              <Textarea v-model="form.description" />
            </div>
            <DialogFooter>
              <Button type="submit" :disabled="creating">{{ creating ? 'Creating…' : 'Create' }}</Button>
            </DialogFooter>
          </form>
        </DialogContent>
      </Dialog>
    </div>

    <div v-if="!workspaces" class="text-sm text-muted-foreground">Loading…</div>
    <div v-else-if="!workspaces.length" class="text-sm text-muted-foreground">No workspaces yet.</div>
    <div v-else class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4">
      <Card v-for="ws in workspaces" :key="ws.id" class="hover:border-primary/50 transition cursor-pointer" @click="$router.push(`/workspaces/${ws.id}`)">
        <CardHeader>
          <CardTitle>{{ ws.name }}</CardTitle>
          <CardDescription>/{{ ws.slug }}</CardDescription>
        </CardHeader>
        <CardContent class="text-sm text-muted-foreground space-y-1">
          <div v-if="ws.description">{{ ws.description }}</div>
          <div class="flex gap-4 pt-2 text-xs">
            <span><Layers class="size-3 inline mr-1" />{{ ws.teamCount }} teams</span>
            <span><Users class="size-3 inline mr-1" />{{ ws.memberCount }} members</span>
          </div>
        </CardContent>
      </Card>
    </div>
  </div>
</template>

<script>
import { Plus, Layers, Users } from 'lucide-vue-next'
import { push } from 'notivue'

export default {
  components: { Plus, Layers, Users },
  data() {
    return {
      workspaces: null,
      dialogOpen: false,
      creating: false,
      form: { slug: '', name: '', description: '' },
    }
  },
  mounted() { this.load() },
  methods: {
    async load() {
      try {
        const res = await this.$http.$get('/api/workspaces')
        if (res && res.success) this.workspaces = res.workspaces
      } catch (e) { push.error('Failed to load workspaces') }
    },
    async create() {
      this.creating = true
      try {
        const res = await this.$http.$post('/api/workspaces', { body: this.form })
        if (res && res.success) {
          push.success('Workspace created')
          this.dialogOpen = false
          this.form = { slug: '', name: '', description: '' }
          await this.load()
        } else { push.error(res?.message) }
      } catch (e) { push.error(e?.data?.message || 'Failed') }
      finally { this.creating = false }
    },
  },
}
</script>

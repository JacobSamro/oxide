<template>
  <div class="space-y-6">
    <div v-if="!workspace" class="text-sm text-muted-foreground">Loading…</div>
    <template v-else>
      <div class="flex items-center justify-between">
        <div>
          <NuxtLink to="/workspaces" class="text-xs text-muted-foreground hover:underline">← Workspaces</NuxtLink>
          <h1 class="text-3xl font-bold">{{ workspace.name }}</h1>
          <p class="text-sm text-muted-foreground">/{{ workspace.slug }} · {{ workspace.description || 'No description' }}</p>
        </div>
        <Button v-if="canDelete" variant="destructive" size="sm" @click="remove">
          <Trash2 class="size-4 mr-1" /> Delete workspace
        </Button>
      </div>

      <Tabs default-value="teams" class="w-full">
        <TabsList>
          <TabsTrigger value="teams"><Layers class="size-4 mr-1" />Teams ({{ teams.length }})</TabsTrigger>
          <TabsTrigger value="members"><Users class="size-4 mr-1" />Members ({{ members.length }})</TabsTrigger>
        </TabsList>

        <TabsContent value="teams" class="space-y-4 pt-4">
          <div class="flex justify-end">
            <Dialog v-model:open="teamDialog">
              <DialogTrigger as-child><Button><Plus class="size-4 mr-1" />New team</Button></DialogTrigger>
              <DialogContent>
                <DialogHeader>
                  <DialogTitle>Create team</DialogTitle>
                </DialogHeader>
                <form class="space-y-3" @submit.prevent="createTeam">
                  <div><Label>Slug</Label><Input v-model="teamForm.slug" required /></div>
                  <div><Label>Name</Label><Input v-model="teamForm.name" required /></div>
                  <div><Label>Description</Label><Textarea v-model="teamForm.description" /></div>
                  <DialogFooter><Button type="submit">Create</Button></DialogFooter>
                </form>
              </DialogContent>
            </Dialog>
          </div>
          <Card v-for="t in teams" :key="t.id">
            <CardHeader class="flex flex-row items-center justify-between space-y-0">
              <div>
                <CardTitle class="text-lg">{{ t.name }}</CardTitle>
                <CardDescription>/{{ t.slug }} · {{ t.memberCount }} members</CardDescription>
              </div>
              <Button variant="ghost" size="sm" @click="deleteTeam(t.id)"><Trash2 class="size-4" /></Button>
            </CardHeader>
          </Card>
          <p v-if="!teams.length" class="text-sm text-muted-foreground">No teams yet.</p>
        </TabsContent>

        <TabsContent value="members" class="space-y-4 pt-4">
          <div class="flex justify-end">
            <Dialog v-model:open="memberDialog">
              <DialogTrigger as-child><Button><Plus class="size-4 mr-1" />Add member</Button></DialogTrigger>
              <DialogContent>
                <DialogHeader><DialogTitle>Add member</DialogTitle></DialogHeader>
                <form class="space-y-3" @submit.prevent="addMember">
                  <div><Label>User email</Label><Input v-model="memberForm.email" type="email" required /></div>
                  <div>
                    <Label>Team</Label>
                    <Select v-model="memberForm.teamId">
                      <SelectTrigger><SelectValue placeholder="Workspace-wide" /></SelectTrigger>
                      <SelectContent>
                        <SelectItem value="__none__">Workspace-wide</SelectItem>
                        <SelectItem v-for="t in teams" :key="t.id" :value="String(t.id)">{{ t.name }}</SelectItem>
                      </SelectContent>
                    </Select>
                  </div>
                  <div>
                    <Label>Role</Label>
                    <Select v-model="memberForm.role">
                      <SelectTrigger><SelectValue /></SelectTrigger>
                      <SelectContent>
                        <SelectItem value="admin">Admin</SelectItem>
                        <SelectItem value="member">Member</SelectItem>
                        <SelectItem value="viewer">Viewer</SelectItem>
                      </SelectContent>
                    </Select>
                  </div>
                  <DialogFooter><Button type="submit">Add</Button></DialogFooter>
                </form>
              </DialogContent>
            </Dialog>
          </div>
          <Table>
            <TableHeader>
              <TableRow>
                <TableHead>User</TableHead>
                <TableHead>Email</TableHead>
                <TableHead>Team</TableHead>
                <TableHead>Role</TableHead>
                <TableHead></TableHead>
              </TableRow>
            </TableHeader>
            <TableBody>
              <TableRow v-for="m in members" :key="m.id">
                <TableCell class="font-medium">{{ m.name }}</TableCell>
                <TableCell class="text-muted-foreground">{{ m.email }}</TableCell>
                <TableCell>{{ teamName(m.teamId) }}</TableCell>
                <TableCell><Badge :variant="m.role === 'owner' ? 'default' : 'secondary'">{{ m.role }}</Badge></TableCell>
                <TableCell class="text-right">
                  <Button v-if="m.role !== 'owner'" variant="ghost" size="sm" @click="removeMember(m.id)">
                    <Trash2 class="size-4" />
                  </Button>
                </TableCell>
              </TableRow>
            </TableBody>
          </Table>
        </TabsContent>
      </Tabs>
    </template>
  </div>
</template>

<script>
import { Plus, Trash2, Layers, Users } from 'lucide-vue-next'
import { push } from 'notivue'

export default {
  components: { Plus, Trash2, Layers, Users },
  data() {
    return {
      workspace: null,
      teams: [],
      members: [],
      me: null,
      teamDialog: false,
      memberDialog: false,
      teamForm: { slug: '', name: '', description: '' },
      memberForm: { email: '', teamId: '__none__', role: 'member' },
    }
  },
  computed: {
    canDelete() {
      if (!this.me || !this.workspace) return false
      return this.me.isAdmin || this.workspace.ownerId === this.me.id
    },
  },
  mounted() { this.load() },
  watch: { '$route.params.id'() { this.load() } },
  methods: {
    teamName(id) {
      if (!id) return '—'
      const t = this.teams.find((x) => x.id === id)
      return t ? t.name : '—'
    },
    async load() {
      const id = this.$route.params.id
      try {
        const me = await this.$http.$get('/api/auth/me')
        this.me = me?.user
        const res = await this.$http.$get(`/api/workspaces/${id}`)
        if (res && res.success) {
          this.workspace = res.workspace
          this.teams = res.teams
          this.members = res.members
        }
      } catch (e) { push.error('Failed to load workspace') }
    },
    async createTeam() {
      try {
        const res = await this.$http.$post('/api/teams', { body: { ...this.teamForm, workspaceId: this.workspace.id } })
        if (res && res.success) {
          push.success('Team created')
          this.teamDialog = false
          this.teamForm = { slug: '', name: '', description: '' }
          await this.load()
        } else push.error(res?.message)
      } catch (e) { push.error(e?.data?.message || 'Failed') }
    },
    async deleteTeam(id) {
      if (!confirm('Delete this team?')) return
      try {
        await this.$http.$delete(`/api/teams/${id}`)
        push.success('Team deleted')
        await this.load()
      } catch (e) { push.error(e?.data?.message || 'Failed') }
    },
    async addMember() {
      try {
        const teamId = this.memberForm.teamId === '__none__' ? null : Number(this.memberForm.teamId)
        const res = await this.$http.$post('/api/members', { body: {
          workspaceId: this.workspace.id,
          email: this.memberForm.email,
          teamId,
          role: this.memberForm.role,
        }})
        if (res && res.success) {
          push.success('Member added')
          this.memberDialog = false
          this.memberForm = { email: '', teamId: '__none__', role: 'member' }
          await this.load()
        } else push.error(res?.message)
      } catch (e) { push.error(e?.data?.message || 'Failed') }
    },
    async removeMember(id) {
      if (!confirm('Remove this member?')) return
      try {
        await this.$http.$delete(`/api/members/${id}`)
        push.success('Member removed')
        await this.load()
      } catch (e) { push.error(e?.data?.message || 'Failed') }
    },
    async remove() {
      if (!confirm(`Delete workspace "${this.workspace.name}" and all teams/members?`)) return
      try {
        await this.$http.$delete(`/api/workspaces/${this.workspace.id}`)
        push.success('Workspace deleted')
        this.$router.push('/workspaces')
      } catch (e) { push.error(e?.data?.message || 'Failed') }
    },
  },
}
</script>

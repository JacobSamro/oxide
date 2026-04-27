<template>
  <div class="space-y-8">
    <div>
      <h1 class="text-3xl font-bold">Overview</h1>
      <p class="text-muted-foreground text-sm">Live stats from the oxide proxy.</p>
    </div>

    <div v-if="!stats" class="text-sm text-muted-foreground">Loading…</div>
    <div v-else-if="unreachable" class="rounded-md border border-destructive/30 bg-destructive/5 p-4 text-sm">
      <div class="font-medium text-destructive">Proxy unreachable</div>
      <div class="text-muted-foreground">Confirm <code>oxide</code> is listening at <code>{{ proxyUrl }}</code>.</div>
    </div>
    <div v-else class="grid grid-cols-1 md:grid-cols-3 gap-4">
      <Stat label="Metadata cache hits" :value="stats.metadata.hits" hint="Fresh in-memory or disk hits" />
      <Stat label="Stale-while-revalidate hits" :value="stats.metadata.swr" hint="Served instantly from stale, refreshed in background" />
      <Stat label="Cold misses" :value="stats.metadata.misses" />
      <Stat label="Coalesced requests" :value="stats.coalesced.metadata + stats.coalesced.tarball" hint="Requests joined to an in-flight upstream fetch" />
      <Stat label="Tarball cache hits" :value="stats.tarballs.hits" />
      <Stat label="Upstream 429s" :value="stats.rateLimited" />
      <Stat label="Active metadata fetches" :value="stats.activeMetaFetches" />
      <Stat label="Active tarball streams" :value="stats.activeTarballStreams" />
      <Stat label="Metadata memory bytes" :value="formatBytes(stats.memCacheBytes)" />
    </div>

    <Card>
      <CardHeader>
        <CardTitle>Cache controls</CardTitle>
        <CardDescription>Invalidate metadata for a package by name.</CardDescription>
      </CardHeader>
      <CardContent class="flex gap-2">
        <Input v-model="invalidatePkg" placeholder="lodash or @scope/pkg" class="max-w-sm" />
        <Button :disabled="!invalidatePkg" @click="invalidate">
          <Trash2 class="size-4 mr-1" /> Invalidate
        </Button>
      </CardContent>
    </Card>
  </div>
</template>

<script>
import { Trash2 } from 'lucide-vue-next'
import { push } from 'notivue'
import Stat from '~/components/Stat.vue'

export default {
  components: { Stat, Trash2 },
  data() {
    return { stats: null, unreachable: false, invalidatePkg: '', proxyUrl: '', timer: null }
  },
  mounted() {
    this.proxyUrl = this.$config.public.proxyPublicUrl
    this.refresh()
    this.timer = setInterval(this.refresh, 5000)
  },
  beforeUnmount() { if (this.timer) clearInterval(this.timer) },
  methods: {
    formatBytes(n) {
      if (!n) return '0'
      const units = ['B', 'KB', 'MB', 'GB']
      let i = 0; let v = Number(n)
      while (v >= 1024 && i < units.length - 1) { v /= 1024; i++ }
      return `${v.toFixed(1)} ${units[i]}`
    },
    async refresh() {
      try {
        const res = await this.$http.$get('/api/proxy/stats')
        if (res && res.success) {
          this.unreachable = !!res.unreachable
          this.stats = res.summary
        }
      } catch (e) { /* keep last */ }
    },
    async invalidate() {
      try {
        const res = await this.$http.$post('/api/proxy/invalidate', { body: { package: this.invalidatePkg } })
        if (res && res.success) push.success(res.message)
        else push.error(res?.message || 'Failed')
      } catch (e) { push.error(e?.data?.message || 'Failed') }
    },
  },
}
</script>

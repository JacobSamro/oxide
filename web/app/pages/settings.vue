<template>
  <div class="space-y-6 max-w-3xl">
    <div>
      <h1 class="text-3xl font-bold">Settings</h1>
      <p class="text-muted-foreground text-sm">Domain, SSL, and storage configuration. Changes take effect immediately.</p>
    </div>

    <Tabs default-value="domain" class="w-full">
      <TabsList>
        <TabsTrigger value="domain"><Globe class="size-4 mr-1" />Domain</TabsTrigger>
        <TabsTrigger value="ssl"><Lock class="size-4 mr-1" />SSL / Let's Encrypt</TabsTrigger>
        <TabsTrigger value="s3"><Cloud class="size-4 mr-1" />S3 storage</TabsTrigger>
        <TabsTrigger value="client"><Terminal class="size-4 mr-1" />Client</TabsTrigger>
      </TabsList>

      <!-- Domain -->
      <TabsContent value="domain" class="pt-4">
        <Card>
          <CardHeader>
            <CardTitle>Domain</CardTitle>
            <CardDescription>The public hostname clients use to reach the registry.</CardDescription>
          </CardHeader>
          <CardContent class="space-y-3">
            <div>
              <Label>Primary domain</Label>
              <Input v-model="settings.domain.primaryDomain" placeholder="registry.example.com" />
            </div>
            <div>
              <Label>Additional domains (comma separated)</Label>
              <Input v-model="extraDomainsRaw" placeholder="alt.example.com, npm.example.com" />
            </div>
            <div>
              <Label>Public URL</Label>
              <Input v-model="settings.domain.publicUrl" placeholder="https://registry.example.com" />
              <p class="text-xs text-muted-foreground mt-1">Used to rewrite tarball URLs in package metadata.</p>
            </div>
          </CardContent>
          <CardFooter>
            <Button :disabled="saving" @click="save('domain')">{{ saving === 'domain' ? 'Saving…' : 'Save domain' }}</Button>
          </CardFooter>
        </Card>
      </TabsContent>

      <!-- SSL -->
      <TabsContent value="ssl" class="pt-4">
        <Card>
          <CardHeader>
            <CardTitle>SSL / Let's Encrypt</CardTitle>
            <CardDescription>Automatic HTTPS via ACME. Requires port 80 + 443 to be reachable from the internet on the configured domains.</CardDescription>
          </CardHeader>
          <CardContent class="space-y-3">
            <div class="flex items-center gap-2">
              <Checkbox v-model:checked="settings.ssl.enabled" id="sslEnabled" />
              <Label for="sslEnabled">Enable HTTPS with Let's Encrypt</Label>
            </div>
            <div>
              <Label>Contact email</Label>
              <Input v-model="settings.ssl.acmeEmail" type="email" placeholder="ops@example.com" />
              <p class="text-xs text-muted-foreground mt-1">Required by Let's Encrypt for expiry notices.</p>
            </div>
            <div class="flex items-center gap-2">
              <Checkbox v-model:checked="settings.ssl.staging" id="sslStaging" />
              <Label for="sslStaging">Use staging directory (recommended while testing)</Label>
            </div>
            <div class="flex items-center gap-2">
              <Checkbox v-model:checked="settings.ssl.httpRedirect" id="sslRedirect" />
              <Label for="sslRedirect">Redirect HTTP to HTTPS</Label>
            </div>
          </CardContent>
          <CardFooter>
            <Button :disabled="saving" @click="save('ssl')">{{ saving === 'ssl' ? 'Saving…' : 'Save SSL' }}</Button>
          </CardFooter>
        </Card>
      </TabsContent>

      <!-- S3 -->
      <TabsContent value="s3" class="pt-4">
        <Card>
          <CardHeader>
            <CardTitle>S3 storage</CardTitle>
            <CardDescription>Optional: persist tarballs to S3 (or any S3-compatible service). Disabled = local filesystem.</CardDescription>
          </CardHeader>
          <CardContent class="space-y-3">
            <div class="flex items-center gap-2">
              <Checkbox v-model:checked="settings.s3.enabled" id="s3Enabled" />
              <Label for="s3Enabled">Use S3 backend</Label>
            </div>
            <div class="grid grid-cols-2 gap-3">
              <div>
                <Label>Endpoint (blank = AWS)</Label>
                <Input v-model="settings.s3.endpoint" placeholder="https://s3.us-east-1.amazonaws.com" />
              </div>
              <div>
                <Label>Region</Label>
                <Input v-model="settings.s3.region" />
              </div>
              <div>
                <Label>Bucket</Label>
                <Input v-model="settings.s3.bucket" />
              </div>
              <div>
                <Label>Path prefix</Label>
                <Input v-model="settings.s3.pathPrefix" />
              </div>
              <div>
                <Label>Access key</Label>
                <Input v-model="settings.s3.accessKey" />
              </div>
              <div>
                <Label>Secret key</Label>
                <Input v-model="settings.s3.secretKey" type="password" />
              </div>
            </div>
            <div class="flex items-center gap-2">
              <Checkbox v-model:checked="settings.s3.pathStyle" id="pathStyle" />
              <Label for="pathStyle">Path-style URLs (MinIO etc.)</Label>
            </div>
          </CardContent>
          <CardFooter>
            <Button :disabled="saving" @click="save('s3')">{{ saving === 's3' ? 'Saving…' : 'Save S3' }}</Button>
          </CardFooter>
        </Card>
      </TabsContent>

      <!-- Client snippet -->
      <TabsContent value="client" class="pt-4">
        <Card>
          <CardHeader>
            <CardTitle>npm client</CardTitle>
            <CardDescription>Point your <code>.npmrc</code> at this URL.</CardDescription>
          </CardHeader>
          <CardContent>
            <pre class="bg-muted rounded-md p-3 text-sm overflow-x-auto">{{ snippet }}</pre>
          </CardContent>
        </Card>
      </TabsContent>
    </Tabs>
  </div>
</template>

<script>
import { Globe, Lock, Cloud, Terminal } from 'lucide-vue-next'
import { push } from 'notivue'

export default {
  components: { Globe, Lock, Cloud, Terminal },
  data() {
    return {
      settings: {
        domain: { primaryDomain: '', extraDomains: [], publicUrl: '' },
        ssl: { enabled: false, acmeEmail: '', staging: true, httpRedirect: true },
        s3: { enabled: false, endpoint: '', region: 'us-east-1', bucket: '', accessKey: '', secretKey: '', pathPrefix: 'tarballs/', pathStyle: false },
      },
      extraDomainsRaw: '',
      saving: '',
    }
  },
  computed: {
    snippet() {
      const url = this.settings.domain.publicUrl || this.$config.public.proxyPublicUrl
      return `# .npmrc\nregistry=${url.replace(/\/$/, '')}/\n`
    },
  },
  mounted() { this.load() },
  methods: {
    async load() {
      try {
        const res = await this.$http.$get('/api/settings')
        if (res && res.success) {
          this.settings = res.settings
          this.extraDomainsRaw = (res.settings.domain.extraDomains || []).join(', ')
        }
      } catch (e) { push.error(e?.data?.message || 'Failed to load settings') }
    },
    async save(section) {
      this.saving = section
      try {
        if (section === 'domain') {
          this.settings.domain.extraDomains = this.extraDomainsRaw
            .split(',').map((s) => s.trim()).filter(Boolean)
        }
        const body = this.settings[section]
        const res = await this.$http.$put(`/api/settings/${section}`, { body })
        if (res && res.success) push.success(res.message || 'Saved')
        else push.error(res?.message || 'Failed')
        await this.load()
      } catch (e) { push.error(e?.data?.message || 'Failed') }
      finally { this.saving = '' }
    },
  },
}
</script>

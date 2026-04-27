<template>
  <NuxtLayout>
    <NuxtPage />
  </NuxtLayout>
  <Notivue v-slot="item">
    <Notification :item="item" />
  </Notivue>
</template>

<script>
import { Notivue, Notification } from 'notivue'

export default {
  components: { Notivue, Notification },
}
</script>

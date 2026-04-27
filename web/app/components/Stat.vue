<template>
  <Card>
    <CardHeader class="pb-2">
      <CardDescription>{{ label }}</CardDescription>
      <CardTitle class="text-3xl tabular-nums">{{ value }}</CardTitle>
    </CardHeader>
    <CardContent v-if="hint" class="pt-0">
      <p class="text-xs text-muted-foreground">{{ hint }}</p>
    </CardContent>
  </Card>
</template>

<script>
export default {
  props: {
    label: { type: String, required: true },
    value: { type: [String, Number], required: true },
    hint: { type: String, default: '' },
  },
}
</script>

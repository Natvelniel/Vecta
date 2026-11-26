<script setup lang="ts">
withDefaults(
  defineProps<{
    compact?: boolean;
  }>(),
  { compact: false },
);
</script>

<template>
  <div class="v-card" :class="{ 'v-card--compact': compact }">
    <slot />
  </div>
</template>

<style scoped>
.v-card {
  min-width: 0;
  max-width: 100%;
  background: var(--bg-elevated);
  border: 1px solid var(--border-default);
  border-radius: var(--radius-card);
  padding: var(--space-6);
}
.v-card--compact {
  padding: var(--space-5);
}
@media (max-width: 639px) {
  .v-card {
    padding: var(--space-4);
  }
  .v-card--compact {
    padding: var(--space-3);
  }
}
</style>

<script setup lang="ts">
defineProps<{
  title: string;
  subtitle?: string;
}>();
</script>

<template>
  <header class="v-page-header">
    <div class="v-page-header__left">
      <h1 class="t-page" style="font-size: 22px">{{ title }}</h1>
      <slot name="title-extra" />
      <span v-if="subtitle" class="t-meta t-num">{{ subtitle }}</span>
    </div>
    <div class="v-page-header__actions">
      <slot name="actions" />
    </div>
  </header>
</template>

<style scoped>
.v-page-header {
  min-height: 64px;
  flex-shrink: 0;
  padding: var(--space-3) var(--space-8);
  border-bottom: 1px solid var(--border-divider);
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: var(--space-4);
  background: var(--bg-primary);
}
.v-page-header__left {
  display: flex;
  align-items: baseline;
  gap: var(--space-3);
  flex-wrap: wrap;
  min-width: 0;
}
.v-page-header__actions {
  display: flex;
  align-items: center;
  gap: var(--space-3);
  flex-wrap: wrap;
}
@media (max-width: 1023px) {
  .v-page-header {
    flex-direction: column;
    align-items: stretch;
    padding: var(--space-3) var(--space-4);
    gap: var(--space-3);
  }
  .v-page-header__actions {
    width: 100%;
  }
  .v-page-header__actions > :deep(button),
  .v-page-header__actions > :deep(select) {
    flex: 1;
  }
}
</style>

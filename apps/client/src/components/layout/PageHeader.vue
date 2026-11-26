<script setup lang="ts">
defineProps<{
  title: string;
  subtitle?: string;
  compactMobile?: boolean;
}>();
</script>

<template>
  <header class="v-page-header" :class="{ 'v-page-header--compact-mobile': compactMobile }">
    <div class="v-page-header__left">
      <h1 class="t-page v-page-header__title">{{ title }}</h1>
      <slot name="title-extra" />
      <span v-if="subtitle" class="t-meta t-num v-page-header__subtitle">{{ subtitle }}</span>
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
.v-page-header__title {
  font-size: 22px;
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
  .v-page-header--compact-mobile {
    min-height: 52px;
    flex-direction: row;
    align-items: center;
    justify-content: space-between;
    padding: var(--space-2) var(--space-4);
    gap: var(--space-2);
  }
  .v-page-header--compact-mobile .v-page-header__left {
    flex: 1;
    gap: 6px;
  }
  .v-page-header--compact-mobile .v-page-header__title {
    font-size: 18px;
  }
  .v-page-header--compact-mobile .v-page-header__subtitle {
    display: block;
    width: 100%;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .v-page-header--compact-mobile .v-page-header__actions {
    width: auto;
    flex: 0 0 auto;
    flex-wrap: nowrap;
    gap: 6px;
  }
  .v-page-header--compact-mobile .v-page-header__actions > :deep(button),
  .v-page-header--compact-mobile .v-page-header__actions > :deep(select) {
    flex: 0 0 auto;
  }
}
</style>

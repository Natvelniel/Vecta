<script setup lang="ts">
export type BadgeTone = "neutral" | "brand" | "info" | "success" | "warning" | "error" | "ai";

withDefaults(
  defineProps<{
    tone?: BadgeTone;
    wrap?: boolean;
  }>(),
  { tone: "neutral", wrap: false },
);
</script>

<template>
  <span class="v-badge" :class="[`v-badge--${tone}`, { 'v-badge--wrap': wrap }]">
    <slot />
  </span>
</template>

<style scoped>
.v-badge {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  padding: 3px 8px;
  border-radius: var(--radius-sm);
  font-size: 12px;
  font-weight: 500;
  white-space: nowrap;
}
.v-badge--wrap {
  white-space: normal;
  max-width: 100%;
  line-height: 1.35;
  text-align: left;
  overflow-wrap: break-word;
}
.v-badge--neutral {
  background: var(--bg-hover);
  color: var(--text-secondary);
}
.v-badge--brand {
  background: var(--brand-soft);
  color: var(--text-primary);
}
.v-badge--info {
  background: var(--info-soft);
  color: var(--status-info);
}
.v-badge--success {
  background: var(--success-soft);
  color: var(--status-success);
}
.v-badge--warning {
  background: var(--warning-soft);
  color: var(--status-warning);
}
.v-badge--error {
  background: var(--error-soft);
  color: var(--status-error);
}
.v-badge--ai {
  background: var(--ai-soft);
  color: var(--ai-accent);
  border: 1px solid var(--ai-border);
}
</style>

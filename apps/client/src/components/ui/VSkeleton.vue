<script setup lang="ts">
withDefaults(
  defineProps<{
    width?: string;
    height?: string;
    radius?: string;
  }>(),
  {
    width: "100%",
    height: "16px",
    radius: "var(--radius-sm)",
  },
);
</script>

<template>
  <span
    class="v-skeleton"
    aria-hidden="true"
    :style="{ width, height, borderRadius: radius }"
  />
</template>

<style scoped>
.v-skeleton {
  display: block;
  background: var(--bg-hover);
  animation: v-skeleton-pulse 1.5s var(--ease-state) infinite;
}
@keyframes v-skeleton-pulse {
  0%,
  100% {
    opacity: 0.45;
  }
  50% {
    opacity: 0.85;
  }
}
@media (prefers-reduced-motion: reduce) {
  .v-skeleton {
    animation: none;
    opacity: 0.6;
  }
}
</style>

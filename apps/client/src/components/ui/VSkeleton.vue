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
  background: linear-gradient(
    110deg,
    var(--surface-2) 20%,
    var(--surface-hover) 40%,
    var(--surface-2) 65%
  );
  background-size: 220% 100%;
  animation: vecta-shimmer 1.5s linear infinite;
}
@media (prefers-reduced-motion: reduce) {
  .v-skeleton {
    animation: none;
    opacity: 0.6;
  }
}
</style>

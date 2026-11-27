<script setup lang="ts">
import { computed } from "vue";

const props = withDefaults(
  defineProps<{
    score: number;
    size?: number;
    strokeWidth?: number;
  }>(),
  {
    size: 44,
    strokeWidth: 3.5,
  },
);

const targetPct = computed(() => Math.max(0, Math.min(100, Math.round(props.score))));
const radius = computed(() => (props.size - props.strokeWidth * 2) / 2);
const circumference = computed(() => 2 * Math.PI * radius.value);
const dashOffset = computed(() => circumference.value * (1 - targetPct.value / 100));

const toneColor = computed(() => {
  if (targetPct.value >= 75) return "var(--status-success)";
  if (targetPct.value >= 50) return "var(--status-warning)";
  return "var(--text-muted)";
});
</script>

<template>
  <div
    class="score-ring"
    :style="{ width: `${size}px`, height: `${size}px` }"
    :aria-label="`Skor kecocokan ${targetPct}%`"
  >
    <svg :width="size" :height="size" :viewBox="`0 0 ${size} ${size}`" class="ring-svg">
      <circle
        class="ring-track"
        :cx="size / 2"
        :cy="size / 2"
        :r="radius"
        :stroke-width="strokeWidth"
      />
      <circle
        class="ring-progress"
        :cx="size / 2"
        :cy="size / 2"
        :r="radius"
        :stroke-width="strokeWidth"
        :stroke="toneColor"
        :stroke-dasharray="circumference"
        :stroke-dashoffset="dashOffset"
      />
    </svg>
    <span class="ring-val t-num" :style="{ color: toneColor }">{{ targetPct }}</span>
  </div>
</template>

<style scoped>
.score-ring {
  position: relative;
  display: inline-flex;
  align-items: center;
  justify-content: center;
  flex-shrink: 0;
}
.ring-svg {
  transform: rotate(-90deg);
}
.ring-track {
  fill: none;
  stroke: var(--border);
}
.ring-progress {
  fill: none;
  stroke-linecap: round;
  transition: stroke-dashoffset var(--dur-meter) var(--ease-luxury), stroke 220ms ease-out;
}
.ring-val {
  position: absolute;
  font-size: 12px;
  font-weight: 700;
  line-height: 1;
}
</style>

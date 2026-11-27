<script setup lang="ts">
const props = defineProps<{
  modelValue: boolean;
  label: string;
}>();

const emit = defineEmits<{
  (e: "update:modelValue", value: boolean): void;
}>();

function toggle() {
  emit("update:modelValue", !props.modelValue);
}
</script>

<template>
  <label class="v-switch-wrap">
    <button
      type="button"
      role="switch"
      :aria-checked="modelValue"
      :aria-label="label"
      class="v-switch"
      @click="toggle"
    >
      <span class="v-switch__thumb" />
    </button>
    <span class="t-secondary">{{ label }}</span>
  </label>
</template>

<style scoped>
.v-switch-wrap {
  display: inline-flex;
  align-items: center;
  gap: 10px;
  cursor: pointer;
}
.v-switch {
  position: relative;
  width: 40px;
  height: 22px;
  border-radius: var(--radius-full);
  border: 1px solid var(--border-default);
  background: var(--bg-elevated);
  padding: 0;
  transition:
    background var(--dur-hover) var(--ease-state),
    border-color var(--dur-hover) var(--ease-state);
}
.v-switch[aria-checked="true"] {
  background: var(--brand-primary);
  border-color: var(--brand-primary);
}
.v-switch__thumb {
  position: absolute;
  top: 2px;
  left: 2px;
  width: 16px;
  height: 16px;
  border-radius: var(--radius-full);
  background: var(--text-primary);
  transition:
    transform 300ms var(--ease-glide),
    background-color 220ms ease-out;
}
.v-switch[aria-checked="true"] .v-switch__thumb {
  transform: translateX(18px);
  background: var(--brand-fg);
}
</style>

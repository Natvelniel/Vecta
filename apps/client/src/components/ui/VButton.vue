<script setup lang="ts">
export type ButtonVariant = "primary" | "secondary" | "ai" | "tertiary" | "destructive";
export type ButtonSize = "default" | "compact" | "tall";

withDefaults(
  defineProps<{
    variant?: ButtonVariant;
    size?: ButtonSize;
    type?: "button" | "submit" | "reset";
    disabled?: boolean;
    loading?: boolean;
    pressed?: boolean;
  }>(),
  {
    variant: "secondary",
    size: "default",
    type: "button",
    disabled: false,
    loading: false,
    pressed: undefined,
  },
);
</script>

<template>
  <button
    :type="type"
    :disabled="disabled || loading"
    :aria-busy="loading ? 'true' : undefined"
    :aria-pressed="pressed"
    class="v-btn"
    :class="[`v-btn--${variant}`, `v-btn--${size}`]"
  >
    <span v-if="loading" class="v-btn__spinner" aria-hidden="true" />
    <slot />
  </button>
</template>

<style scoped>
.v-btn {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  gap: var(--space-2);
  border-radius: var(--radius-control);
  border: 1px solid transparent;
  font-size: 13px;
  font-weight: 500;
  white-space: nowrap;
  text-decoration: none;
  background: transparent;
  color: var(--text-primary);
  transition:
    background var(--dur-hover) var(--ease-state),
    border-color var(--dur-hover) var(--ease-state),
    color var(--dur-hover) var(--ease-state),
    transform var(--dur-press) ease-out;
}

.v-btn:not(:disabled):active {
  transform: scale(0.96);
}

.v-btn--default {
  height: 40px;
  padding: 0 var(--space-4);
}
.v-btn--tall {
  height: 44px;
  padding: 0 var(--space-4);
}
.v-btn--compact {
  height: 34px;
  padding: 0 var(--space-3);
}

.v-btn--primary {
  background: var(--brand);
  color: var(--brand-fg);
  font-weight: 600;
  box-shadow: var(--shadow-control);
}
.v-btn--primary:not(:disabled):active {
  background: var(--brand-active);
  color: var(--brand-fg);
}

.v-btn--secondary {
  background: var(--bg-elevated);
  border-color: var(--border-default);
  color: var(--text-primary);
  box-shadow: var(--shadow-control);
}
.v-btn--secondary[aria-pressed="true"] {
  background: var(--brand-soft);
  border-color: var(--brand-border);
  color: var(--text-primary);
}

.v-btn--ai {
  background: var(--ai-soft);
  border-color: var(--ai-border);
  color: var(--ai-accent);
  box-shadow: var(--shadow-control);
}

.v-btn--tertiary {
  color: var(--text-secondary);
  padding: 0 var(--space-3);
}

.v-btn--destructive {
  background: var(--error-soft);
  border-color: var(--error-border);
  color: var(--status-error);
}

@media (hover: hover) {
  .v-btn--primary:hover:not(:disabled) {
    background: var(--brand-hover);
    color: var(--brand-fg);
  }
  .v-btn--secondary:hover:not(:disabled) {
    background: var(--bg-hover);
  }
  .v-btn--ai:hover:not(:disabled) {
    background: var(--surface-hover);
    color: var(--text-1);
  }
  .v-btn--tertiary:hover:not(:disabled) {
    background: var(--bg-hover);
    color: var(--text-primary);
  }
  .v-btn--destructive:hover:not(:disabled) {
    background: var(--error-soft);
    filter: brightness(1.08);
  }
}

.v-btn__spinner {
  width: 14px;
  height: 14px;
  border-radius: var(--radius-full);
  border: 2px solid currentColor;
  border-right-color: transparent;
  animation: v-spin 0.65s linear infinite;
}

@keyframes v-spin {
  to {
    transform: rotate(360deg);
  }
}

@media (max-width: 1023px) {
  .v-btn--default,
  .v-btn--tall {
    min-height: 44px;
  }
  .v-btn--compact {
    min-height: 40px;
  }
}
</style>

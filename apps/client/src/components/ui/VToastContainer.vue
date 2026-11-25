<script setup lang="ts">
import { useToast } from "@/composables/useToast";
import VButton from "./VButton.vue";
import VIcon from "./VIcon.vue";

const { toasts, dismiss } = useToast();

async function runAction(id: number, fn?: () => unknown) {
  if (fn) await fn();
  dismiss(id);
}
</script>

<template>
  <div class="v-toast-host" aria-live="polite" aria-atomic="false">
    <div
      v-for="t in toasts"
      :key="t.id"
      class="v-toast"
      :class="`v-toast--${t.tone}`"
      role="status"
    >
      <span class="v-toast__dot" aria-hidden="true" />
      <p class="v-toast__msg">{{ t.message }}</p>
      <VButton
        v-if="t.actionLabel"
        variant="primary"
        size="compact"
        @click="runAction(t.id, t.onAction)"
      >
        {{ t.actionLabel }}
      </VButton>
      <button
        type="button"
        class="v-toast__close"
        aria-label="Tutup notifikasi"
        @click="dismiss(t.id)"
      >
        <VIcon name="close" :size="13" />
      </button>
    </div>
  </div>
</template>

<style scoped>
.v-toast-host {
  position: fixed;
  right: var(--space-6);
  bottom: var(--space-6);
  z-index: 90;
  display: flex;
  flex-direction: column;
  gap: var(--space-2);
  max-width: 440px;
}
.v-toast {
  background: var(--bg-elevated);
  border: 1px solid var(--border-default);
  border-radius: var(--radius-card);
  padding: var(--space-3) var(--space-4);
  box-shadow: var(--shadow-overlay);
  display: flex;
  align-items: center;
  gap: var(--space-3);
}
.v-toast__dot {
  width: 6px;
  height: 6px;
  border-radius: var(--radius-full);
  flex-shrink: 0;
  background: var(--brand-hover);
}
.v-toast--success .v-toast__dot {
  background: var(--status-success);
}
.v-toast--warning .v-toast__dot {
  background: var(--status-warning);
}
.v-toast--error .v-toast__dot {
  background: var(--status-error);
}
.v-toast__msg {
  margin: 0;
  flex: 1;
  font-size: 13px;
  line-height: 1.45;
  color: var(--text-primary);
}
.v-toast__close {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: 28px;
  height: 28px;
  border: 0;
  border-radius: var(--radius-sm);
  background: transparent;
  color: var(--text-secondary);
}
.v-toast__close:hover {
  background: var(--bg-hover);
  color: var(--text-primary);
}
@media (max-width: 1023px) {
  .v-toast-host {
    left: var(--space-4);
    right: var(--space-4);
    bottom: 74px;
    max-width: none;
  }
}
</style>

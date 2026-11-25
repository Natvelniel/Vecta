<script setup lang="ts">
import { nextTick, onBeforeUnmount, onMounted, ref, watch } from "vue";
import VIcon from "./VIcon.vue";

const props = defineProps<{
  open: boolean;
  title: string;
}>();

const emit = defineEmits<{
  (e: "close"): void;
}>();

const panelRef = ref<HTMLElement | null>(null);
let previouslyFocused: HTMLElement | null = null;

const FOCUSABLE_SELECTOR = [
  "a[href]",
  "button:not([disabled])",
  "input:not([disabled])",
  "select:not([disabled])",
  "textarea:not([disabled])",
  "[tabindex]:not([tabindex='-1'])",
].join(", ");

function getFocusableElements(): HTMLElement[] {
  if (!panelRef.value) return [];
  return Array.from(panelRef.value.querySelectorAll<HTMLElement>(FOCUSABLE_SELECTOR)).filter(
    (el) => el.offsetParent !== null,
  );
}

watch(
  () => props.open,
  async (isOpen) => {
    if (isOpen) {
      previouslyFocused =
        document.activeElement instanceof HTMLElement ? document.activeElement : null;
      await nextTick();
      const focusables = getFocusableElements();
      if (focusables.length > 0) {
        focusables[0].focus();
      } else {
        panelRef.value?.focus();
      }
    } else if (previouslyFocused) {
      previouslyFocused.focus();
      previouslyFocused = null;
    }
  },
);

function handleKeydown(e: KeyboardEvent) {
  if (!props.open) return;
  if (e.key === "Escape") {
    emit("close");
    return;
  }
  if (e.key === "Tab") {
    const focusables = getFocusableElements();
    if (focusables.length === 0) {
      e.preventDefault();
      panelRef.value?.focus();
      return;
    }
    const first = focusables[0];
    const last = focusables[focusables.length - 1];
    const active = document.activeElement;
    if (e.shiftKey) {
      if (active === first || !panelRef.value?.contains(active)) {
        e.preventDefault();
        last.focus();
      }
    } else if (active === last || !panelRef.value?.contains(active)) {
      e.preventDefault();
      first.focus();
    }
  }
}

onMounted(() => window.addEventListener("keydown", handleKeydown));
onBeforeUnmount(() => {
  window.removeEventListener("keydown", handleKeydown);
  if (previouslyFocused) {
    previouslyFocused.focus();
    previouslyFocused = null;
  }
});
</script>

<template>
  <Teleport to="body">
    <div
      v-if="open"
      class="v-dialog-backdrop"
      role="dialog"
      aria-modal="true"
      :aria-label="title"
      @click.self="emit('close')"
    >
      <div ref="panelRef" class="v-dialog-panel" tabindex="-1">
        <div class="v-dialog-head">
          <h2 class="t-section" style="font-size: 20px">{{ title }}</h2>
          <button
            type="button"
            class="v-icon-btn"
            aria-label="Tutup dialog"
            @click="emit('close')"
          >
            <VIcon name="close" :size="15" />
          </button>
        </div>
        <slot />
      </div>
    </div>
  </Teleport>
</template>

<style scoped>
.v-dialog-backdrop {
  position: fixed;
  inset: 0;
  background: rgba(7, 10, 18, 0.72);
  display: flex;
  align-items: center;
  justify-content: center;
  padding: var(--space-5);
  z-index: 80;
}
.v-dialog-panel {
  width: 100%;
  max-width: 540px;
  background: var(--bg-elevated);
  border: 1px solid var(--border-default);
  border-radius: var(--radius-dialog);
  padding: var(--space-6);
  box-shadow: var(--shadow-overlay);
}
.v-dialog-panel:focus {
  outline: none;
}
.v-dialog-head {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: var(--space-3);
  margin-bottom: var(--space-5);
}
.v-icon-btn {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: 36px;
  height: 36px;
  border-radius: var(--radius-control);
  border: 1px solid transparent;
  background: transparent;
  color: var(--text-secondary);
  transition: background var(--dur-hover) var(--ease-state), color var(--dur-hover) var(--ease-state);
}
.v-icon-btn:hover {
  background: var(--bg-hover);
  color: var(--text-primary);
}
</style>

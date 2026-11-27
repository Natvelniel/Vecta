<script setup lang="ts">
import { computed, nextTick, onBeforeUnmount, ref, watch } from "vue";
import { useBackOverlay } from "@/composables/useBackOverlay";
import VIcon from "@/components/ui/VIcon.vue";

export interface VSelectOption<T extends string | number | null = string | number | null> {
  value: T;
  label: string;
  hint?: string;
}

const props = withDefaults(
  defineProps<{
    modelValue: string | number | null;
    options: Array<VSelectOption>;
    id?: string;
    label?: string;
    placeholder?: string;
    disabled?: boolean;
    compact?: boolean;
    fullWidth?: boolean;
    ariaLabel?: string;
  }>(),
  {
    id: undefined,
    label: "",
    placeholder: "Pilih opsi...",
    disabled: false,
    compact: false,
    fullWidth: true,
    ariaLabel: "",
  },
);

const emit = defineEmits<{
  (e: "update:modelValue", value: string | number | null): void;
  (e: "change", value: string | number | null): void;
}>();

const open = ref(false);
const isMobileSheet = ref(false);
const triggerRef = ref<HTMLButtonElement | null>(null);
const listRef = ref<HTMLElement | null>(null);
const activeIndex = ref(0);
const desktopCoords = ref<{ top: number; left: number; width: number; flipUp: boolean }>({
  top: 0,
  left: 0,
  width: 220,
  flipUp: false,
});

useBackOverlay(open, () => {
  open.value = false;
}, "vselect");

const selectedOption = computed(() =>
  props.options.find((opt) => opt.value === props.modelValue) ?? null,
);

const displayLabel = computed(
  () => selectedOption.value?.label ?? props.placeholder,
);

function syncViewportMode() {
  if (typeof window === "undefined") return;
  isMobileSheet.value = window.innerWidth < 1024;
  if (!isMobileSheet.value && triggerRef.value && open.value) {
    const rect = triggerRef.value.getBoundingClientRect();
    const spaceBelow = window.innerHeight - rect.bottom;
    const flipUp = spaceBelow < 230 && rect.top > 220;
    desktopCoords.value = {
      top: flipUp ? rect.top - 6 : rect.bottom + 6,
      left: Math.min(rect.left, Math.max(12, window.innerWidth - Math.max(rect.width, 220) - 12)),
      width: Math.max(rect.width, 200),
      flipUp,
    };
  }
}

async function openMenu() {
  if (props.disabled || props.options.length === 0) return;
  syncViewportMode();
  const idx = props.options.findIndex((opt) => opt.value === props.modelValue);
  activeIndex.value = idx >= 0 ? idx : 0;
  open.value = true;
  await nextTick();
  syncViewportMode();
  listRef.value?.focus({ preventScroll: true });
}

function closeMenu() {
  open.value = false;
  triggerRef.value?.focus({ preventScroll: true });
}

function toggleMenu() {
  if (open.value) {
    closeMenu();
  } else {
    void openMenu();
  }
}

function choose(value: string | number | null) {
  emit("update:modelValue", value);
  emit("change", value);
  closeMenu();
}

function handleKeydown(e: KeyboardEvent) {
  if (!open.value) return;
  if (e.key === "Escape") {
    e.preventDefault();
    closeMenu();
    return;
  }
  if (e.key === "ArrowDown") {
    e.preventDefault();
    activeIndex.value = Math.min(props.options.length - 1, activeIndex.value + 1);
  } else if (e.key === "ArrowUp") {
    e.preventDefault();
    activeIndex.value = Math.max(0, activeIndex.value - 1);
  } else if (e.key === "Home") {
    e.preventDefault();
    activeIndex.value = 0;
  } else if (e.key === "End") {
    e.preventDefault();
    activeIndex.value = Math.max(0, props.options.length - 1);
  } else if (e.key === "Enter" || e.key === " ") {
    e.preventDefault();
    const opt = props.options[activeIndex.value];
    if (opt) choose(opt.value);
  }
}

watch(open, (val) => {
  if (typeof window === "undefined") return;
  if (val) {
    window.addEventListener("resize", syncViewportMode);
  } else {
    window.removeEventListener("resize", syncViewportMode);
  }
});

onBeforeUnmount(() => {
  if (typeof window !== "undefined") {
    window.removeEventListener("resize", syncViewportMode);
  }
});
</script>

<template>
  <div class="v-sel-wrap" :class="{ 'v-sel-wrap--full': fullWidth }">
    <button
      :id="id"
      ref="triggerRef"
      type="button"
      role="combobox"
      aria-haspopup="listbox"
      :aria-expanded="open"
      :aria-label="ariaLabel || label || placeholder"
      :disabled="disabled"
      class="v-sel-trigger"
      :class="{
        'v-sel-trigger--compact': compact,
        'v-sel-trigger--open': open,
        'v-sel-trigger--placeholder': !selectedOption,
      }"
      @click="toggleMenu"
    >
      <span class="v-sel-value">{{ displayLabel }}</span>
      <VIcon name="chevronDown" :size="14" class="v-sel-chevron" />
    </button>

    <Teleport to="body">
      <Transition name="v-backdrop-fade">
        <div
          v-if="open"
          class="v-sel-backdrop"
          :class="{ 'v-sel-backdrop--desktop': !isMobileSheet }"
          aria-hidden="true"
          @click="closeMenu"
        />
      </Transition>

      <Transition :name="isMobileSheet ? 'v-sheet-slide' : 'v-dialog-pop'">
        <div
          v-if="open"
          ref="listRef"
          class="v-sel-surface"
          :class="isMobileSheet ? 'v-sel-surface--sheet' : 'v-sel-surface--pop'"
          :style="
            !isMobileSheet
              ? {
                  top: `${desktopCoords.top}px`,
                  left: `${desktopCoords.left}px`,
                  minWidth: `${desktopCoords.width}px`,
                  transform: desktopCoords.flipUp ? 'translateY(-100%)' : undefined,
                  transformOrigin: desktopCoords.flipUp ? 'bottom left' : 'top left',
                }
              : undefined
          "
          role="listbox"
          :aria-label="ariaLabel || label || placeholder"
          tabindex="-1"
          @keydown="handleKeydown"
        >
          <div v-if="isMobileSheet" class="v-sel-grab" aria-hidden="true" />
          <div v-if="isMobileSheet && (label || ariaLabel)" class="v-sel-sheet-head">
            <span class="v-sel-sheet-title">{{ label || ariaLabel }}</span>
            <button
              type="button"
              class="v-sel-close"
              aria-label="Tutup pilihan"
              @click="closeMenu"
            >
              <VIcon name="close" :size="14" />
            </button>
          </div>

          <div class="v-sel-options">
            <button
              v-for="(opt, idx) in options"
              :key="String(opt.value ?? '__null__')"
              type="button"
              role="option"
              class="v-sel-option"
              :class="{
                'v-sel-option--selected': opt.value === modelValue,
                'v-sel-option--focused': idx === activeIndex,
              }"
              :aria-selected="opt.value === modelValue"
              @click="choose(opt.value)"
            >
              <div class="v-sel-option__copy">
                <span class="v-sel-option__label">{{ opt.label }}</span>
                <span v-if="opt.hint" class="v-sel-option__hint">{{ opt.hint }}</span>
              </div>
              <span v-if="opt.value === modelValue" class="v-sel-option__check" aria-hidden="true">
                <VIcon name="check" :size="14" />
              </span>
            </button>
          </div>
        </div>
      </Transition>
    </Teleport>
  </div>
</template>

<style scoped>
.v-sel-wrap {
  display: inline-flex;
  min-width: 0;
  max-width: 100%;
}
.v-sel-wrap--full {
  width: 100%;
}
.v-sel-trigger {
  width: 100%;
  min-width: 0;
  height: 40px;
  padding: 0 12px;
  border-radius: var(--radius-control);
  border: 1px solid var(--border-input);
  background: var(--surface-1);
  color: var(--text-1);
  box-shadow: var(--shadow-control);
  font-size: 13px;
  font-weight: 500;
  display: inline-flex;
  align-items: center;
  justify-content: space-between;
  gap: 8px;
  text-align: left;
  transition:
    border-color var(--dur-hover) var(--ease-luxury),
    background-color var(--dur-hover) var(--ease-luxury),
    transform var(--dur-press) var(--ease-luxury);
}
.v-sel-trigger--compact {
  height: 36px;
  padding: 0 10px 0 12px;
  font-size: 12.5px;
}
.v-sel-trigger--placeholder {
  color: var(--text-2);
}
.v-sel-trigger--open {
  border-color: var(--brand-hover);
}
.v-sel-trigger:not(:disabled):active {
  transform: scale(0.975);
}
.v-sel-value {
  flex: 1;
  min-width: 0;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.v-sel-chevron {
  flex-shrink: 0;
  color: var(--text-2);
  transition: transform 280ms var(--ease-luxury), color 180ms ease-out;
}
.v-sel-trigger--open .v-sel-chevron {
  transform: rotate(180deg);
  color: var(--text-1);
}

.v-sel-backdrop {
  position: fixed;
  inset: 0;
  z-index: 94;
  background: rgba(7, 10, 18, 0.58);
}
.v-sel-backdrop--desktop {
  background: transparent;
}
.v-sel-surface {
  position: fixed;
  z-index: 95;
  background: var(--surface-1);
  border: 1px solid var(--border-default);
  box-shadow: var(--shadow-overlay), inset 0 1px 0 rgba(255, 255, 255, 0.08);
  display: flex;
  flex-direction: column;
  overscroll-behavior: contain;
}
.v-sel-surface:focus {
  outline: none;
}
.v-sel-surface--pop {
  max-width: min(360px, calc(100vw - 24px));
  max-height: 290px;
  padding: 6px;
  border-radius: var(--radius-card);
}
.v-sel-surface--sheet {
  left: 8px;
  right: 8px;
  bottom: calc(8px + env(safe-area-inset-bottom, 0px));
  max-height: 64dvh;
  padding: 12px 12px 14px;
  border-radius: 22px;
}
.v-sel-grab {
  width: 38px;
  height: 4px;
  border-radius: var(--radius-full);
  background: var(--border-strong);
  margin: 0 auto 10px;
  flex-shrink: 0;
}
.v-sel-sheet-head {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 8px;
  padding: 0 4px 10px;
  margin-bottom: 6px;
  border-bottom: 1px solid var(--border-divider);
}
.v-sel-sheet-title {
  font-size: 14px;
  font-weight: 600;
  color: var(--text-1);
}
.v-sel-close {
  width: 34px;
  height: 34px;
  border-radius: var(--radius-control);
  border: 0;
  background: transparent;
  color: var(--text-2);
  display: inline-flex;
  align-items: center;
  justify-content: center;
}
.v-sel-options {
  overflow-y: auto;
  overscroll-behavior: contain;
  display: flex;
  flex-direction: column;
  gap: 3px;
}
.v-sel-option {
  width: 100%;
  min-height: 44px;
  padding: 9px 12px;
  border-radius: var(--radius-control);
  border: 1px solid transparent;
  background: transparent;
  color: var(--text-1);
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 10px;
  text-align: left;
  transition:
    background-color 180ms var(--ease-luxury),
    border-color 180ms var(--ease-luxury),
    transform 150ms ease-out;
}
.v-sel-option:not(:disabled):active {
  transform: scale(0.985);
}
@media (hover: hover) {
  .v-sel-option:hover,
  .v-sel-option--focused {
    background: var(--surface-hover);
  }
}
.v-sel-option--selected {
  background: var(--brand-soft);
  border-color: var(--brand-border);
  font-weight: 600;
}
.v-sel-option__copy {
  min-width: 0;
  flex: 1;
  display: flex;
  flex-direction: column;
  gap: 2px;
}
.v-sel-option__label {
  font-size: 13.5px;
  line-height: 1.35;
  overflow-wrap: break-word;
}
.v-sel-option__hint {
  font-size: 11.5px;
  color: var(--text-2);
}
.v-sel-option__check {
  flex-shrink: 0;
  color: var(--status-success);
  display: inline-flex;
}
</style>

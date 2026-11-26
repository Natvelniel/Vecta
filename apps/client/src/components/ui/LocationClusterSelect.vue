<script setup lang="ts">
import { computed, ref } from "vue";
import {
  formatLocationClusterLabel,
  searchLocationClusters,
} from "@/lib/locationClusters";
import VIcon from "@/components/ui/VIcon.vue";

const props = withDefaults(
  defineProps<{
    modelValue: string[];
    allLabel?: string;
    placeholder?: string;
  }>(),
  {
    allLabel: "Semua Wilayah (Nasional & Global)",
    placeholder: "Cari kota, provinsi, atau negara (mis. BSD, IKN, Solo)...",
  },
);

const emit = defineEmits<{
  (e: "update:modelValue", value: string[]): void;
}>();

const open = ref(false);
const searchQuery = ref("");

const groupedOptions = computed(() => searchLocationClusters(searchQuery.value));

const triggerSummary = computed(() => {
  if (props.modelValue.length === 0) return props.allLabel;
  if (props.modelValue.length === 1) {
    return formatLocationClusterLabel(props.modelValue[0]);
  }
  const first = formatLocationClusterLabel(props.modelValue[0]);
  return `${first} +${props.modelValue.length - 1}`;
});

function toggleCluster(id: string) {
  const exists = props.modelValue.includes(id);
  const next = exists
    ? props.modelValue.filter((v) => v !== id)
    : [...props.modelValue, id];
  emit("update:modelValue", next);
}

function clearAll() {
  emit("update:modelValue", []);
}
</script>

<template>
  <div class="v-lcs">
    <button
      type="button"
      class="v-lcs-trigger"
      :class="{ 'v-lcs-trigger--active': modelValue.length > 0, 'v-lcs-trigger--open': open }"
      :aria-expanded="open"
      aria-haspopup="listbox"
      @click="open = !open"
    >
      <span class="v-lcs-trigger__left">
        <VIcon name="mapPin" :size="14" class="v-lcs-pin" />
        <span class="v-lcs-trigger__text">{{ triggerSummary }}</span>
      </span>
      <span class="v-lcs-trigger__right">
        <span v-if="modelValue.length > 0" class="v-lcs-count t-num">
          {{ modelValue.length }}
        </span>
        <span
          v-if="modelValue.length > 0"
          role="button"
          tabindex="0"
          class="v-lcs-clear"
          aria-label="Reset kluster lokasi ke Semua Wilayah"
          @click.stop="clearAll"
          @keydown.enter.stop.prevent="clearAll"
        >
          <VIcon name="close" :size="11" />
        </span>
        <VIcon
          name="chevronDown"
          :size="14"
          class="v-lcs-chevron"
          :class="{ 'v-lcs-chevron--open': open }"
        />
      </span>
    </button>

    <!-- Selected Cluster Chips (Pure Color Pine Emerald, ZERO Checkmarks) -->
    <div v-if="modelValue.length > 0" class="v-lcs-selected" aria-label="Wilayah terpilih">
      <button
        v-for="id in modelValue"
        :key="id"
        type="button"
        class="v-lcs-chip"
        @click="toggleCluster(id)"
      >
        <span>{{ formatLocationClusterLabel(id) }}</span>
        <VIcon name="close" :size="10" />
      </button>
    </div>

    <!-- Inline Searchable Multi-Select Panel (Never Clipped inside Bottom Sheet) -->
    <Transition name="v-lcs-expand">
      <div v-if="open" class="v-lcs-panel" role="listbox" aria-multiselectable="true">
        <div class="v-lcs-search-wrap">
          <div class="v-lcs-search-box">
            <VIcon name="search" :size="13" class="v-lcs-search-icon" />
            <label for="lcs-search-input" class="sr-only">Cari kota atau provinsi</label>
            <input
              id="lcs-search-input"
              v-model="searchQuery"
              type="search"
              class="v-lcs-search-input"
              :placeholder="placeholder"
            />
            <button
              v-if="searchQuery.trim()"
              type="button"
              class="v-lcs-search-clear"
              aria-label="Hapus pencarian wilayah"
              @click="searchQuery = ''"
            >
              <VIcon name="close" :size="11" />
            </button>
          </div>
        </div>

        <div class="v-lcs-scroll">
          <button
            type="button"
            class="v-lcs-opt v-lcs-opt--all"
            :class="{ 'v-lcs-opt--all-active': modelValue.length === 0 }"
            @click="clearAll"
          >
            <span>{{ allLabel }}</span>
          </button>

          <p v-if="groupedOptions.length === 0" class="t-meta v-lcs-empty">
            Wilayah tidak ditemukan. Coba ketik nama kota, kabupaten, atau provinsi lain.
          </p>

          <div v-for="grp in groupedOptions" :key="grp.group" class="v-lcs-group">
            <div class="v-lcs-group-title">{{ grp.group }}</div>
            <div class="v-lcs-group-list">
              <button
                v-for="opt in grp.items"
                :key="opt.id"
                type="button"
                role="option"
                :aria-selected="modelValue.includes(opt.id)"
                class="v-lcs-opt"
                :class="{ 'v-lcs-opt--active': modelValue.includes(opt.id) }"
                @click="toggleCluster(opt.id)"
              >
                <span>{{ opt.label }}</span>
              </button>
            </div>
          </div>
        </div>
      </div>
    </Transition>
  </div>
</template>

<style scoped>
.v-lcs {
  display: flex;
  flex-direction: column;
  gap: 8px;
  min-width: 0;
  width: 100%;
}
.v-lcs-trigger {
  width: 100%;
  min-width: 0;
  min-height: 40px;
  padding: 6px 10px 6px 12px;
  border-radius: var(--radius-control);
  border: 1px solid var(--border);
  background: var(--surface-2);
  color: var(--text-1);
  box-shadow: var(--shadow-control);
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 8px;
  font-size: 12.5px;
  font-weight: 600;
  text-align: left;
  transition:
    background-color 140ms ease-out,
    border-color 140ms ease-out,
    transform var(--dur-press) ease-out;
}
.v-lcs-trigger:not(:disabled):active {
  transform: scale(0.985);
}
.v-lcs-trigger--active {
  border-color: var(--filter-active-border);
}
.v-lcs-trigger--open {
  border-color: var(--brand-border);
}
.v-lcs-trigger__left {
  display: inline-flex;
  align-items: center;
  gap: 8px;
  min-width: 0;
  flex: 1;
}
.v-lcs-pin {
  flex-shrink: 0;
  color: var(--text-2);
}
.v-lcs-trigger--active .v-lcs-pin {
  color: var(--status-success);
}
.v-lcs-trigger__text {
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.v-lcs-trigger__right {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  flex-shrink: 0;
}
.v-lcs-count {
  min-width: 20px;
  height: 20px;
  padding: 0 6px;
  border-radius: var(--radius-full);
  background: var(--filter-active-bg);
  color: var(--filter-active-fg);
  font-size: 11px;
  font-weight: 700;
  display: inline-flex;
  align-items: center;
  justify-content: center;
}
.v-lcs-clear {
  width: 22px;
  height: 22px;
  border-radius: var(--radius-full);
  background: var(--surface-hover);
  color: var(--text-2);
  display: inline-flex;
  align-items: center;
  justify-content: center;
}
.v-lcs-chevron {
  color: var(--text-2);
  transition: transform 180ms var(--ease-snappy);
}
.v-lcs-chevron--open {
  transform: rotate(180deg);
}
.v-lcs-selected {
  display: flex;
  flex-wrap: wrap;
  gap: 6px;
}
.v-lcs-chip {
  height: 26px;
  padding: 0 9px;
  border-radius: var(--radius-full);
  border: 1px solid var(--filter-active-border);
  background: var(--filter-active-bg);
  color: var(--filter-active-fg);
  font-size: 11.5px;
  font-weight: 600;
  display: inline-flex;
  align-items: center;
  gap: 5px;
  transition: background-color 140ms ease-out, transform var(--dur-press) ease-out;
}
.v-lcs-chip:not(:disabled):active {
  transform: scale(0.96);
}
.v-lcs-panel {
  border-radius: var(--radius-control);
  border: 1px solid var(--border);
  background: var(--surface-1);
  box-shadow: var(--shadow-surface);
  overflow: hidden;
  display: flex;
  flex-direction: column;
}
.v-lcs-search-wrap {
  padding: 8px;
  border-bottom: 1px solid var(--border-divider);
  background: var(--surface-1);
}
.v-lcs-search-box {
  height: 36px;
  padding: 0 8px 0 10px;
  border-radius: var(--radius-sm);
  border: 1px solid var(--border);
  background: var(--surface-recessed);
  box-shadow: var(--shadow-recessed);
  display: flex;
  align-items: center;
  gap: 7px;
}
.v-lcs-search-box:focus-within {
  border-color: var(--brand-border);
}
.v-lcs-search-icon {
  color: var(--text-3);
  flex-shrink: 0;
}
.v-lcs-search-input {
  flex: 1;
  min-width: 0;
  height: 100%;
  border: 0;
  background: transparent;
  color: var(--text-1);
  font-size: 12.5px;
  outline: none;
}
.v-lcs-search-clear {
  width: 22px;
  height: 22px;
  border: 0;
  border-radius: var(--radius-sm);
  background: transparent;
  color: var(--text-2);
  display: inline-flex;
  align-items: center;
  justify-content: center;
  flex-shrink: 0;
}
.v-lcs-scroll {
  max-height: 230px;
  overflow-y: auto;
  overscroll-behavior: contain;
  padding: 8px;
  display: flex;
  flex-direction: column;
  gap: 10px;
}
.v-lcs-group {
  display: flex;
  flex-direction: column;
  gap: 5px;
}
.v-lcs-group-title {
  font-size: 10px;
  font-weight: 700;
  letter-spacing: 0.06em;
  text-transform: uppercase;
  color: var(--text-3);
  padding: 0 2px;
}
.v-lcs-group-list {
  display: flex;
  flex-wrap: wrap;
  gap: 5px;
}
.v-lcs-opt {
  min-height: 30px;
  padding: 4px 10px;
  border-radius: var(--radius-control);
  border: 1px solid var(--border);
  background: var(--surface-2);
  color: var(--text-2);
  font-size: 12px;
  font-weight: 600;
  text-align: left;
  transition: transform var(--dur-press) ease-out;
}
.v-lcs-opt:not(:disabled):active {
  transform: scale(0.96);
}
.v-lcs-opt--all {
  width: 100%;
  text-align: left;
}
.v-lcs-opt.v-lcs-opt--all-active {
  background: var(--pill-all-bg);
  border-color: var(--pill-all-border);
  color: var(--pill-all-fg);
}
.v-lcs-opt.v-lcs-opt--active {
  background: var(--filter-active-bg);
  border-color: var(--filter-active-border);
  color: var(--filter-active-fg);
}
.v-lcs-empty {
  padding: 12px 4px;
  text-align: center;
}
@media (hover: hover) {
  .v-lcs-trigger:hover {
    background: var(--surface-hover);
  }
  .v-lcs-opt:hover {
    background: var(--surface-hover);
    color: var(--text-1);
  }
  .v-lcs-opt.v-lcs-opt--all-active:hover {
    background: var(--pill-all-bg);
    color: var(--pill-all-fg);
  }
  .v-lcs-opt.v-lcs-opt--active:hover {
    background: var(--filter-active-hover);
    color: var(--filter-active-fg);
  }
}
.v-lcs-expand-enter-active {
  transition:
    opacity 180ms var(--ease-snappy),
    transform 180ms var(--ease-snappy);
}
.v-lcs-expand-leave-active {
  transition:
    opacity 120ms ease-out,
    transform 120ms ease-out;
}
.v-lcs-expand-enter-from,
.v-lcs-expand-leave-to {
  opacity: 0;
  transform: translate3d(0, -6px, 0);
}
</style>

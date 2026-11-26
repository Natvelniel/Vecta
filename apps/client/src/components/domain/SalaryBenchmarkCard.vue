<script setup lang="ts">
import { computed, onMounted, ref, watch } from "vue";
import type { SalaryBenchmarkSummary } from "@/lib/bindings";
import { formatLocationClusterLabel } from "@/lib/locationClusters";
import { ipc } from "@/lib/ipc";
import VCard from "@/components/ui/VCard.vue";

const props = defineProps<{
  roleFilter: string | null;
}>();

const selectedCluster = ref<string | null>(null);
const loading = ref(false);
const data = ref<SalaryBenchmarkSummary | null>(null);

function formatCluster(slug: string): string {
  return formatLocationClusterLabel(slug);
}

function formatMillionIdr(val: number | null | undefined): string {
  if (!val || val <= 0) return "-";
  const jt = val / 1_000_000;
  return `Rp ${jt % 1 === 0 ? jt.toFixed(0) : jt.toFixed(1).replace(".", ",")} jt`;
}

async function fetchBenchmark() {
  loading.value = true;
  try {
    data.value = await ipc.getSalaryBenchmark(props.roleFilter, selectedCluster.value);
  } catch {
    data.value = null;
  } finally {
    loading.value = false;
  }
}

watch(
  () => props.roleFilter,
  () => {
    void fetchBenchmark();
  },
);

onMounted(() => {
  void fetchBenchmark();
});

const clusterOptions = computed(() => (data.value?.by_cluster ?? []).slice(0, 6));

const medianPct = computed(() => {
  const min = data.value?.p25_idr ?? data.value?.min_idr ?? 0;
  const max = data.value?.p75_idr ?? data.value?.max_idr ?? 0;
  const med = data.value?.median_idr ?? 0;
  if (max <= min) return 50;
  return Math.max(8, Math.min(92, Math.round(((med - min) / (max - min)) * 100)));
});
</script>

<template>
  <VCard style="margin-bottom: var(--space-5)">
    <div class="v-sb-head">
      <div>
        <h2 class="t-section">Benchmark Gaji Pasar (0 Token)</h2>
        <p class="t-meta" style="margin-top: 2px">
          Agregasi estimasi gaji bulanan dari lowongan lokal tersimpan{{ roleFilter ? ` untuk ${roleFilter}` : "" }}.
        </p>
      </div>
      <span v-if="data" class="t-meta t-num">{{ data.sample_count }} sampel bergaji</span>
    </div>

    <div v-if="clusterOptions.length > 0" class="v-sb-pills" role="group" aria-label="Filter wilayah benchmark gaji">
      <button
        type="button"
        class="v-sb-pill"
        :class="{ 'v-sb-pill--all': selectedCluster === null }"
        @click="selectedCluster = null; fetchBenchmark()"
      >
        Semua Wilayah
      </button>
      <button
        v-for="cl in clusterOptions"
        :key="cl.location_cluster"
        type="button"
        class="v-sb-pill"
        :class="{ 'v-sb-pill--manual': selectedCluster === cl.location_cluster }"
        @click="selectedCluster = cl.location_cluster; fetchBenchmark()"
      >
        {{ formatCluster(cl.location_cluster) }}
      </button>
    </div>

    <p v-if="loading" class="t-meta" style="margin-top: var(--space-3)">Menghitung distribusi gaji...</p>

    <p v-else-if="!data || data.sample_count === 0" class="t-secondary" style="margin-top: var(--space-3)">
      Belum ada data gaji tercantum pada sampel lowongan ini.
    </p>

    <div v-else class="v-sb-body">
      <div v-if="data.is_small_sample" class="v-sb-warn" role="status">
        Sampel masih kecil (&lt; 3 lowongan bergaji); scrape lebih banyak untuk akurasi pasar.
      </div>

      <div class="v-sb-stats">
        <div>
          <span class="t-meta">P25 (Bawah)</span>
          <strong class="v-sb-val t-num">{{ formatMillionIdr(data.p25_idr) }}</strong>
        </div>
        <div>
          <span class="t-meta">Median Pasar</span>
          <strong class="v-sb-val v-sb-val--med t-num">{{ formatMillionIdr(data.median_idr) }}</strong>
        </div>
        <div>
          <span class="t-meta">P75 (Atas)</span>
          <strong class="v-sb-val t-num">{{ formatMillionIdr(data.p75_idr) }}</strong>
        </div>
      </div>

      <div class="v-sb-bar" aria-label="Rentang gaji P25 sampai P75">
        <div class="v-sb-bar__fill" />
        <div class="v-sb-bar__marker" :style="{ left: `${medianPct}%` }" />
      </div>
    </div>
  </VCard>
</template>

<style scoped>
.v-sb-head {
  display: flex;
  align-items: baseline;
  justify-content: space-between;
  gap: var(--space-3);
  flex-wrap: wrap;
}
.v-sb-pills {
  display: flex;
  flex-wrap: wrap;
  gap: 6px;
  margin-top: var(--space-3);
}
.v-sb-pill {
  height: 28px;
  padding: 0 10px;
  border-radius: var(--radius-control);
  border: 1px solid var(--border);
  background: var(--surface-2);
  color: var(--text-2);
  font-size: 12px;
  font-weight: 600;
}
.v-sb-pill--all {
  background: var(--pill-all-bg);
  border-color: var(--pill-all-border);
  color: var(--pill-all-fg);
}
.v-sb-pill--manual {
  background: var(--filter-active-bg);
  border-color: var(--filter-active-border);
  color: var(--filter-active-fg);
}
.v-sb-body {
  margin-top: var(--space-4);
  display: flex;
  flex-direction: column;
  gap: var(--space-3);
}
.v-sb-warn {
  padding: 8px 12px;
  border-radius: var(--radius-control);
  border: 1px solid var(--warning-border);
  background: var(--warning-soft);
  color: var(--status-warning);
  font-size: 12.5px;
  font-weight: 500;
}
.v-sb-stats {
  display: grid;
  grid-template-columns: repeat(3, 1fr);
  gap: var(--space-3);
}
.v-sb-val {
  display: block;
  margin-top: 2px;
  font-size: 16px;
  color: var(--text-1);
}
.v-sb-val--med {
  color: var(--status-success);
}
.v-sb-bar {
  position: relative;
  height: 8px;
  border-radius: var(--radius-full);
  background: var(--surface-2);
  overflow: visible;
}
.v-sb-bar__fill {
  position: absolute;
  inset: 0;
  border-radius: var(--radius-full);
  background: linear-gradient(90deg, var(--brand-soft), var(--filter-active-tint));
}
.v-sb-bar__marker {
  position: absolute;
  top: -3px;
  width: 14px;
  height: 14px;
  transform: translateX(-50%);
  border-radius: 50%;
  background: var(--status-success);
  border: 2px solid var(--surface-1);
}
</style>

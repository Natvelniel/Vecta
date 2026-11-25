<script setup lang="ts">
import { onBeforeUnmount, onMounted, ref } from "vue";
import { useProfileStore } from "@/stores/profile";
import { useToast } from "@/composables/useToast";
import { formatFitType } from "@/lib/format";
import VButton from "@/components/ui/VButton.vue";
import VIcon from "@/components/ui/VIcon.vue";

const profileStore = useProfileStore();
const toast = useToast();

const open = ref(false);
const customRole = ref("");
const anchorRef = ref<HTMLElement | null>(null);

async function handleToggle(index: number) {
  const ok = await profileStore.toggleKeyword(index);
  const kw = profileStore.keywordsDraft[index];
  if (ok && kw) {
    toast.show(
      `Kata kunci ${kw.role_label} ${kw.is_active ? "diaktifkan" : "dinonaktifkan"} (0 token Gemini).`,
      { tone: "success" },
    );
  }
}

async function handleAddCustom() {
  const val = customRole.value.trim();
  if (!val) return;
  const ok = await profileStore.addCustomKeyword(val);
  if (ok) {
    customRole.value = "";
    toast.show(`Peran kustom ${val} ditambahkan.`, { tone: "success" });
  }
}

function onDocClick(e: MouseEvent) {
  if (open.value && anchorRef.value && !anchorRef.value.contains(e.target as Node)) {
    open.value = false;
  }
}

function onKeydown(e: KeyboardEvent) {
  if (open.value && e.key === "Escape") open.value = false;
}

onMounted(() => {
  document.addEventListener("mousedown", onDocClick);
  window.addEventListener("keydown", onKeydown);
});
onBeforeUnmount(() => {
  document.removeEventListener("mousedown", onDocClick);
  window.removeEventListener("keydown", onKeydown);
});
</script>

<template>
  <div ref="anchorRef" class="v-pop-anchor">
    <VButton
      variant="secondary"
      size="compact"
      style="height: 36px"
      :aria-expanded="String(open)"
      @click="open = !open"
    >
      <VIcon name="sliders" :size="14" />
      <span class="t-num">Kunci ({{ profileStore.activeKeywordsCount }})</span>
    </VButton>

    <div
      v-if="open"
      class="v-pop-panel"
      role="dialog"
      aria-label="Kelola kata kunci pencarian lowongan"
    >
      <div class="v-pop-head">
        <strong style="font-size: 14px">Kata Kunci Aktif</strong>
        <span class="t-meta">0 token Gemini</span>
      </div>

      <div v-if="profileStore.keywordsDraft.length === 0" class="t-meta" style="padding: 8px 0">
        Belum ada kata kunci. Tambahkan peran kustom di bawah atau lengkapi Profil Cerita.
      </div>

      <div v-else class="v-pop-list">
        <label
          v-for="(kw, idx) in profileStore.keywordsDraft"
          :key="kw.id ?? kw.role_label"
          class="v-pop-item"
        >
          <span class="v-pop-item__left">
            <input
              type="checkbox"
              :checked="kw.is_active"
              @change="handleToggle(idx)"
            />
            <span style="font-size: 13px; font-weight: 500">{{ kw.role_label }}</span>
          </span>
          <span class="t-meta">{{ formatFitType(kw.fit_type) }}</span>
        </label>
      </div>

      <form class="v-pop-add" @submit.prevent="handleAddCustom">
        <label for="pop-custom-role" class="sr-only">Tambah peran kustom</label>
        <input
          id="pop-custom-role"
          v-model="customRole"
          type="text"
          class="v-pop-input"
          placeholder="Tambah peran kustom..."
        />
        <VButton type="submit" variant="primary" size="compact" style="height: 36px">
          Tambah
        </VButton>
      </form>
    </div>
  </div>
</template>

<style scoped>
.v-pop-anchor {
  position: relative;
}
.v-pop-panel {
  position: absolute;
  top: calc(100% + 8px);
  right: 0;
  width: 316px;
  z-index: 60;
  background: var(--bg-elevated);
  border: 1px solid var(--border-default);
  border-radius: var(--radius-card);
  padding: var(--space-4);
  box-shadow: var(--shadow-overlay);
}
.v-pop-head {
  display: flex;
  align-items: center;
  justify-content: space-between;
  margin-bottom: var(--space-3);
}
.v-pop-list {
  display: flex;
  flex-direction: column;
  gap: var(--space-2);
  max-height: 220px;
  overflow-y: auto;
}
.v-pop-item {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: var(--space-2);
  padding: var(--space-2) 10px;
  border-radius: var(--radius-sm);
  background: var(--bg-primary);
  cursor: pointer;
}
.v-pop-item__left {
  display: flex;
  align-items: center;
  gap: var(--space-2);
}
.v-pop-add {
  display: flex;
  gap: var(--space-2);
  margin-top: var(--space-3);
}
.v-pop-input {
  flex: 1;
  min-width: 0;
  height: 36px;
  padding: 0 var(--space-3);
  border-radius: var(--radius-control);
  border: 1px solid var(--border-default);
  background: var(--bg-primary);
  color: var(--text-primary);
  font-size: 13px;
}
.v-pop-input:focus {
  border-color: var(--brand-hover);
}
@media (max-width: 1023px) {
  .v-pop-panel {
    left: 0;
    right: auto;
    width: 296px;
  }
}
</style>

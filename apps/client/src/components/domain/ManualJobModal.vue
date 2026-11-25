<script setup lang="ts">
import { ref } from "vue";
import VButton from "@/components/ui/VButton.vue";
import VDialog from "@/components/ui/VDialog.vue";
import VIcon from "@/components/ui/VIcon.vue";

defineProps<{
  open: boolean;
  submitting?: boolean;
}>();

const emit = defineEmits<{
  (e: "close"): void;
  (e: "submit", payload: { titleHint: string | null; url: string | null; rawText: string }): void;
}>();

const titleHint = ref("");
const url = ref("");
const rawText = ref("");
const localError = ref<string | null>(null);

function handleSubmit() {
  const cleanText = rawText.value.trim();
  if (cleanText.length < 30) {
    localError.value = "Teks deskripsi lowongan minimal 30 karakter.";
    return;
  }
  if (cleanText.length > 12000) {
    localError.value = "Teks deskripsi lowongan maksimal 12.000 karakter.";
    return;
  }
  localError.value = null;
  emit("submit", {
    titleHint: titleHint.value.trim() || null,
    url: url.value.trim() || null,
    rawText: cleanText,
  });
}

function resetFields() {
  titleHint.value = "";
  url.value = "";
  rawText.value = "";
  localError.value = null;
}

defineExpose({ resetFields });
</script>

<template>
  <VDialog :open="open" title="Tambah Lowongan Manual" @close="emit('close')">
    <form class="v-form-stack" @submit.prevent="handleSubmit">
      <div>
        <label for="mj-title" class="v-field-label">Judul Lowongan (opsional)</label>
        <input
          id="mj-title"
          v-model="titleHint"
          type="text"
          class="v-input"
          placeholder="Misal: Backend Engineer Rust"
        />
      </div>

      <div>
        <label for="mj-url" class="v-field-label">Tautan Lowongan (opsional)</label>
        <input
          id="mj-url"
          v-model="url"
          type="url"
          class="v-input"
          placeholder="https://..."
        />
      </div>

      <div>
        <div class="v-field-head">
          <label for="mj-desc" class="v-field-label" style="margin-bottom: 0">
            Teks Deskripsi Lowongan (minimal 30 karakter)
          </label>
          <span class="t-meta t-num">{{ rawText.trim().length }} / 12.000</span>
        </div>
        <textarea
          id="mj-desc"
          v-model="rawText"
          class="v-textarea"
          placeholder="Tempel deskripsi lowongan dari portal kerja atau pesan rekruter..."
        />
        <p v-if="localError" class="v-field-err" role="alert">{{ localError }}</p>
      </div>

      <div class="v-form-actions">
        <VButton variant="secondary" @click="emit('close')">Batal</VButton>
        <VButton type="submit" variant="primary" :loading="submitting">
          <VIcon v-if="!submitting" name="aiNode" :size="14" />
          <span>{{ submitting ? "Mengekstrak..." : "Ekstrak & Hitung Skor" }}</span>
        </VButton>
      </div>
    </form>
  </VDialog>
</template>

<style scoped>
.v-form-stack {
  display: flex;
  flex-direction: column;
  gap: var(--space-4);
}
.v-field-head {
  display: flex;
  align-items: baseline;
  justify-content: space-between;
  gap: var(--space-3);
  margin-bottom: var(--space-2);
}
.v-field-label {
  display: block;
  font-size: 13px;
  font-weight: 500;
  color: var(--text-primary);
  margin-bottom: var(--space-2);
}
.v-input,
.v-textarea {
  width: 100%;
  background: var(--bg-primary);
  color: var(--text-primary);
  border: 1px solid var(--border-default);
  border-radius: var(--radius-control);
  font-size: 14px;
}
.v-input {
  height: 44px;
  padding: 0 var(--space-4);
}
.v-textarea {
  min-height: 148px;
  padding: var(--space-3) var(--space-4);
  resize: vertical;
  line-height: 1.6;
}
.v-input:focus,
.v-textarea:focus {
  border-color: var(--brand-hover);
}
.v-field-err {
  margin: var(--space-2) 0 0;
  font-size: 12px;
  color: var(--status-error);
}
.v-form-actions {
  display: flex;
  justify-content: flex-end;
  gap: var(--space-3);
  margin-top: var(--space-2);
}
</style>

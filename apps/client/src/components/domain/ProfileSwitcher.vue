<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref } from "vue";
import { useRouter } from "vue-router";
import { useProfileStore } from "@/stores/profile";
import { useJobsStore } from "@/stores/jobs";
import { useCompetencyStore } from "@/stores/competency";
import { useRoadmapStore } from "@/stores/roadmap";
import { useToast } from "@/composables/useToast";
import { useBackOverlay } from "@/composables/useBackOverlay";
import { formatExperienceLevel } from "@/lib/format";
import VIcon from "@/components/ui/VIcon.vue";

const props = defineProps<{
  compact?: boolean;
}>();

const router = useRouter();
const profileStore = useProfileStore();
const jobsStore = useJobsStore();
const competencyStore = useCompetencyStore();
const roadmapStore = useRoadmapStore();
const toast = useToast();

const open = ref(false);
useBackOverlay(open, () => {
  open.value = false;
}, "profile-menu");

function handleEscapeKey(e: KeyboardEvent) {
  if (e.key === "Escape" && open.value) {
    open.value = false;
  }
}

onMounted(() => window.addEventListener("keydown", handleEscapeKey));
onBeforeUnmount(() => window.removeEventListener("keydown", handleEscapeKey));

const activeLabel = computed(
  () => profileStore.activeProfile?.profile_name || "Profil Utama",
);

function sanitizeProfileError(raw: string | null | undefined, fallback: string): string {
  if (!raw) return fallback;
  if (/https?:\/\/|sqlite|panic|stack/i.test(raw)) return fallback;
  return raw;
}

async function refreshWorkspaceForUser(uid: string) {
  await Promise.all([
    jobsStore.loadJobs(uid),
    jobsStore.loadWatchlists(uid),
    competencyStore.loadRolesAndMap(uid),
    roadmapStore.loadRoadmaps(uid),
    profileStore.loadMasterCv(uid),
  ]);
}

async function handleSwitch(profileId: string) {
  open.value = false;
  if (profileId === profileStore.userId) return;
  const ok = await profileStore.switchProfile(profileId);
  if (ok && profileStore.userId) {
    await refreshWorkspaceForUser(profileStore.userId);
    toast.show(`Beralih ke jalur profil ${profileStore.activeProfile?.profile_name ?? ""}.`, {
      tone: "success",
    });
  } else if (profileStore.error) {
    toast.show(sanitizeProfileError(profileStore.error, "Belum dapat beralih profil."), {
      tone: "info",
    });
  }
}

async function handleDelete(profileId: string, name: string) {
  const ok = await profileStore.deleteProfile(profileId);
  if (ok && profileStore.userId) {
    await refreshWorkspaceForUser(profileStore.userId);
    toast.show(`Profil "${name}" dihapus.`, { tone: "info" });
  } else if (profileStore.error) {
    toast.show(sanitizeProfileError(profileStore.error, "Profil tidak dapat dihapus saat ini."), {
      tone: "info",
    });
  }
}

function handleNewProfile() {
  open.value = false;
  profileStore.startNewProfileDraft();
  router.push("/onboarding");
}
</script>

<template>
  <div class="v-ps-wrap">
    <button
      type="button"
      class="v-ps-trigger"
      :class="{ 'v-ps-trigger--compact': compact }"
      :aria-expanded="open"
      aria-label="Pilih atau kelola profil jalur karier"
      @click="open = !open"
    >
      <span class="v-ps-dot" aria-hidden="true" />
      <span class="v-ps-name">{{ activeLabel }}</span>
      <span v-if="!compact && profileStore.profiles.length > 1" class="v-ps-count t-num">
        {{ profileStore.profiles.length }}
      </span>
      <VIcon name="chevronDown" :size="13" />
    </button>

    <Transition name="v-backdrop-fade">
      <div v-if="open" class="v-ps-backdrop" @click="open = false" />
    </Transition>

    <Transition :name="compact ? 'v-ps-center-pop' : 'v-dialog-pop'">
      <div
        v-if="open"
        class="v-ps-menu"
        :class="{ 'v-ps-menu--compact': compact }"
        role="menu"
        aria-label="Daftar jalur profil"
      >
      <div class="v-ps-menu-head t-meta">Jalur Profil Tersimpan</div>
      <div class="v-ps-list">
        <div
          v-for="p in profileStore.profiles"
          :key="p.id"
          class="v-ps-item"
          :class="{ 'v-ps-item--active': p.is_active }"
        >
          <button type="button" class="v-ps-item-btn" @click="handleSwitch(p.id)">
            <strong class="v-ps-item-title">{{ p.profile_name }}</strong>
            <span class="t-meta">
              {{ p.target_roles[0] || formatExperienceLevel(p.experience_level) }} · {{ p.skill_count }} skill
            </span>
          </button>
          <button
            v-if="profileStore.profiles.length > 1"
            type="button"
            class="v-ps-del"
            :aria-label="`Hapus profil ${p.profile_name}`"
            @click.stop="handleDelete(p.id, p.profile_name)"
          >
            <VIcon name="close" :size="12" />
          </button>
        </div>
      </div>

      <button type="button" class="v-ps-new" @click="handleNewProfile">
        <VIcon name="plus" :size="14" />
        <span>Buat Profil Jalur Baru</span>
      </button>
    </div>
    </Transition>
  </div>
</template>

<style scoped>
.v-ps-wrap {
  position: relative;
  min-width: 0;
  flex-shrink: 1;
}
.v-ps-trigger {
  width: 100%;
  height: 36px;
  padding: 0 10px;
  border-radius: var(--radius-control);
  border: 1px solid var(--border);
  background: var(--surface-2);
  color: var(--text-1);
  display: inline-flex;
  align-items: center;
  gap: 8px;
  font-size: 12.5px;
  font-weight: 600;
}
.v-ps-trigger--compact {
  width: auto;
  max-width: 146px;
  min-width: 0;
  height: 32px;
  padding: 0 10px;
  gap: 6px;
  font-size: 12px;
  justify-content: center;
}
@media (max-width: 390px) {
  .v-ps-trigger--compact {
    max-width: 132px;
    padding: 0 8px;
  }
}
.v-ps-dot {
  width: 7px;
  height: 7px;
  border-radius: 50%;
  background: var(--text-2);
  flex-shrink: 0;
}
.v-ps-name {
  flex: 1;
  min-width: 0;
  text-align: left;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.v-ps-count {
  font-size: 11px;
  padding: 1px 5px;
  border-radius: var(--radius-full);
  background: var(--brand-soft);
  color: var(--text-1);
}
.v-ps-backdrop {
  position: fixed;
  inset: 0;
  z-index: 48;
}
.v-ps-menu {
  position: absolute;
  top: calc(100% + 6px);
  left: 0;
  min-width: 230px;
  max-width: 280px;
  z-index: 55;
  padding: 8px;
  border-radius: var(--radius-card);
  border: 1px solid var(--border);
  background: var(--surface-1);
  box-shadow: var(--shadow-overlay), inset 0 1px 0 rgba(255, 255, 255, 0.06);
  display: flex;
  flex-direction: column;
  gap: 6px;
}
.v-ps-menu--compact {
  position: fixed;
  top: calc(54px + env(safe-area-inset-top, 0px));
  left: 50%;
  right: auto;
  width: min(320px, calc(100vw - 24px));
  max-width: min(320px, calc(100vw - 24px));
  transform: translate3d(-50%, 0, 0);
  transform-origin: top center;
}
.v-ps-center-pop-enter-active {
  transition: opacity 180ms cubic-bezier(0.22, 1, 0.36, 1), transform 180ms cubic-bezier(0.22, 1, 0.36, 1);
}
.v-ps-center-pop-leave-active {
  transition: opacity 130ms ease-out, transform 130ms ease-out;
}
.v-ps-center-pop-enter-from,
.v-ps-center-pop-leave-to {
  opacity: 0;
  transform: translate3d(-50%, 8px, 0);
}
.v-ps-menu-head {
  padding: 2px 6px;
  font-weight: 700;
}
.v-ps-list {
  display: flex;
  flex-direction: column;
  gap: 4px;
  max-height: 220px;
  overflow-y: auto;
  overscroll-behavior: contain;
}
.v-ps-item {
  display: flex;
  align-items: center;
  border-radius: var(--radius-control);
  border: 1px solid transparent;
  background: var(--bg-primary);
}
.v-ps-item--active {
  border-color: var(--brand-border);
  background: var(--brand-soft);
}
.v-ps-item-btn {
  flex: 1;
  min-width: 0;
  padding: 7px 8px;
  border: 0;
  background: transparent;
  text-align: left;
  display: flex;
  flex-direction: column;
  gap: 1px;
}
.v-ps-item-title {
  font-size: 12.5px;
  color: var(--text-1);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.v-ps-del {
  width: 28px;
  height: 28px;
  margin-right: 4px;
  border: 0;
  border-radius: var(--radius-sm);
  background: transparent;
  color: var(--text-3);
  display: inline-flex;
  align-items: center;
  justify-content: center;
}
.v-ps-new {
  width: 100%;
  height: 34px;
  padding: 0 8px;
  border-radius: var(--radius-control);
  border: 1px dashed var(--border-strong);
  background: transparent;
  color: var(--text-1);
  font-size: 12px;
  font-weight: 600;
  display: inline-flex;
  align-items: center;
  gap: 6px;
}
@media (hover: hover) {
  .v-ps-del:hover {
    color: var(--status-error);
    background: var(--error-soft);
  }
  .v-ps-new:hover {
    border-color: var(--brand);
  }
}
</style>

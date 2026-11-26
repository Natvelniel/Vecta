<script setup lang="ts">
import { computed, onMounted, ref } from "vue";
import { useRoute, useRouter } from "vue-router";
import { useProfileStore } from "@/stores/profile";
import { useSettingsStore } from "@/stores/settings";
import { useToast } from "@/composables/useToast";
import { useTheme } from "@/composables/useTheme";
import { registerApiKeyMissingHandler } from "@/lib/ipc";
import AppShell from "@/components/layout/AppShell.vue";
import VToastContainer from "@/components/ui/VToastContainer.vue";

const route = useRoute();
const router = useRouter();
const settingsStore = useSettingsStore();
const profileStore = useProfileStore();
const toast = useToast();

// Initialize persisted dual theme on boot
useTheme();

const bootstrapped = ref(false);

const isFirstRunChrome = computed(
  () => route.path === "/onboarding" && route.query.firstRun === "1",
);

onMounted(async () => {
  registerApiKeyMissingHandler(() => {
    toast.show("Kunci API Gemini belum terhubung. Hubungkan dulu di Pengaturan.", {
      tone: "warning",
      actionLabel: "Buka Pengaturan",
      onAction: () => router.push("/settings"),
    });
  });

  await Promise.all([
    settingsStore.loadApiKeyStatus(),
    settingsStore.loadOfflineEngineStatus(),
    profileStore.loadActiveProfile(),
  ]);
  bootstrapped.value = true;

  const skipped = sessionStorage.getItem("vecta:skipped_onboarding") === "1";
  const needsOnboarding = !profileStore.activeProfile && !skipped;

  if (needsOnboarding && route.path === "/dashboard") {
    await router.replace({ path: "/onboarding" });
  }
});
</script>

<template>
  <div v-if="!bootstrapped" class="v-boot-screen" role="status">
    <span class="t-secondary">Menyiapkan ruang kerja lokal Vecta...</span>
  </div>

  <div v-else-if="isFirstRunChrome" class="v-first-run-shell">
    <RouterView />
    <VToastContainer />
  </div>

  <AppShell v-else>
    <RouterView />
  </AppShell>
</template>

<style scoped>
.v-boot-screen {
  height: 100vh;
  width: 100%;
  display: flex;
  align-items: center;
  justify-content: center;
  background: var(--bg-primary);
}
.v-first-run-shell {
  min-height: 100vh;
  width: 100%;
  overflow-y: auto;
  background: var(--bg-primary);
}
</style>

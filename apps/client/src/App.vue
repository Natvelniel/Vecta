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

const DOCK_ORDER = ["/dashboard", "/onboarding", "/jobs", "/cv", "/competency"];
const routeTransitionName = ref<"v-route-forward" | "v-route-backward" | "v-route-fade">("v-route-fade");

const isFirstRunChrome = computed(
  () => route.path === "/onboarding" && route.query.firstRun === "1",
);

router.beforeEach((to, from) => {
  const fromIdx = DOCK_ORDER.indexOf(from.path);
  const toIdx = DOCK_ORDER.indexOf(to.path);
  if (fromIdx >= 0 && toIdx >= 0 && fromIdx !== toIdx) {
    routeTransitionName.value = toIdx > fromIdx ? "v-route-forward" : "v-route-backward";
  } else {
    routeTransitionName.value = "v-route-fade";
  }
});

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

  const skipped = sessionStorage.getItem("vecta:skipped_onboarding") === "1";
  const needsOnboarding = !profileStore.activeProfile && !skipped;

  if (needsOnboarding && route.path === "/dashboard") {
    await router.replace({ path: "/onboarding" });
  }
  bootstrapped.value = true;
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
    <RouterView v-slot="{ Component, route: viewRoute }">
      <Transition :name="routeTransitionName">
        <KeepAlive :max="6">
          <component :is="Component" :key="String(viewRoute.name ?? viewRoute.path)" />
        </KeepAlive>
      </Transition>
    </RouterView>
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

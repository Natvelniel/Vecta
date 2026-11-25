<script setup lang="ts">
import { computed } from "vue";
import { useRoute, useRouter } from "vue-router";
import { useJobsStore } from "@/stores/jobs";
import { useSettingsStore } from "@/stores/settings";
import VIcon, { type IconName } from "@/components/ui/VIcon.vue";
import VButton from "@/components/ui/VButton.vue";
import VToastContainer from "@/components/ui/VToastContainer.vue";

const route = useRoute();
const router = useRouter();
const settingsStore = useSettingsStore();
const jobsStore = useJobsStore();

const navItems: Array<{ to: string; label: string; short: string; icon: IconName }> = [
  { to: "/dashboard", label: "Dashboard", short: "Beranda", icon: "dashboard" },
  { to: "/onboarding", label: "Profil & CV", short: "Profil", icon: "story" },
  { to: "/jobs", label: "Lowongan", short: "Lowongan", icon: "jobs" },
  { to: "/cv", label: "Studio CV ATS", short: "CV ATS", icon: "sliders" },
  { to: "/competency", label: "Peta Kompetensi", short: "Peta", icon: "competency" },
  { to: "/roadmap", label: "Roadmap", short: "Roadmap", icon: "roadmap" },
];

const jobsBadge = computed(() => (jobsStore.jobs.length > 0 ? jobsStore.jobs.length : null));
const showKeyBanner = computed(
  () =>
    !settingsStore.apiKeyStatus.has_key &&
    !settingsStore.localAiStatus.is_ready &&
    route.path !== "/settings",
);
</script>

<template>
  <div class="v-shell">
    <!-- Mobile Top Bar (< 1024px) -->
    <header class="v-mobile-topbar">
      <RouterLink to="/dashboard" class="v-brand-link">
        <VIcon name="mark" :size="20" />
        <span class="v-brand-wordmark">Vecta</span>
      </RouterLink>
      <button
        type="button"
        class="v-mobile-settings"
        aria-label="Buka Pengaturan"
        @click="router.push('/settings')"
      >
        <VIcon name="settings" :size="18" />
      </button>
    </header>

    <!-- Desktop Subordinate Sidebar (240px, Section 7.1 & 8.9) -->
    <aside class="v-sidebar" aria-label="Navigasi utama">
      <div>
        <RouterLink to="/dashboard" class="v-brand-bar">
          <VIcon name="mark" :size="21" />
          <span class="v-brand-wordmark">Vecta</span>
        </RouterLink>

        <nav class="v-nav-group">
          <RouterLink
            v-for="item in navItems"
            :key="item.to"
            :to="item.to"
            class="v-nav-item"
            :aria-current="route.path === item.to ? 'page' : undefined"
          >
            <VIcon :name="item.icon" :size="17" />
            <span>{{ item.label }}</span>
            <span v-if="item.to === '/jobs' && jobsBadge !== null" class="v-nav-count t-num">
              {{ jobsBadge }}
            </span>
          </RouterLink>
        </nav>
      </div>

      <div class="v-sidebar-footer">
        <RouterLink
          to="/settings"
          class="v-nav-item"
          :aria-current="route.path === '/settings' ? 'page' : undefined"
        >
          <VIcon name="settings" :size="17" />
          <span>Pengaturan</span>
          <span class="v-nav-status">
            <span
              class="v-status-dot"
              :style="{
                background: settingsStore.apiKeyStatus.has_key
                  ? 'var(--status-success)'
                  : 'var(--status-warning)',
              }"
            />
            <span>{{ settingsStore.apiKeyStatus.has_key ? "Siap" : "Belum" }}</span>
          </span>
        </RouterLink>
      </div>
    </aside>

    <!-- Main Workspace -->
    <div class="v-workspace">
      <div v-if="showKeyBanner" class="v-key-banner" role="status">
        <span class="t-secondary" style="color: var(--status-warning)">
          Kunci API Gemini belum terhubung. Fitur ekstraksi cerita dan analisis AI memerlukan kunci lokal.
        </span>
        <VButton variant="secondary" size="compact" @click="router.push('/settings')">
          Hubungkan Kunci
        </VButton>
      </div>

      <main class="v-main">
        <slot />
      </main>
    </div>

    <!-- Mobile Bottom Navigation (< 1024px) -->
    <nav class="v-mobile-bottomnav" aria-label="Navigasi seluler">
      <RouterLink
        v-for="item in navItems"
        :key="item.to"
        :to="item.to"
        class="v-mobile-tab"
        :aria-current="route.path === item.to ? 'page' : undefined"
      >
        <VIcon :name="item.icon" :size="17" />
        <span>{{ item.short }}</span>
      </RouterLink>
    </nav>

    <VToastContainer />
  </div>
</template>

<style scoped>
.v-shell {
  display: flex;
  height: 100vh;
  width: 100%;
  overflow: hidden;
  background: var(--bg-primary);
}
.v-sidebar {
  width: 240px;
  flex-shrink: 0;
  height: 100vh;
  background: var(--bg-secondary);
  border-right: 1px solid var(--border-divider);
  display: flex;
  flex-direction: column;
  justify-content: space-between;
  padding: var(--space-4);
}
.v-brand-bar {
  height: 48px;
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 0 var(--space-3);
  margin-bottom: var(--space-4);
  text-decoration: none;
}
.v-brand-link {
  display: flex;
  align-items: center;
  gap: 8px;
  text-decoration: none;
}
.v-brand-wordmark {
  font-size: 18px;
  font-weight: 700;
  letter-spacing: -0.025em;
  color: var(--text-primary);
}
.v-nav-group {
  display: flex;
  flex-direction: column;
  gap: var(--space-1);
}
.v-nav-item {
  width: 100%;
  height: 40px;
  padding: 0 var(--space-3);
  border-radius: var(--radius-control);
  color: var(--text-secondary);
  font-size: 14px;
  font-weight: 500;
  text-decoration: none;
  display: flex;
  align-items: center;
  gap: var(--space-3);
  transition: background var(--dur-hover) var(--ease-state), color var(--dur-hover) var(--ease-state);
}
.v-nav-item:hover {
  background: var(--bg-hover);
  color: var(--text-primary);
}
.v-nav-item[aria-current="page"] {
  background: var(--brand-soft);
  color: var(--text-primary);
}
.v-nav-item[aria-current="page"] :deep(svg) {
  color: var(--brand-hover);
}
.v-nav-count {
  margin-left: auto;
  font-size: 12px;
  color: var(--text-secondary);
}
.v-nav-status {
  margin-left: auto;
  display: inline-flex;
  align-items: center;
  gap: 6px;
  font-size: 12px;
  color: var(--text-secondary);
}
.v-status-dot {
  width: 6px;
  height: 6px;
  border-radius: var(--radius-full);
}
.v-sidebar-footer {
  border-top: 1px solid var(--border-divider);
  padding-top: var(--space-3);
}
.v-workspace {
  flex: 1;
  min-width: 0;
  height: 100vh;
  display: flex;
  flex-direction: column;
  overflow: hidden;
  background: var(--bg-primary);
}
.v-key-banner {
  flex-shrink: 0;
  padding: var(--space-3) var(--space-8);
  background: var(--warning-soft);
  border-bottom: 1px solid var(--warning-border);
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: var(--space-3);
  flex-wrap: wrap;
}
.v-main {
  flex: 1;
  min-height: 0;
  overflow-y: auto;
  display: flex;
  flex-direction: column;
}
.v-mobile-topbar,
.v-mobile-bottomnav {
  display: none;
}

@media (max-width: 1023px) {
  .v-shell {
    flex-direction: column;
  }
  .v-sidebar {
    display: none;
  }
  .v-mobile-topbar {
    display: flex;
    align-items: center;
    justify-content: space-between;
    min-height: 54px;
    padding: env(safe-area-inset-top, 0px) var(--space-4) 0;
    flex-shrink: 0;
    background: var(--bg-secondary);
    border-bottom: 1px solid var(--border-divider);
  }
  .v-mobile-settings {
    width: 40px;
    height: 40px;
    border-radius: var(--radius-control);
    border: 0;
    background: transparent;
    color: var(--text-secondary);
    display: inline-flex;
    align-items: center;
    justify-content: center;
  }
  .v-mobile-settings:hover {
    background: var(--bg-hover);
    color: var(--text-primary);
  }
  .v-workspace {
    height: auto;
    flex: 1;
    min-height: 0;
  }
  .v-key-banner {
    padding: var(--space-3) var(--space-4);
  }
  .v-mobile-bottomnav {
    display: grid;
    grid-template-columns: repeat(6, 1fr);
    min-height: 62px;
    padding-bottom: env(safe-area-inset-bottom, 0px);
    flex-shrink: 0;
    background: var(--bg-secondary);
    border-top: 1px solid var(--border-divider);
  }
  .v-mobile-tab {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 4px;
    min-height: 44px;
    text-decoration: none;
    color: var(--text-secondary);
    font-size: 12px;
    font-weight: 500;
  }
  .v-mobile-tab[aria-current="page"] {
    color: var(--brand-hover);
    font-weight: 600;
  }
}
</style>

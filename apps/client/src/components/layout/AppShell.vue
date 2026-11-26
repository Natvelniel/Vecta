<script setup lang="ts">
import { computed, nextTick, onMounted, ref, watch } from "vue";
import { useRoute, useRouter } from "vue-router";
import { useJobsStore } from "@/stores/jobs";
import { useSettingsStore } from "@/stores/settings";
import { useTheme } from "@/composables/useTheme";
import ProfileSwitcher from "@/components/domain/ProfileSwitcher.vue";
import VIcon, { type IconName } from "@/components/ui/VIcon.vue";
import VToastContainer from "@/components/ui/VToastContainer.vue";

const route = useRoute();
const router = useRouter();
const settingsStore = useSettingsStore();
const jobsStore = useJobsStore();
const { theme, toggleTheme } = useTheme();

const desktopNavItems: Array<{ to: string; label: string; icon: IconName }> = [
  { to: "/dashboard", label: "Dashboard", icon: "dashboard" },
  { to: "/onboarding", label: "Profil & CV", icon: "story" },
  { to: "/jobs", label: "Lowongan", icon: "jobs" },
  { to: "/cv", label: "Studio CV ATS", icon: "cv" },
  { to: "/competency", label: "Peta Kompetensi", icon: "competency" },
  { to: "/roadmap", label: "Roadmap Belajar", icon: "roadmap" },
];

// 5 primary balanced tabs for Mobile Floating Liquid Glass Dock
const mobileDockItems: Array<{ to: string; short: string; icon: IconName }> = [
  { to: "/dashboard", short: "Beranda", icon: "dashboard" },
  { to: "/onboarding", short: "Profil", icon: "story" },
  { to: "/jobs", short: "Lowongan", icon: "jobs" },
  { to: "/cv", short: "CV ATS", icon: "cv" },
  { to: "/competency", short: "Peta", icon: "competency" },
];

const jobsBadge = computed(() => (jobsStore.jobs.length > 0 ? jobsStore.jobs.length : null));

const activeDockIndex = computed(() =>
  mobileDockItems.findIndex((item) => route.path === item.to),
);

const bouncePath = ref<string | null>(null);
watch(
  () => route.path,
  (nextPath) => {
    bouncePath.value = nextPath;
    setTimeout(() => {
      if (bouncePath.value === nextPath) bouncePath.value = null;
    }, 360);
    updateDesktopIndicator();
  },
);

const sidebarNavRef = ref<HTMLElement | null>(null);
const indicatorTop = ref(0);
const indicatorHeight = ref(0);
const indicatorVisible = ref(false);

async function updateDesktopIndicator() {
  await nextTick();
  const container = sidebarNavRef.value;
  if (!container) return;
  const activeEl = container.querySelector<HTMLElement>('[aria-current="page"]');
  if (activeEl) {
    indicatorTop.value = activeEl.offsetTop;
    indicatorHeight.value = activeEl.offsetHeight;
    indicatorVisible.value = true;
  } else {
    indicatorVisible.value = false;
  }
}

onMounted(() => {
  updateDesktopIndicator();
});
</script>

<template>
  <div class="v-shell">
    <!-- Subtle ambient gradient backdrop behind translucent Liquid Glass surfaces -->
    <div class="v-ambient" aria-hidden="true" />

    <!-- Mobile Top Micro-Bar (< 1024px) -->
    <header class="v-mobile-topbar liquid-glass">
      <RouterLink to="/dashboard" class="v-brand-link">
        <VIcon name="mark" :size="20" />
        <span class="v-brand-wordmark">Vecta</span>
      </RouterLink>
      <div class="v-mobile-actions">
        <ProfileSwitcher compact />
        <button
          type="button"
          class="v-icon-btn"
          :aria-label="route.path === '/roadmap' ? 'Roadmap Aktif' : 'Buka Roadmap Belajar'"
          title="Roadmap Belajar"
          @click="router.push('/roadmap')"
        >
          <VIcon name="roadmap" :size="17" />
        </button>
        <button
          type="button"
          class="v-icon-btn"
          :aria-label="theme === 'dark' ? 'Ganti ke Tema Putih Swiss' : 'Ganti ke Tema Hitam Obsidian'"
          :title="theme === 'dark' ? 'Tema Putih' : 'Tema Hitam'"
          @click="toggleTheme"
        >
          <VIcon :name="theme === 'dark' ? 'sun' : 'moon'" :size="17" />
        </button>
        <button
          type="button"
          class="v-icon-btn"
          aria-label="Buka Pengaturan"
          title="Pengaturan"
          @click="router.push('/settings')"
        >
          <VIcon name="settings" :size="17" />
        </button>
      </div>
    </header>

    <!-- Desktop Subordinate Sidebar (240px, with Glassy Vertical Sliding Capsule) -->
    <aside class="v-sidebar" aria-label="Navigasi utama">
      <div>
        <RouterLink to="/dashboard" class="v-brand-bar">
          <VIcon name="mark" :size="21" />
          <span class="v-brand-wordmark">Vecta</span>
        </RouterLink>

        <div style="padding: 0 12px 12px">
          <ProfileSwitcher />
        </div>

        <nav ref="sidebarNavRef" class="v-nav-group">
          <div
            v-if="indicatorVisible"
            class="v-sidebar-capsule"
            aria-hidden="true"
            :style="{
              transform: `translate3d(0, ${indicatorTop}px, 0)`,
              height: `${indicatorHeight}px`,
            }"
          />
          <RouterLink
            v-for="item in desktopNavItems"
            :key="item.to"
            :to="item.to"
            class="v-nav-item"
            :aria-current="route.path === item.to ? 'page' : undefined"
          >
            <span :class="{ 'animate-nav-bounce': bouncePath === item.to }" class="v-nav-icon">
              <VIcon :name="item.icon" :size="17" />
            </span>
            <span>{{ item.label }}</span>
            <span v-if="item.to === '/jobs' && jobsBadge !== null" class="v-nav-count t-num">
              {{ jobsBadge }}
            </span>
          </RouterLink>
        </nav>
      </div>

      <div class="v-sidebar-footer">
        <button
          type="button"
          class="v-theme-toggle"
          @click="toggleTheme"
        >
          <VIcon :name="theme === 'dark' ? 'sun' : 'moon'" :size="16" />
          <span>{{ theme === 'dark' ? 'Tema Putih Swiss' : 'Tema Hitam Obsidian' }}</span>
        </button>

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
            <span>{{ settingsStore.apiKeyStatus.has_key ? "Siap" : "Lokal" }}</span>
          </span>
        </RouterLink>
      </div>
    </aside>

    <!-- Main Workspace -->
    <div class="v-workspace">
      <main class="v-main">
        <slot />
      </main>
    </div>

    <!-- Floating Mobile Liquid Glass Bottom Dock (< 1024px) with Glassy Sliding Capsule -->
    <div class="v-dock-wrap">
      <nav class="v-mobile-dock liquid-glass" aria-label="Navigasi seluler">
        <div
          v-if="activeDockIndex >= 0"
          class="v-dock-capsule"
          aria-hidden="true"
          :style="{
            transform: `translate3d(${activeDockIndex * 100}%, 0, 0)`,
          }"
        />
        <RouterLink
          v-for="item in mobileDockItems"
          :key="item.to"
          :to="item.to"
          class="v-mobile-tab"
          :aria-current="route.path === item.to ? 'page' : undefined"
        >
          <span :class="{ 'animate-nav-bounce': bouncePath === item.to }" class="v-dock-icon">
            <VIcon :name="item.icon" :size="20" />
          </span>
          <span class="v-dock-label">{{ item.short }}</span>
        </RouterLink>
      </nav>
    </div>

    <VToastContainer />
  </div>
</template>

<style scoped>
.v-shell {
  position: relative;
  display: flex;
  min-height: 100vh;
  min-height: 100dvh;
  width: 100%;
  max-width: 100%;
  overflow-x: clip;
  background: var(--bg);
}

.v-ambient {
  position: fixed;
  inset: 0;
  pointer-events: none;
  z-index: 0;
  background:
    radial-gradient(620px 360px at 14% 8%, var(--ambient-glow-1), transparent 70%),
    radial-gradient(540px 340px at 86% 88%, var(--ambient-glow-2), transparent 72%);
}

.v-sidebar {
  position: relative;
  z-index: 2;
  width: 242px;
  flex-shrink: 0;
  background: var(--surface-1);
  border-right: 1px solid var(--border);
  padding: var(--space-5) var(--space-4);
  display: flex;
  flex-direction: column;
  justify-content: space-between;
}

.v-brand-bar,
.v-brand-link {
  display: inline-flex;
  align-items: center;
  gap: 10px;
  text-decoration: none;
}
.v-brand-bar {
  padding: var(--space-2) var(--space-3) var(--space-5);
}
.v-brand-wordmark {
  font-size: 16px;
  font-weight: 700;
  letter-spacing: -0.025em;
  color: var(--text-1);
}

.v-nav-group {
  position: relative;
  display: flex;
  flex-direction: column;
  gap: 3px;
}

.v-sidebar-capsule {
  position: absolute;
  left: 0;
  right: 0;
  top: 0;
  border-radius: var(--radius-control);
  background: var(--glass-pill);
  border: 1px solid var(--glass-pill-border);
  box-shadow: inset 0 1px 0 var(--glass-specular);
  transition:
    transform 300ms var(--ease-spring),
    height 200ms var(--ease-spring);
  pointer-events: none;
  z-index: 0;
}

.v-nav-item {
  position: relative;
  z-index: 1;
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 9px 12px;
  border-radius: var(--radius-control);
  color: var(--text-2);
  text-decoration: none;
  font-size: 13.5px;
  font-weight: 500;
  transition: color 150ms ease, background-color 150ms ease;
}
.v-nav-icon {
  display: inline-flex;
  align-items: center;
  justify-content: center;
}
.v-nav-item:hover {
  color: var(--text-1);
}
.v-nav-item[aria-current="page"] {
  color: var(--text-1);
  font-weight: 600;
}

.v-nav-count {
  margin-left: auto;
  font-size: 11px;
  font-weight: 700;
  color: var(--text-2);
  background: var(--surface-2);
  padding: 2px 7px;
  border-radius: var(--radius-full);
}

.v-nav-status {
  margin-left: auto;
  display: inline-flex;
  align-items: center;
  gap: 5px;
  font-size: 11px;
  color: var(--text-2);
}
.v-status-dot {
  width: 6px;
  height: 6px;
  border-radius: var(--radius-full);
}

.v-sidebar-footer {
  border-top: 1px solid var(--border);
  padding-top: var(--space-3);
  display: flex;
  flex-direction: column;
  gap: 4px;
}

.v-theme-toggle {
  display: flex;
  align-items: center;
  gap: 10px;
  width: 100%;
  padding: 8px 12px;
  border-radius: var(--radius-control);
  border: 1px solid var(--border);
  background: var(--surface-2);
  color: var(--text-2);
  font-size: 12.5px;
  font-weight: 500;
  transition: background-color 150ms ease, color 150ms ease, transform 110ms ease;
}
.v-theme-toggle:hover {
  background: var(--surface-hover);
  color: var(--text-1);
}
.v-theme-toggle:active {
  transform: scale(0.98);
}

.v-workspace {
  position: relative;
  z-index: 1;
  flex: 1;
  min-width: 0;
  width: 100%;
  height: 100vh;
  height: 100dvh;
  display: flex;
  flex-direction: column;
  overflow: hidden;
  background: transparent;
}

.v-main {
  flex: 1;
  min-width: 0;
  min-height: 0;
  width: 100%;
  overflow-y: auto;
  overflow-x: hidden;
  display: flex;
  flex-direction: column;
}

.v-mobile-topbar,
.v-dock-wrap {
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
    position: sticky;
    top: 0;
    z-index: 30;
    display: flex;
    align-items: center;
    justify-content: space-between;
    min-height: 48px;
    padding: env(safe-area-inset-top, 0px) var(--space-3) 0 var(--space-4);
    border-top: 0;
    border-left: 0;
    border-right: 0;
    border-radius: 0;
    flex-shrink: 0;
  }

  .v-mobile-actions {
    display: inline-flex;
    align-items: center;
    gap: 2px;
  }

  .v-icon-btn {
    width: 38px;
    height: 38px;
    border-radius: var(--radius-control);
    border: 0;
    background: transparent;
    color: var(--text-2);
    display: inline-flex;
    align-items: center;
    justify-content: center;
    transition: background-color 140ms ease, color 140ms ease, transform 100ms ease;
  }
  .v-icon-btn:hover {
    background: var(--surface-hover);
    color: var(--text-1);
  }
  .v-icon-btn:active {
    transform: scale(0.92);
  }

  .v-workspace {
    height: auto;
    flex: 1;
    min-height: 0;
  }

  .v-main {
    padding-bottom: calc(86px + env(safe-area-inset-bottom, 0px));
  }

  .v-dock-wrap {
    position: fixed;
    left: 12px;
    right: 12px;
    bottom: calc(10px + env(safe-area-inset-bottom, 0px));
    z-index: 40;
    display: flex;
    justify-content: center;
    pointer-events: none;
  }

  .v-mobile-dock {
    position: relative;
    pointer-events: auto;
    width: 100%;
    max-width: 460px;
    display: grid;
    grid-template-columns: repeat(5, minmax(0, 1fr));
    padding: 4px;
    border-radius: 24px;
  }

  .v-dock-capsule {
    position: absolute;
    top: 4px;
    bottom: 4px;
    left: 4px;
    width: calc((100% - 8px) / 5);
    border-radius: 20px;
    background: var(--glass-pill);
    border: 1px solid var(--glass-pill-border);
    box-shadow: inset 0 1px 0 var(--glass-specular);
    transition: transform 320ms var(--ease-spring);
    pointer-events: none;
    z-index: 0;
  }

  .v-mobile-tab {
    position: relative;
    z-index: 1;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 2px;
    min-height: 50px;
    min-width: 0;
    text-decoration: none;
    color: var(--text-2);
    transition: color 160ms ease, transform 100ms ease;
    -webkit-tap-highlight-color: transparent;
  }
  .v-mobile-tab:active {
    transform: scale(0.94);
  }
  .v-mobile-tab[aria-current="page"] {
    color: var(--text-1);
    font-weight: 600;
  }

  .v-dock-icon {
    display: inline-flex;
    align-items: center;
    justify-content: center;
  }

  .v-dock-label {
    font-size: 10.5px;
    line-height: 1.15;
    white-space: nowrap;
    letter-spacing: -0.01em;
  }
}
</style>

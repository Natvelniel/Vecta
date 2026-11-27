<script setup lang="ts">
import { computed, nextTick, onMounted, ref, watch } from "vue";
import { useRoute, useRouter } from "vue-router";
import { useJobsStore } from "@/stores/jobs";
import { useSettingsStore } from "@/stores/settings";
import { useTheme } from "@/composables/useTheme";
import { registerMainScroller } from "@/composables/useRouteScroll";
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

const lastDockIndex = ref(Math.max(0, activeDockIndex.value));
const capsuleGlide = ref(true);
const mainScrollRef = ref<HTMLElement | null>(null);
registerMainScroller(mainScrollRef);

function scrollContainerToTop(el: HTMLElement | null) {
  if (!el) return;
  const reduced =
    typeof window !== "undefined" &&
    window.matchMedia?.("(prefers-reduced-motion: reduce)").matches;
  if (typeof el.scrollTo === "function") {
    el.scrollTo({ top: 0, behavior: reduced ? "auto" : "smooth" });
  } else {
    el.scrollTop = 0;
  }
}

function handleDockTabClick(to: string, event: MouseEvent) {
  if (route.path !== to) return;
  if (route.path === "/jobs" && route.query.detail) {
    event.preventDefault();
    void router.replace({ path: "/jobs", query: { ...route.query, detail: undefined } });
    return;
  }
  event.preventDefault();
  scrollContainerToTop(mainScrollRef.value);
  const innerList = mainScrollRef.value?.querySelector<HTMLElement>(".v-jobs-list") ?? null;
  scrollContainerToTop(innerList);
}

watch(
  activeDockIndex,
  (nextIdx, prevIdx) => {
    if (nextIdx >= 0) {
      capsuleGlide.value = prevIdx !== undefined && prevIdx >= 0;
      lastDockIndex.value = nextIdx;
    } else {
      capsuleGlide.value = false;
    }
  },
  { immediate: true },
);

watch(
  () => route.path,
  () => {
    void updateDesktopIndicator();
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
    <!-- Mobile Top Micro-Bar (< 1024px): Symmetric 3-Zone Architectural Bar -->
    <header class="v-mobile-topbar">
      <RouterLink to="/dashboard" class="v-brand-link">
        <VIcon name="mark" :size="16" />
        <span class="v-brand-wordmark">Vecta</span>
      </RouterLink>
      <div class="v-mobile-center">
        <ProfileSwitcher compact />
      </div>
      <div class="v-mobile-actions">
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
          <Transition name="v-theme-icon-swap" mode="out-in">
            <VIcon :key="theme" :name="theme === 'dark' ? 'sun' : 'moon'" :size="17" />
          </Transition>
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
          <VIcon name="mark" :size="18" />
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
            <span class="v-nav-icon">
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
      <main ref="mainScrollRef" class="v-main">
        <slot />
      </main>
    </div>

    <!-- Floating Mobile Liquid Glass Bottom Dock (< 1024px) with Glassy Sliding Capsule -->
    <div class="v-dock-wrap">
      <nav class="v-mobile-dock liquid-glass" aria-label="Navigasi seluler">
        <div
          class="v-dock-capsule"
          :class="{
            'v-dock-capsule--hidden': activeDockIndex < 0,
            'v-dock-capsule--instant': !capsuleGlide,
          }"
          aria-hidden="true"
          :style="{
            transform: `translate3d(${lastDockIndex * 100}%, 0, 0)`,
          }"
        />
        <RouterLink
          v-for="item in mobileDockItems"
          :key="item.to"
          :to="item.to"
          class="v-mobile-tab"
          :aria-current="route.path === item.to ? 'page' : undefined"
          @click="handleDockTabClick(item.to, $event)"
        >
          <span class="v-dock-icon">
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
  color: var(--text-1);
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
    transform 380ms var(--ease-glide),
    height 260ms var(--ease-luxury);
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
  transition:
    background-color 140ms ease-out,
    color 140ms ease-out,
    transform var(--dur-press) ease-out;
}
.v-theme-toggle:not(:disabled):active {
  transform: scale(0.96);
}

@media (hover: hover) {
  .v-nav-item:hover {
    color: var(--text-1);
  }
  .v-theme-toggle:hover {
    background: var(--surface-hover);
    color: var(--text-1);
  }
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
  position: relative;
  flex: 1;
  min-width: 0;
  min-height: 0;
  width: 100%;
  overflow-y: auto;
  overflow-x: hidden;
  touch-action: pan-y;
  overscroll-behavior-x: none;
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
    display: grid;
    grid-template-columns: auto minmax(0, 1fr) auto;
    align-items: center;
    gap: 6px;
    min-height: calc(50px + env(safe-area-inset-top, 0px));
    padding: env(safe-area-inset-top, 0px) 8px 0 12px;
    background: var(--surface-1);
    border-bottom: 1px solid var(--border);
    box-shadow: var(--shadow-topbar);
    flex-shrink: 0;
  }

  .v-brand-link {
    gap: 6px;
    flex-shrink: 0;
    min-width: 0;
  }
  .v-brand-wordmark {
    font-size: 14.5px;
    white-space: nowrap;
  }

  .v-mobile-center {
    display: flex;
    justify-content: center;
    align-items: center;
    min-width: 0;
  }

  .v-mobile-actions {
    display: inline-flex;
    align-items: center;
    justify-content: flex-end;
    gap: 0;
    min-width: 0;
    flex-shrink: 0;
  }

  .v-icon-btn {
    width: 34px;
    min-width: 34px;
    min-height: 44px;
    height: 44px;
    flex-shrink: 0;
    border-radius: var(--radius-control);
    border: 0;
    background: transparent;
    color: var(--text-2);
    display: inline-flex;
    align-items: center;
    justify-content: center;
    transition:
      background-color 140ms ease-out,
      color 140ms ease-out,
      transform var(--dur-press) ease-out;
  }
  @media (hover: hover) {
    .v-icon-btn:hover {
      background: var(--surface-hover);
      color: var(--text-1);
    }
  }
  .v-icon-btn:not(:disabled):active {
    transform: scale(0.96);
  }

  .v-workspace {
    height: auto;
    flex: 1;
    min-height: 0;
  }

  .v-main {
    padding-bottom: calc(96px + env(safe-area-inset-bottom, 0px));
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
    transition:
      transform var(--dur-pill-slide) var(--ease-glide),
      opacity 220ms ease-out;
    will-change: transform, opacity;
    pointer-events: none;
    z-index: 0;
  }
  .v-dock-capsule--hidden {
    opacity: 0;
  }
  .v-dock-capsule--instant {
    transition: opacity 220ms ease-out;
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
    transition: color 240ms var(--ease-luxury), transform var(--dur-press) var(--ease-luxury);
    -webkit-tap-highlight-color: transparent;
  }
  .v-mobile-tab:active {
    transform: scale(0.95);
  }
  .v-mobile-tab[aria-current="page"] {
    color: var(--text-1);
    font-weight: 600;
  }

  .v-dock-icon {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    transition: transform 320ms var(--ease-luxury);
  }
  .v-mobile-tab[aria-current="page"] .v-dock-icon {
    transform: translateY(-1.5px) scale(1.08);
  }

  .v-dock-label {
    font-size: 10.5px;
    line-height: 1.15;
    white-space: nowrap;
    letter-spacing: -0.01em;
  }
}
</style>

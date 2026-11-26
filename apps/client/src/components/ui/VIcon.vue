<script setup lang="ts">
export type IconName =
  | "mark"
  | "dashboard"
  | "story"
  | "jobs"
  | "competency"
  | "roadmap"
  | "settings"
  | "bookmark"
  | "bookmarkFill"
  | "plus"
  | "refresh"
  | "external"
  | "close"
  | "check"
  | "back"
  | "sliders"
  | "search"
  | "sun"
  | "moon"
  | "mapPin"
  | "sparkle"
  | "download"
  | "upload"
  | "trash"
  | "chevronDown"
  | "chevronRight"
  | "moreVertical"
  | "cv"
  | "aiNode";

withDefaults(
  defineProps<{
    name: IconName;
    size?: number;
  }>(),
  { size: 16 },
);
</script>

<template>
  <svg
    v-if="name === 'mark'"
    :width="Math.round(size * 1.65)"
    :height="Math.round(size * 0.85)"
    viewBox="0 0 245 120"
    fill="none"
    aria-hidden="true"
  >
    <path
      d="M0 0H58C74 0 83 8 92 20L157 104L146 115C141 119 135 119 126 119H106C97 119 90 115 84 107L0 0Z"
      fill="currentColor"
    />
    <path
      d="M204 0H244L207 43C199 52 191 55 180 55H143L180 12C187 4 194 0 204 0Z"
      fill="currentColor"
    />
    <path
      d="M147 67H189L166.5 92L147 67Z"
      fill="currentColor"
    />
  </svg>

  <svg
    v-else-if="name === 'dashboard'"
    :width="size"
    :height="size"
    viewBox="0 0 24 24"
    fill="none"
    stroke="currentColor"
    stroke-width="1.75"
    stroke-linecap="round"
    stroke-linejoin="round"
    aria-hidden="true"
  >
    <rect x="3" y="3" width="8" height="8" rx="1.5" />
    <rect x="13" y="3" width="8" height="5" rx="1.5" />
    <rect x="13" y="10" width="8" height="11" rx="1.5" />
    <rect x="3" y="13" width="8" height="8" rx="1.5" />
  </svg>

  <svg
    v-else-if="name === 'story'"
    :width="size"
    :height="size"
    viewBox="0 0 24 24"
    fill="none"
    stroke="currentColor"
    stroke-width="1.75"
    stroke-linecap="round"
    stroke-linejoin="round"
    aria-hidden="true"
  >
    <path d="M4 19.5V5.5A1.5 1.5 0 0 1 5.5 4H14l6 6v9.5a1.5 1.5 0 0 1-1.5 1.5h-13A1.5 1.5 0 0 1 4 19.5Z" />
    <path d="M14 4v6h6" />
    <path d="M8 13h8" />
    <path d="M8 17h5" />
  </svg>

  <svg
    v-else-if="name === 'jobs'"
    :width="size"
    :height="size"
    viewBox="0 0 24 24"
    fill="none"
    stroke="currentColor"
    stroke-width="1.75"
    stroke-linecap="round"
    stroke-linejoin="round"
    aria-hidden="true"
  >
    <rect x="3" y="7" width="18" height="13" rx="2" />
    <path d="M8 7V5a2 2 0 0 1 2-2h4a2 2 0 0 1 2 2v2" />
    <path d="M3 12h18" />
  </svg>

  <svg
    v-else-if="name === 'competency'"
    :width="size"
    :height="size"
    viewBox="0 0 24 24"
    fill="none"
    stroke="currentColor"
    stroke-width="1.75"
    stroke-linecap="round"
    stroke-linejoin="round"
    aria-hidden="true"
  >
    <path d="M4 19h16" />
    <path d="M6 15V9" />
    <path d="M12 15V5" />
    <path d="M18 15v-3" />
  </svg>

  <svg
    v-else-if="name === 'roadmap'"
    :width="size"
    :height="size"
    viewBox="0 0 24 24"
    fill="none"
    stroke="currentColor"
    stroke-width="1.75"
    stroke-linecap="round"
    stroke-linejoin="round"
    aria-hidden="true"
  >
    <circle cx="6" cy="6" r="2.5" />
    <circle cx="18" cy="18" r="2.5" />
    <path d="M6 8.5V12a4 4 0 0 0 4 4h5.5" />
  </svg>

  <svg
    v-else-if="name === 'settings'"
    :width="size"
    :height="size"
    viewBox="0 0 24 24"
    fill="none"
    stroke="currentColor"
    stroke-width="1.75"
    stroke-linecap="round"
    stroke-linejoin="round"
    aria-hidden="true"
  >
    <circle cx="12" cy="12" r="3" />
    <path d="M19.4 15a1.65 1.65 0 0 0 .33 1.82l.06.06a2 2 0 1 1-2.83 2.83l-.06-.06a1.65 1.65 0 0 0-1.82-.33 1.65 1.65 0 0 0-1 1.51V21a2 2 0 1 1-4 0v-.09a1.65 1.65 0 0 0-1-1.51 1.65 1.65 0 0 0-1.82.33l-.06.06a2 2 0 1 1-2.83-2.83l.06-.06a1.65 1.65 0 0 0 .33-1.82 1.65 1.65 0 0 0-1.51-1H3a2 2 0 1 1 0-4h.09a1.65 1.65 0 0 0 1.51-1 1.65 1.65 0 0 0-.33-1.82l-.06-.06a2 2 0 1 1 2.83-2.83l.06.06a1.65 1.65 0 0 0 1.82.33h.08a1.65 1.65 0 0 0 1-1.51V3a2 2 0 1 1 4 0v.09a1.65 1.65 0 0 0 1 1.51 1.65 1.65 0 0 0 1.82-.33l.06-.06a2 2 0 1 1 2.83 2.83l-.06.06a1.65 1.65 0 0 0-.33 1.82v.08a1.65 1.65 0 0 0 1.51 1H21a2 2 0 1 1 0 4h-.09a1.65 1.65 0 0 0-1.51 1Z" />
  </svg>

  <svg
    v-else-if="name === 'bookmark'"
    :width="size"
    :height="size"
    viewBox="0 0 24 24"
    fill="none"
    stroke="currentColor"
    stroke-width="1.8"
    stroke-linecap="round"
    stroke-linejoin="round"
    aria-hidden="true"
  >
    <path d="M6 4h12v16l-6-4-6 4V4Z" />
  </svg>

  <svg
    v-else-if="name === 'bookmarkFill'"
    :width="size"
    :height="size"
    viewBox="0 0 24 24"
    fill="currentColor"
    stroke="currentColor"
    stroke-width="1.8"
    stroke-linecap="round"
    stroke-linejoin="round"
    aria-hidden="true"
  >
    <path d="M6 4h12v16l-6-4-6 4V4Z" />
  </svg>

  <svg
    v-else-if="name === 'plus'"
    :width="size"
    :height="size"
    viewBox="0 0 24 24"
    fill="none"
    stroke="currentColor"
    stroke-width="2"
    stroke-linecap="round"
    aria-hidden="true"
  >
    <path d="M12 5v14M5 12h14" />
  </svg>

  <svg
    v-else-if="name === 'refresh'"
    :width="size"
    :height="size"
    viewBox="0 0 24 24"
    fill="none"
    stroke="currentColor"
    stroke-width="1.9"
    stroke-linecap="round"
    stroke-linejoin="round"
    aria-hidden="true"
  >
    <path d="M20 11a8 8 0 1 0-2.3 5.7" />
    <path d="M20 4v7h-7" />
  </svg>

  <svg
    v-else-if="name === 'external'"
    :width="size"
    :height="size"
    viewBox="0 0 24 24"
    fill="none"
    stroke="currentColor"
    stroke-width="1.8"
    stroke-linecap="round"
    stroke-linejoin="round"
    aria-hidden="true"
  >
    <path d="M14 5h5v5" />
    <path d="M10 14L19 5" />
    <path d="M19 13v6H5V5h6" />
  </svg>

  <svg
    v-else-if="name === 'close'"
    :width="size"
    :height="size"
    viewBox="0 0 24 24"
    fill="none"
    stroke="currentColor"
    stroke-width="2"
    stroke-linecap="round"
    aria-hidden="true"
  >
    <path d="M18 6L6 18M6 6l12 12" />
  </svg>

  <svg
    v-else-if="name === 'check'"
    :width="size"
    :height="size"
    viewBox="0 0 24 24"
    fill="none"
    stroke="currentColor"
    stroke-width="2.3"
    stroke-linecap="round"
    stroke-linejoin="round"
    aria-hidden="true"
  >
    <path d="M5 12.5l4.5 4.5L19 7.5" />
  </svg>

  <svg
    v-else-if="name === 'back'"
    :width="size"
    :height="size"
    viewBox="0 0 24 24"
    fill="none"
    stroke="currentColor"
    stroke-width="1.8"
    stroke-linecap="round"
    stroke-linejoin="round"
    aria-hidden="true"
  >
    <path d="M15 18l-6-6 6-6" />
  </svg>

  <svg
    v-else-if="name === 'sliders'"
    :width="size"
    :height="size"
    viewBox="0 0 24 24"
    fill="none"
    stroke="currentColor"
    stroke-width="1.8"
    stroke-linecap="round"
    aria-hidden="true"
  >
    <path d="M4 7h16M4 17h16" />
    <circle cx="9" cy="7" r="2.2" fill="var(--bg-elevated)" />
    <circle cx="15" cy="17" r="2.2" fill="var(--bg-elevated)" />
  </svg>

  <svg
    v-else-if="name === 'search'"
    :width="size"
    :height="size"
    viewBox="0 0 24 24"
    fill="none"
    stroke="currentColor"
    stroke-width="2"
    stroke-linecap="round"
    aria-hidden="true"
  >
    <circle cx="11" cy="11" r="7" />
    <path d="m20 20-3.5-3.5" />
  </svg>

  <svg
    v-else-if="name === 'sun'"
    :width="size"
    :height="size"
    viewBox="0 0 24 24"
    fill="none"
    stroke="currentColor"
    stroke-width="1.9"
    stroke-linecap="round"
    stroke-linejoin="round"
    aria-hidden="true"
  >
    <circle cx="12" cy="12" r="4" />
    <path d="M12 2v2M12 20v2M4.93 4.93l1.41 1.41M17.66 17.66l1.41 1.41M2 12h2M20 12h2M4.93 19.07l1.41-1.41M17.66 6.34l1.41-1.41" />
  </svg>

  <svg
    v-else-if="name === 'moon'"
    :width="size"
    :height="size"
    viewBox="0 0 24 24"
    fill="none"
    stroke="currentColor"
    stroke-width="1.9"
    stroke-linecap="round"
    stroke-linejoin="round"
    aria-hidden="true"
  >
    <path d="M21 12.79A9 9 0 1 1 11.21 3 7 7 0 0 0 21 12.79z" />
  </svg>

  <svg
    v-else-if="name === 'mapPin'"
    :width="size"
    :height="size"
    viewBox="0 0 24 24"
    fill="none"
    stroke="currentColor"
    stroke-width="1.85"
    stroke-linecap="round"
    stroke-linejoin="round"
    aria-hidden="true"
  >
    <path d="M12 21s-6-5.3-6-10.5A6 6 0 1 1 18 10.5C18 15.7 12 21 12 21z" />
    <circle cx="12" cy="10.5" r="2.2" />
  </svg>

  <svg
    v-else-if="name === 'sparkle'"
    :width="size"
    :height="size"
    viewBox="0 0 24 24"
    fill="none"
    stroke="currentColor"
    stroke-width="1.85"
    stroke-linecap="round"
    stroke-linejoin="round"
    aria-hidden="true"
  >
    <path d="M12 3l1.9 5.6L19.5 10.5l-5.6 1.9L12 18l-1.9-5.6L4.5 10.5l5.6-1.9z" />
  </svg>

  <svg
    v-else-if="name === 'download'"
    :width="size"
    :height="size"
    viewBox="0 0 24 24"
    fill="none"
    stroke="currentColor"
    stroke-width="1.85"
    stroke-linecap="round"
    stroke-linejoin="round"
    aria-hidden="true"
  >
    <path d="M12 3v12M7 10l5 5 5-5M4 19h16" />
  </svg>

  <svg
    v-else-if="name === 'upload'"
    :width="size"
    :height="size"
    viewBox="0 0 24 24"
    fill="none"
    stroke="currentColor"
    stroke-width="1.85"
    stroke-linecap="round"
    stroke-linejoin="round"
    aria-hidden="true"
  >
    <path d="M12 15V3M7 8l5-5 5 5M4 19h16" />
  </svg>

  <svg
    v-else-if="name === 'trash'"
    :width="size"
    :height="size"
    viewBox="0 0 24 24"
    fill="none"
    stroke="currentColor"
    stroke-width="1.8"
    stroke-linecap="round"
    stroke-linejoin="round"
    aria-hidden="true"
  >
    <path d="M4 7h16M9 7V4h6v3M6 7l1 13h10l1-13" />
  </svg>

  <svg
    v-else-if="name === 'cv'"
    :width="size"
    :height="size"
    viewBox="0 0 24 24"
    fill="none"
    stroke="currentColor"
    stroke-width="1.8"
    stroke-linecap="round"
    stroke-linejoin="round"
    aria-hidden="true"
  >
    <path d="M7 3h7l5 5v13a1 1 0 0 1-1 1H7a1 1 0 0 1-1-1V4a1 1 0 0 1 1-1z" />
    <path d="M14 3v5h5M9.5 13h5M9.5 16.5h3.5" />
  </svg>

  <svg
    v-else-if="name === 'chevronDown'"
    :width="size"
    :height="size"
    viewBox="0 0 24 24"
    fill="none"
    stroke="currentColor"
    stroke-width="2"
    stroke-linecap="round"
    stroke-linejoin="round"
    aria-hidden="true"
  >
    <path d="M6 9l6 6 6-6" />
  </svg>

  <svg
    v-else-if="name === 'chevronRight'"
    :width="size"
    :height="size"
    viewBox="0 0 24 24"
    fill="none"
    stroke="currentColor"
    stroke-width="2"
    stroke-linecap="round"
    stroke-linejoin="round"
    aria-hidden="true"
  >
    <path d="M9 18l6-6-6-6" />
  </svg>

  <svg
    v-else-if="name === 'moreVertical'"
    :width="size"
    :height="size"
    viewBox="0 0 24 24"
    fill="currentColor"
    aria-hidden="true"
  >
    <circle cx="12" cy="5.5" r="1.75" />
    <circle cx="12" cy="12" r="1.75" />
    <circle cx="12" cy="18.5" r="1.75" />
  </svg>

  <svg
    v-else
    :width="size"
    :height="size"
    viewBox="0 0 24 24"
    fill="none"
    stroke="currentColor"
    stroke-width="2"
    stroke-linecap="round"
    stroke-linejoin="round"
    aria-hidden="true"
  >
    <circle cx="12" cy="12" r="3" />
    <path d="M12 3v3M12 18v3M3 12h3M18 12h3" />
  </svg>
</template>

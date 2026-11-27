import { onBeforeUnmount, watch, type Ref } from "vue";

interface OverlayEntry {
  id: string;
  close: () => void;
}

const overlayStack: OverlayEntry[] = [];
let listenerAttached = false;
let idSeq = 0;
let suppressNextPopCount = 0;
let closingFromPopstate = false;

function handleWindowPopstate() {
  if (suppressNextPopCount > 0) {
    suppressNextPopCount -= 1;
    return;
  }
  const top = overlayStack[overlayStack.length - 1];
  if (!top) return;

  const activeMarker =
    typeof window !== "undefined" &&
    window.history.state &&
    typeof window.history.state === "object"
      ? (window.history.state as Record<string, unknown>).__vectaOverlay
      : undefined;

  if (activeMarker === top.id) return;

  overlayStack.pop();
  closingFromPopstate = true;
  try {
    top.close();
  } finally {
    closingFromPopstate = false;
  }
}

function ensureListener() {
  if (listenerAttached || typeof window === "undefined") return;
  window.addEventListener("popstate", handleWindowPopstate);
  listenerAttached = true;
}

/**
 * Coordinates overlays, bottom sheets, dialogs, and popovers with Android WebView
 * hardware/gesture Back navigation via browser history state entries.
 */
export function useBackOverlay(
  isOpen: Ref<boolean>,
  onClose: () => void,
  idPrefix = "overlay",
) {
  ensureListener();
  let currentId: string | null = null;

  function registerOpen() {
    if (typeof window === "undefined" || currentId !== null) return;
    idSeq += 1;
    const id = `${idPrefix}-${idSeq}`;
    currentId = id;
    overlayStack.push({ id, close: onClose });
    const baseState =
      window.history.state && typeof window.history.state === "object"
        ? window.history.state
        : {};
    window.history.pushState({ ...baseState, __vectaOverlay: id }, "");
  }

  function unregisterClose() {
    if (currentId === null) return;
    const id = currentId;
    currentId = null;
    const idx = overlayStack.findIndex((item) => item.id === id);
    if (idx >= 0) {
      overlayStack.splice(idx, 1);
    }
    if (closingFromPopstate || typeof window === "undefined") return;
    const activeMarker =
      window.history.state && typeof window.history.state === "object"
        ? (window.history.state as Record<string, unknown>).__vectaOverlay
        : undefined;
    if (activeMarker === id) {
      suppressNextPopCount += 1;
      window.history.back();
    }
  }

  watch(
    isOpen,
    (open) => {
      if (open) {
        registerOpen();
      } else {
        unregisterClose();
      }
    },
    { flush: "sync" },
  );

  onBeforeUnmount(() => {
    unregisterClose();
  });
}

/**
 * Closes any currently tracked overlays synchronously before a route transition
 * so popovers or sheets never remain visible across pages.
 */
export function closeAllOverlaysForRouteChange() {
  if (overlayStack.length === 0) return;
  const entries = overlayStack.splice(0, overlayStack.length);
  closingFromPopstate = true;
  try {
    for (let i = entries.length - 1; i >= 0; i -= 1) {
      entries[i]?.close();
    }
  } finally {
    closingFromPopstate = false;
  }
}

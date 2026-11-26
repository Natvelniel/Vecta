import { ref } from "vue";

export type VectaTheme = "dark" | "light";

const STORAGE_KEY = "vecta_theme_v1";

function readInitialTheme(): VectaTheme {
  if (typeof window === "undefined") return "dark";
  try {
    const saved = window.localStorage.getItem(STORAGE_KEY);
    if (saved === "light" || saved === "dark") return saved;
  } catch {
    // Ignore storage read errors in restricted contexts
  }
  return "dark";
}

const currentTheme = ref<VectaTheme>(readInitialTheme());

function applyThemeToDom(theme: VectaTheme) {
  if (typeof document === "undefined") return;
  document.documentElement.dataset.theme = theme;
  document.documentElement.style.colorScheme = theme;
}

applyThemeToDom(currentTheme.value);

export function useTheme() {
  function setTheme(next: VectaTheme, origin?: { x: number; y: number }) {
    if (next === currentTheme.value) {
      applyThemeToDom(next);
      return;
    }

    const prefersReduced =
      typeof window !== "undefined" &&
      window.matchMedia?.("(prefers-reduced-motion: reduce)").matches;

    const docAny = document as Document & {
      startViewTransition?: (cb: () => void) => { ready: Promise<void> };
    };

    const commit = () => {
      currentTheme.value = next;
      applyThemeToDom(next);
      try {
        window.localStorage.setItem(STORAGE_KEY, next);
      } catch {
        // Ignore storage errors
      }
    };

    if (typeof document !== "undefined" && docAny.startViewTransition && !prefersReduced) {
      const x = origin?.x ?? window.innerWidth / 2;
      const y = origin?.y ?? window.innerHeight / 2;
      const maxR = Math.hypot(
        Math.max(x, window.innerWidth - x),
        Math.max(y, window.innerHeight - y),
      );
      const transition = docAny.startViewTransition(commit);
      transition.ready
        .then(() => {
          document.documentElement.animate(
            {
              clipPath: [
                `circle(0px at ${x}px ${y}px)`,
                `circle(${maxR}px at ${x}px ${y}px)`,
              ],
            },
            {
              duration: 420,
              easing: "cubic-bezier(0.22, 1, 0.36, 1)",
              pseudoElement: "::view-transition-new(root)",
            },
          );
        })
        .catch(() => {
          // Safe no-op if View Transition aborted
        });
    } else {
      commit();
    }
  }

  function toggleTheme(event?: MouseEvent) {
    const next: VectaTheme = currentTheme.value === "dark" ? "light" : "dark";
    const origin =
      event && typeof event.clientX === "number"
        ? { x: event.clientX, y: event.clientY }
        : undefined;
    setTheme(next, origin);
  }

  return {
    theme: currentTheme,
    setTheme,
    toggleTheme,
  };
}

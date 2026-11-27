import type { Ref } from "vue";

const routeScrollMemory = new Map<string, number>();
let scrollerRef: Ref<HTMLElement | null> | null = null;

export function registerMainScroller(elRef: Ref<HTMLElement | null>) {
  scrollerRef = elRef;
}

export function saveScrollForPath(path: string) {
  if (!path || !scrollerRef?.value) return;
  routeScrollMemory.set(path, scrollerRef.value.scrollTop);
}

export function restoreScrollForPath(path: string) {
  if (!scrollerRef?.value) return;
  scrollerRef.value.scrollTop = routeScrollMemory.get(path) ?? 0;
}

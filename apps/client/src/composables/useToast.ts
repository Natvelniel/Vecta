import { ref } from "vue";

export type ToastTone = "info" | "success" | "warning" | "error";

export interface ToastItem {
  id: number;
  message: string;
  tone: ToastTone;
  actionLabel?: string;
  onAction?: () => unknown;
}

const toasts = ref<ToastItem[]>([]);
let nextId = 1;

export function useToast() {
  function dismiss(id: number) {
    toasts.value = toasts.value.filter((t) => t.id !== id);
  }

  function show(
    message: string,
    options?: {
      tone?: ToastTone;
      actionLabel?: string;
      onAction?: () => unknown;
      durationMs?: number;
    },
  ): number {
    const id = nextId++;
    toasts.value = [
      ...toasts.value.slice(-2),
      {
        id,
        message,
        tone: options?.tone ?? "info",
        actionLabel: options?.actionLabel,
        onAction: options?.onAction,
      },
    ];
    const duration = options?.durationMs ?? (options?.actionLabel ? 8000 : 4500);
    if (duration > 0) {
      setTimeout(() => dismiss(id), duration);
    }
    return id;
  }

  return { toasts, show, dismiss };
}

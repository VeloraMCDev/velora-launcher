export type Toast = { id: number; kind: 'ok' | 'error' | 'info'; text: string };

export const toasts = $state<Toast[]>([]);
let next = 1;

export function toast(text: string, kind: Toast['kind'] = 'ok') {
  const id = next++;
  toasts.push({ id, kind, text });
  setTimeout(() => dismiss(id), kind === 'error' ? 6000 : 3200);
}

export function dismiss(id: number) {
  const i = toasts.findIndex((t) => t.id === id);
  if (i >= 0) toasts.splice(i, 1);
}

export function toastError(e: unknown) {
  toast(e instanceof Error ? e.message : String(e), 'error');
}

import { route } from './router.svelte';
import { logout, session } from './session.svelte';

export { ApiError } from '../../../../shared/http/client.mjs';
import { createPlatformClient } from '../../../../shared/http/platform.mjs';
import type { Options } from '../../../../shared/http/client.mjs';

export const platform = createPlatformClient({
  getToken: () => session.token,
  getInstance: () => route.instanceId,
  onUnauthorized: logout,
  timeoutMs: 0, // Preserve the compatibility host's original request deadline.
});
const transport = platform.transport;
export const api = <T = any>(path: string, opts: Options = {}): Promise<T> => transport.api<T>(path, opts);
export const get = <T = any>(p: string): Promise<T> => transport.get<T>(p);
export const post = <T = any>(p: string, body?: unknown): Promise<T> => transport.post<T>(p, body);
export const put = <T = any>(p: string, body: unknown): Promise<T> => transport.put<T>(p, body);
export const patch = <T = any>(p: string, body: unknown): Promise<T> => transport.patch<T>(p, body);
export const del = <T = any>(p: string): Promise<T> => transport.del<T>(p);

export async function uploadMedia(file: File): Promise<string> {
  const form = new FormData();
  form.append('file', file);
  const res = await api<{ url: string }>('/api/admin/uploads', { method: 'POST', form });
  return res.url;
}

export function formatBytes(n: number): string {
  if (!n) return '0 B';
  const units = ['B', 'KB', 'MB', 'GB'];
  const i = Math.min(units.length - 1, Math.floor(Math.log(n) / Math.log(1024)));
  return `${(n / 1024 ** i).toFixed(i ? 1 : 0)} ${units[i]}`;
}

export function timeAgo(iso?: string | null): string {
  if (!iso) return 'never';
  const s = (Date.now() - new Date(iso).getTime()) / 1000;
  if (s < 60) return 'just now';
  if (s < 3600) return `${Math.floor(s / 60)}m ago`;
  if (s < 86400) return `${Math.floor(s / 3600)}h ago`;
  if (s < 86400 * 30) return `${Math.floor(s / 86400)}d ago`;
  return new Date(iso).toLocaleDateString();
}

export const LOADERS = [
  { id: 'vanilla', label: 'Vanilla' },
  { id: 'fabric', label: 'Fabric' },
  { id: 'quilt', label: 'Quilt' },
  { id: 'forge', label: 'Forge' },
  { id: 'neoforge', label: 'NeoForge' },
] as const;

export const loaderLabel = (id: string) => LOADERS.find((l) => l.id === id)?.label ?? id;

export function duration(secs: number): string {
  if (!secs) return '0m';
  const h = Math.floor(secs / 3600);
  const m = Math.floor((secs % 3600) / 60);
  if (h >= 100) return `${h.toLocaleString()}h`;
  return h ? `${h}h ${m}m` : `${m}m`;
}

export async function copy(text: string) {
  await navigator.clipboard.writeText(text);
}

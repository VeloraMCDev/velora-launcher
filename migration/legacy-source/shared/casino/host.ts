// The casino is shared by the launcher (Tauri) and the web player panel. Each app tells it how to reach the panel API,
// how to show a toast and where avatars live, once at start-up, with `setCasinoHost`.
export type ToastKind = 'ok' | 'error' | 'info';

export interface CasinoHost {
  get<T>(serverId: number, path: string): Promise<T>;
  post<T>(serverId: number, path: string, body: Record<string, unknown>): Promise<T>;
  toast(text: string, kind?: ToastKind): void;
  /** A head image URL for a player, or null to fall back to a coloured initial. */
  avatar(uuid: string | null | undefined, size: number): string | null;
}

let current: CasinoHost | null = null;

export function setCasinoHost(host: CasinoHost) {
  current = host;
}

function host(): CasinoHost {
  if (!current) throw new Error('The casino has no host. Call setCasinoHost() first.');
  return current;
}

export const hostGet = <T>(serverId: number, path: string) => host().get<T>(serverId, path);
export const hostPost = <T>(serverId: number, path: string, body: Record<string, unknown>) => host().post<T>(serverId, path, body);
export const toast = (text: string, kind: ToastKind = 'ok') => host().toast(text, kind);
export const avatarUrl = (uuid: string | null | undefined, size = 64) => current?.avatar(uuid, size) ?? null;
export const errorText = (e: unknown): string => (typeof e === 'string' ? e : e instanceof Error ? e.message : JSON.stringify(e));

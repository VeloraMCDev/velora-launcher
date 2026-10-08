// Thin wrapper over Tauri IPC. When the UI runs in a normal browser
// (vite dev / screenshots), calls go to an in-memory mock instead.
import { invoke as tauriInvoke } from '@tauri-apps/api/core';
import { listen as tauriListen, type UnlistenFn } from '@tauri-apps/api/event';

export const inTauri = typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window;

let experienceReady: Promise<unknown> = Promise.resolve();
export function setExperienceContext(instanceId: string | null) {
  if (inTauri) experienceReady = experienceReady.catch(() => {}).then(() => tauriInvoke('select_experience', { instanceId }));
}

type Handler = (payload: any) => void;
const mockListeners = new Map<string, Set<Handler>>();

export function mockEmit(event: string, payload: unknown) {
  mockListeners.get(event)?.forEach((h) => h(payload));
}

export async function invoke<T>(cmd: string, args?: Record<string, unknown>): Promise<T> {
  if (inTauri) {
    await experienceReady;
    const result = await tauriInvoke<T>(cmd, args);
    const kinds: Record<string, string> = {
      bootstrap: 'launcher_open', save_settings: 'settings_changed', select_account: 'account_selected',
      cancel_launch: 'launch_cancelled', delete_instance_data: 'instance_deleted', clear_cache: 'cache_cleared',
      install_update: 'update_installed', open_folder: 'folder_opened',
    };
    if (kinds[cmd]) void tauriInvoke('record_activity', { kind: kinds[cmd], instanceId: args?.instanceId ?? null }).catch(() => {});
    return result;
  }
  const { mockInvoke } = await import('./mock');
  return mockInvoke(cmd, args ?? {}) as Promise<T>;
}

export async function listen<T>(event: string, handler: (payload: T) => void): Promise<UnlistenFn> {
  if (inTauri) return tauriListen<T>(event, (e) => handler(e.payload));
  const set = mockListeners.get(event) ?? new Set();
  set.add(handler as Handler);
  mockListeners.set(event, set);
  return () => set.delete(handler as Handler);
}

export async function windowAction(action: 'minimize' | 'maximize' | 'close' | 'drag') {
  if (!inTauri) return;
  const { getCurrentWindow } = await import('@tauri-apps/api/window');
  const w = getCurrentWindow();
  if (action === 'minimize') await w.minimize();
  else if (action === 'maximize') await w.toggleMaximize();
  else if (action === 'close') {
    await tauriInvoke('record_activity', { kind: 'launcher_close', instanceId: null }).catch(() => {});
    await w.close();
  }
  else await w.startDragging();
}

export async function openUrl(url: string) {
  if (!inTauri) {
    window.open(url, '_blank');
    return;
  }
  const { openUrl } = await import('@tauri-apps/plugin-opener');
  await openUrl(url);
  void tauriInvoke('record_activity', { kind: 'link_opened', instanceId: null }).catch(() => {});
}

export async function pickFile(title: string, filters?: { name: string; extensions: string[] }[]): Promise<string | null> {
  if (!inTauri) return null;
  const { open } = await import('@tauri-apps/plugin-dialog');
  const r = await open({ title, multiple: false, directory: false, filters });
  return typeof r === 'string' ? r : null;
}

export function errorText(e: unknown): string {
  return typeof e === 'string' ? e : e instanceof Error ? e.message : JSON.stringify(e);
}

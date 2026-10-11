import { iconUrl, spriteUrl } from '@velora/experience/icons.svelte';
// Legacy texture status remains compatible; interface images load from ccVaults.
import { invoke, listen } from './tauri';

export const textures = $state({ ready: false, count: 0, version: null as string | null, checked: false, bannerDismissed: false });

/** Bumped whenever the texture set changes so tiles ask again. */
export const texVersion = $state({ n: 0 });

function apply(s: { ready: boolean; count: number; version: string | null }) {
  const changed = s.ready !== textures.ready || s.version !== textures.version;
  textures.ready = s.ready;
  textures.count = s.count;
  textures.version = s.version;
  textures.checked = true;
  if (changed) {
    texVersion.n++;
  }
}

export async function initTextures() {
  listen<{ ready: boolean; count: number; version: string | null }>('textures://status', apply);
  try {
    apply(await invoke('textures_status'));
  } catch {
    textures.checked = true;
  }
}

/** The texture for an item id from ccVaults or the administrator override. */
export function itemTexture(id: string): Promise<string | null> {
  return Promise.resolve(iconUrl(id));
}

/** Resolve a GUI or effect icon from ccVaults without requiring a game installation. */
export function sprite(name: string): Promise<string | null> {
  return Promise.resolve(spriteUrl(name));
}

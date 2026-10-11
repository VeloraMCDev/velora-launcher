import { iconUrl, spriteUrl, VAULT_ITEMS } from '@velora/experience/icons.svelte';
// UI icons load from ccVaults. Legacy local-texture status is retained for existing operator controls.

export const mc = $state({ loaded: false, ready: false, v: '', version: null as string | null, source: null as string | null });

let started: Promise<void> | null = null;

/** Ask the panel (once) whether textures are installed. Safe to call from anywhere, signed in or not. */
export function loadMc(force = false): Promise<void> {
  if (started && !force) return started;
  started = fetch('/api/v1/mc/status')
    .then((r) => (r.ok ? r.json() : null))
    .then((s) => {
      mc.ready = !!s?.ready;
      mc.v = s?.v ?? '';
      mc.version = s?.version ?? null;
      mc.source = s?.source ?? null;
    })
    .catch(() => { mc.ready = false; })
    .finally(() => { mc.loaded = true; });
  return started;
}


/** `minecraft:diamond_sword`, `DIAMOND_SWORD` or `diamond_sword` → texture URL, or null when it can't be one. */
export function itemUrl(id: string | null | undefined): string | null {
  return iconUrl(id);
}

/** A GUI sprite such as `hud/heart/full`, `container/slot` (null when textures aren't installed). */
export function guiUrl(path: string): string | null {
  return spriteUrl(path);
}

/** A status-effect icon such as `speed`. */
export function effectUrl(name: string): string | null {
  return spriteUrl('effect/' + name);
}

/** A block texture such as `deepslate_bricks` (tiled backdrops). */
export function blockUrl(name: string): string | null {
  return mc.ready && /^[a-z0-9_]+$/.test(name) ? `/api/v1/mc/block/${name}.png?v=${encodeURIComponent(mc.v)}` : null;
}

/** Every item id that has a texture (admin only — the item pickers search this). */
export const mcItems = $state({ list: [] as string[], loaded: false });

export async function loadMcItems(): Promise<void> {
  mcItems.list = Object.keys(VAULT_ITEMS).sort();
  mcItems.loaded = true;
}

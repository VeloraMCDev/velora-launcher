// Minecraft's own textures, served by the panel from a client jar (see crates/platform-utils/src/mc_textures.rs).
// Nothing is bundled: until the panel has them every helper returns null and callers keep their own fallback icon.

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

const ID = /^[a-z0-9_]+$/;
const url = (kind: string, file: string) => `/api/v1/mc/${kind}/${file}?v=${encodeURIComponent(mc.v)}`;

/** `minecraft:diamond_sword`, `DIAMOND_SWORD` or `diamond_sword` → texture URL, or null when it can't be one. */
export function itemUrl(id: string | null | undefined): string | null {
  const name = (id ?? '').trim().replace(/^minecraft:/i, '').toLowerCase();
  return mc.ready && ID.test(name) ? url('item', `${name}.png`) : null;
}

/** A GUI sprite such as `hud/heart/full`, `container/slot` (null when textures aren't installed). */
export function guiUrl(path: string): string | null {
  return mc.ready && /^[a-z0-9_/]+$/.test(path) ? url('gui', `${path}.png`) : null;
}

/** A status-effect icon such as `speed`. */
export function effectUrl(name: string): string | null {
  return mc.ready && ID.test(name) ? url('effect', `${name}.png`) : null;
}

/** A block texture such as `deepslate_bricks` (tiled backdrops). */
export function blockUrl(name: string): string | null {
  return mc.ready && ID.test(name) ? url('block', `${name}.png`) : null;
}

/** Every item id that has a texture (admin only — the item pickers search this). */
export const mcItems = $state({ list: [] as string[], loaded: false });

export async function loadMcItems(): Promise<void> {
  if (mcItems.loaded) return;
  mcItems.loaded = true;
  try {
    const { get } = await import('./api');
    mcItems.list = (await get<{ items?: string[] }>('/api/admin/mc-textures')).items ?? [];
  } catch {
    mcItems.loaded = false;
  }
}

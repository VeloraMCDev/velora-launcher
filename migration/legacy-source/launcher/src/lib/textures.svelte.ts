// Real Minecraft item textures. The launcher copies them out of the player's own client jar (never shipped with SCOPENET), so
// until the game has been installed once the UI shows emoji instead and a banner explains why.
import { invoke, listen } from './tauri';

export const textures = $state({ ready: false, count: 0, version: null as string | null, checked: false, bannerDismissed: false });

const cache = new Map<string, string | null>();
const waiting = new Map<string, Set<(uri: string | null) => void>>();
let timer: ReturnType<typeof setTimeout> | null = null;
/** Bumped whenever the texture set changes so tiles ask again. */
export const texVersion = $state({ n: 0 });

function apply(s: { ready: boolean; count: number; version: string | null }) {
  const changed = s.ready !== textures.ready || s.version !== textures.version;
  textures.ready = s.ready;
  textures.count = s.count;
  textures.version = s.version;
  textures.checked = true;
  if (changed) {
    cache.clear();
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

async function flush() {
  timer = null;
  const names = [...waiting.keys()];
  let found: Record<string, string> = {};
  try {
    found = await invoke<Record<string, string>>('item_textures', { names });
  } catch {
    /* fall back to emoji */
  }
  for (const n of names) {
    const uri = found[n] ?? null;
    cache.set(n, uri);
    waiting.get(n)?.forEach((cb) => cb(uri));
    waiting.delete(n);
  }
}

/** The texture for an item id as a data URI, or null when there isn't one (yet). Requests are batched into one call. */
export function itemTexture(id: string): Promise<string | null> {
  if (!textures.ready) return Promise.resolve(null);
  if (cache.has(id)) return Promise.resolve(cache.get(id) ?? null);
  return new Promise((resolve) => {
    const set = waiting.get(id) ?? new Set();
    set.add(resolve);
    waiting.set(id, set);
    timer ??= setTimeout(flush, 16);
  });
}

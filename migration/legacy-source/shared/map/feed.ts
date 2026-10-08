import type { MapInfo, MapOverlay } from './types';

export interface FeedOptions {
  loadInfo: () => Promise<MapInfo>;
  loadOverlay: () => Promise<MapOverlay>;
  onInfo: (info: MapInfo) => void;
  onOverlay: (overlay: MapOverlay) => void;
  onError?: (message: string) => void;
  /** How often players move on the map. */
  overlayMs?: number;
}

/**
 * Keeps a map fresh: the overlay (players, claims, pins) every couple of seconds while the window is visible, and the info
 * (which carries the tile key) when it is about to expire. Returns a function that stops everything.
 */
export function startFeed(o: FeedOptions): () => void {
  let stopped = false;
  let infoTimer: ReturnType<typeof setTimeout> | undefined;
  let overlayTimer: ReturnType<typeof setTimeout> | undefined;
  let ready = false;

  const fail = (e: unknown) => o.onError?.(typeof e === 'string' ? e : (e as { message?: string })?.message ?? 'Could not reach the map');

  async function info() {
    try {
      const next = await o.loadInfo();
      if (stopped) return;
      ready = next.ready;
      o.onInfo(next);
      // The tile key is good for an hour: renew it with plenty of time to spare. While waiting for the first tiles, check often.
      const wait = next.ready ? Math.max(30, Math.min(next.token_ttl_secs * 0.6, 1200)) * 1000 : 8000;
      infoTimer = setTimeout(info, wait);
    } catch (e) {
      fail(e);
      if (!stopped) infoTimer = setTimeout(info, 10000);
    }
  }

  async function overlay() {
    if (stopped) return;
    if (ready && !document.hidden) {
      try { o.onOverlay(await o.loadOverlay()); } catch (e) { fail(e); }
    }
    if (!stopped) overlayTimer = setTimeout(overlay, o.overlayMs ?? 2500);
  }

  info();
  overlayTimer = setTimeout(overlay, 600);
  return () => { stopped = true; clearTimeout(infoTimer); clearTimeout(overlayTimer); };
}

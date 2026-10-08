// What the panel's map endpoints return (see panel/server/src/routes/worldmap.rs), shared by the launcher and the admin panel.

export interface MapBounds { min_x: number; max_x: number; min_y: number; max_y: number }

export interface MapDimension {
  id: string;
  slug: string;
  label: string;
  available: boolean;
  tiles: number;
  bytes: number;
  bounds: MapBounds | null;
  /** The populated heart of the map (10th to 90th percentile of tiles), in zoom-0 tile units. */
  core?: MapBounds | null;
  /** Tiles stored per zoom level (index = zoom). */
  levels?: number[];
}

export interface MapInfo {
  enabled: boolean;
  ready: boolean;
  message: string;
  tile_size: number;
  max_zoom: number;
  /** Where tiles live, e.g. `https://panel.example.com/api/map/3`. */
  tile_base: string;
  /** Short-lived key that goes on each tile address. */
  token: string | null;
  token_ttl_secs: number;
  dimensions: MapDimension[];
  players: number;
}

export interface LivePlayer {
  uuid: string;
  name: string;
  dimension: string;
  x: number;
  y: number;
  z: number;
  yaw: number;
}

export interface MapClaim {
  id: string;
  guild_id: string;
  name: string;
  tag: string;
  icon: string;
  color: string;
  dimension: string;
  chunks: number;
  label_x: number;
  label_z: number;
  outer: [number, number][];
  holes: [number, number][][];
  lines: string[];
}

export type PinKind = 'spawn' | 'warp' | 'home' | 'guild_home' | 'market' | 'shop';

export interface MapPin {
  id: string;
  kind: PinKind | string;
  label: string;
  dimension: string;
  x: number;
  y: number;
  z: number;
  lines: string[];
}

export interface MapOverlay {
  players: LivePlayer[];
  claims: MapClaim[];
  pins: MapPin[];
  updated?: string | null;
}

export type MapSelection =
  | { type: 'player'; player: LivePlayer }
  | { type: 'pin'; pin: MapPin }
  | { type: 'claim'; claim: MapClaim };

export interface MapLayers { claims: boolean; pins: boolean; players: boolean }

/** Same naming as the panel: `minecraft:the_nether` -> `the_nether`, `mod:dim` -> `mod__dim`. */
export function dimSlug(id: string): string {
  const trimmed = id.startsWith('minecraft:') ? id.slice(10) : id;
  if (trimmed === 'nether') return 'the_nether';
  if (trimmed === 'end') return 'the_end';
  return trimmed.replace(':', '__');
}

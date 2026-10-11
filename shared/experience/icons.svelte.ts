import catalogue from './vault-icons.json';
import sprites from './vault-interface.json';

// URL metadata only. Artwork stays on ccVaults and retains its original terms.
export const VAULT_ITEMS: Record<string, string> = { ...sprites, ...catalogue };
export const interfaceIcons = $state({ overrides: {} as Record<string, string>, base: '' });
export function configureIcons(overrides?: Record<string, string>, base = '') {
  interfaceIcons.overrides = overrides ?? {};
  interfaceIcons.base = base;
}
export function imageValue(value: string): string | null {
  if (/^https:\/\/[^\s]+$/i.test(value)) return value;
  if (/^\/uploads\/[^\s]+$/i.test(value)) return interfaceIcons.base + value;
  const icon = /^iconify:([a-z0-9-]+):([a-z0-9_-]+)(?:@([0-9a-f]{3}|[0-9a-f]{6}))?$/i.exec(value);
  return icon ? `${interfaceIcons.base}/api/v1/icons/${icon[1]}/${icon[2]}.svg${icon[3] ? '?color=' + icon[3] : ''}` : null;
}
export function iconUrl(id?: string | null, key?: string): string | null {
  const normalized = (id ?? '').trim().replace(/^minecraft:/i, '').toLowerCase();
  const value = (key && interfaceIcons.overrides[key]) || interfaceIcons.overrides[`item:${normalized}`] || id || '';
  const image = imageValue(value);
  if (image) return image;
  const name = value.trim().replace(/^minecraft:/i, '').toLowerCase();
  const aliases: Record<string, string> = { skull: 'skeleton_skull', castle: 'white_banner', trophy: 'golden_helmet', map: 'filled_map' };
  const path = VAULT_ITEMS[aliases[name] ?? name];
  return path ? `https://ccvaults.com/assets/${path.split('/').map(encodeURIComponent).join('/')}` : null;
}

export function spriteUrl(path: string): string | null {
  const normalized = path.replace(/^gui\//, '').replace(/\.png$/, '');
  const name = normalized.replace(/^hud\//, '').replace(/^effect\//, '').replaceAll('/', '_');
  return iconUrl(name, 'sprite:' + normalized);
}

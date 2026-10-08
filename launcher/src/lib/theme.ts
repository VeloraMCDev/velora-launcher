import type { Branding, Settings } from './types';

let styleEl: HTMLStyleElement | null = null;

/** Push the admin's branding (and the player's overrides) into CSS variables. */
export function applyTheme(b: Branding, s: Settings | null) {
  const root = document.documentElement;
  const custom = s?.theme_mode === 'custom' && b.features.allow_user_theme;
  const accent = (custom && s?.accent) || b.colors.accent;
  const vars: Record<string, string> = {
    '--accent': accent,
    '--accent-2': custom && s?.accent ? shift(accent) : b.colors.accent_2,
    '--bg': b.colors.background,
    '--surface': b.colors.surface,
    '--text': b.colors.text,
    '--muted': b.colors.muted,
    '--success': b.colors.success,
    '--danger': b.colors.danger,
    '--radius': `${b.radius}px`,
    '--radius-sm': `${Math.max(4, Math.round(b.radius * 0.65))}px`,
    '--font': `'${b.font} Variable', '${b.font}', 'Outfit Variable', system-ui, sans-serif`,
  };
  for (const [k, v] of Object.entries(vars)) root.style.setProperty(k, v);
  const glass = s?.glass ?? b.glass;
  root.classList.toggle('no-glass', !glass);
  root.classList.toggle('reduce-motion', !!s?.reduce_motion);
  root.style.fontSize = `${(s?.ui_scale ?? 100) * 0.15}px`;

  if (!styleEl) {
    styleEl = document.createElement('style');
    styleEl.id = 'admin-css';
    document.head.appendChild(styleEl);
  }
  styleEl.textContent = b.custom_css ?? '';
}

/** A second accent derived from the first by rotating its hue. */
function shift(hex: string): string {
  const m = hex.replace('#', '').match(/.{2}/g);
  if (!m) return hex;
  let [r, g, b] = m.map((x) => parseInt(x, 16) / 255);
  const max = Math.max(r, g, b), min = Math.min(r, g, b);
  let h = 0;
  const l = (max + min) / 2;
  const d = max - min;
  const sat = d === 0 ? 0 : d / (1 - Math.abs(2 * l - 1));
  if (d) {
    if (max === r) h = ((g - b) / d) % 6;
    else if (max === g) h = (b - r) / d + 2;
    else h = (r - g) / d + 4;
  }
  h = (h * 60 + 50 + 360) % 360;
  return `hsl(${h} ${Math.round(sat * 100)}% ${Math.round(l * 100)}%)`;
}

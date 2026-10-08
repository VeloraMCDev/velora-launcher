// Markdown-style Minecraft text. People type **bold**, *italic*, __underline__, ~~strike~~, {gold} / {#ff8800} for colours and {/} to
// reset; the game (and the panel API) store the classic &-codes. `markdownToAmp` and `ampToMarkdown` convert both ways so editing an
// existing item shows what the author would have typed.

import { spans, COLORS, type Span } from './mccolor';

export const COLOR_NAMES: Record<string, string> = {
  black: '0', dark_blue: '1', dark_green: '2', dark_aqua: '3', dark_red: '4', dark_purple: '5', gold: '6', gray: '7',
  dark_gray: '8', blue: '9', green: 'a', aqua: 'b', red: 'c', light_purple: 'd', yellow: 'e', white: 'f',
};
const NAME_BY_CODE = Object.fromEntries(Object.entries(COLOR_NAMES).map(([n, c]) => [c, n]));

/** The 16 vanilla colours as [name, code, hex]. */
export const PALETTE: [string, string, string][] = Object.entries(COLOR_NAMES).map(([name, code]) => [name, code, COLORS[code]]);

interface State { color: string | null; bold: boolean; italic: boolean; underline: boolean; strike: boolean }
const fresh = (): State => ({ color: null, bold: false, italic: false, underline: false, strike: false });

function colorCode(c: string): string {
  const hex = c.toLowerCase();
  if (/^#[0-9a-f]{6}$/.test(hex)) {
    const vanilla = Object.entries(COLORS).find(([, v]) => v.toLowerCase() === hex);
    return vanilla ? '&' + vanilla[0] : '&' + hex;
  }
  return '&' + c;
}

/** `**Epic** {gold}Blade` → `&l Epic&r&6 Blade`. Unknown tokens stay as typed. */
export function markdownToAmp(md: string): string {
  let out = '';
  let st = fresh();
  let dirty = false;
  let i = 0;
  const emitState = () => {
    out += '&r';
    if (st.color) out += st.color;
    if (st.bold) out += '&l';
    if (st.italic) out += '&o';
    if (st.underline) out += '&n';
    if (st.strike) out += '&m';
    dirty = false;
  };
  while (i < md.length) {
    const rest = md.slice(i);
    if (rest[0] === '\\' && rest.length > 1) {
      if (dirty) emitState();
      out += rest[1];
      i += 2;
      continue;
    }
    const brace = /^\{(\/|reset|#[0-9a-fA-F]{6}|[a-z_]+)\}/.exec(rest);
    if (brace) {
      const t = brace[1];
      if (t === '/' || t === 'reset') st = fresh();
      else if (t.startsWith('#')) st = { ...fresh(), color: colorCode(t) };
      else if (COLOR_NAMES[t]) st = { ...fresh(), color: '&' + COLOR_NAMES[t] };
      else { if (dirty) emitState(); out += brace[0]; i += brace[0].length; continue; }
      dirty = true;
      i += brace[0].length;
      continue;
    }
    if (rest.startsWith('**')) { st.bold = !st.bold; dirty = true; i += 2; continue; }
    if (rest.startsWith('__')) { st.underline = !st.underline; dirty = true; i += 2; continue; }
    if (rest.startsWith('~~')) { st.strike = !st.strike; dirty = true; i += 2; continue; }
    if (rest[0] === '*') { st.italic = !st.italic; dirty = true; i += 1; continue; }
    if (dirty) emitState();
    out += rest[0];
    i += 1;
  }
  return out;
}

function nameOf(hex: string): string {
  const h = hex.toLowerCase();
  const code = Object.entries(COLORS).find(([, v]) => v.toLowerCase() === h)?.[0];
  return code ? `{${NAME_BY_CODE[code]}}` : `{${h}}`;
}

/** `&6&lEpic &7Blade` → `{gold}**Epic **{gray}Blade`. `base` is the colour text has when no code applies. */
export function ampToMarkdown(amp: string, base = '#ffffff'): string {
  const list: Span[] = spans(amp, base);
  let out = '';
  let prev = { color: base.toLowerCase(), bold: false, italic: false, underline: false, strike: false };
  const esc = (t: string) => t.replace(/([\\*_~{])/g, '\\$1');
  for (const s of list) {
    if (!s.text) continue;
    // turn styles off first, then recolour, then turn styles on
    const color = s.color.toLowerCase();
    const recolour = color !== prev.color;
    if (prev.bold && !s.bold) out += '**';
    if (prev.italic && !s.italic) out += '*';
    if (prev.underline && !s.underline) out += '__';
    if (prev.strike && !s.strike) out += '~~';
    if (recolour) out += color === base.toLowerCase() ? '{/}' : nameOf(color);
    if (s.bold && !prev.bold) out += '**';
    if (s.italic && !prev.italic) out += '*';
    if (s.underline && !prev.underline) out += '__';
    if (s.strike && !prev.strike) out += '~~';
    out += esc(s.text);
    prev = { color, bold: s.bold, italic: s.italic, underline: s.underline, strike: s.strike };
  }
  if (prev.bold) out += '**';
  if (prev.italic) out += '*';
  if (prev.underline) out += '__';
  if (prev.strike) out += '~~';
  return out;
}

/** Strip every code and token, for plain titles. */
export function plainText(amp: string): string {
  return amp.replace(/&#[0-9a-fA-F]{6}/g, '').replace(/&[0-9a-fk-orA-FK-OR]/g, '');
}

function lerp(a: number, b: number, t: number) { return Math.round(a + (b - a) * t); }
const hexToRgb = (h: string) => [1, 3, 5].map((i) => parseInt(h.slice(i, i + 2), 16));
const rgbToHex = (r: number[]) => '#' + r.map((v) => v.toString(16).padStart(2, '0')).join('');

/** Colour each character of `text` along a gradient through `stops` (2+ hex colours). Returns markdown tokens. */
export function gradientMarkdown(text: string, stops: string[]): string {
  const chars = [...text];
  const visible = chars.filter((c) => c.trim()).length;
  if (visible === 0 || stops.length < 2) return text;
  let n = 0;
  let out = '';
  for (const c of chars) {
    if (!c.trim()) { out += c; continue; }
    const t = visible === 1 ? 0 : n / (visible - 1);
    const seg = Math.min(stops.length - 2, Math.floor(t * (stops.length - 1)));
    const local = t * (stops.length - 1) - seg;
    const a = hexToRgb(stops[seg]), b = hexToRgb(stops[seg + 1]);
    out += `{${rgbToHex([lerp(a[0], b[0], local), lerp(a[1], b[1], local), lerp(a[2], b[2], local)])}}${c === '*' || c === '_' || c === '~' || c === '{' || c === '\\' ? '\\' + c : c}`;
    n++;
  }
  return out + '{/}';
}

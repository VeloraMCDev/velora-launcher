// Pixel-art rank badge renderer: gradient plate, chunky 5x7 lettering with outline and drop shadow, optional pixel icon.
// Everything is drawn from bitmaps below, so the output is identical on every machine and needs no font files.

const GLYPHS: Record<string, string[]> = {
  A: ['01110', '10001', '10001', '11111', '10001', '10001', '10001'],
  B: ['11110', '10001', '10001', '11110', '10001', '10001', '11110'],
  C: ['01110', '10001', '10000', '10000', '10000', '10001', '01110'],
  D: ['11110', '10001', '10001', '10001', '10001', '10001', '11110'],
  E: ['11111', '10000', '10000', '11110', '10000', '10000', '11111'],
  F: ['11111', '10000', '10000', '11110', '10000', '10000', '10000'],
  G: ['01110', '10001', '10000', '10111', '10001', '10001', '01111'],
  H: ['10001', '10001', '10001', '11111', '10001', '10001', '10001'],
  I: ['11111', '00100', '00100', '00100', '00100', '00100', '11111'],
  J: ['00111', '00010', '00010', '00010', '00010', '10010', '01100'],
  K: ['10001', '10010', '10100', '11000', '10100', '10010', '10001'],
  L: ['10000', '10000', '10000', '10000', '10000', '10000', '11111'],
  M: ['10001', '11011', '10101', '10101', '10001', '10001', '10001'],
  N: ['10001', '11001', '10101', '10011', '10001', '10001', '10001'],
  O: ['01110', '10001', '10001', '10001', '10001', '10001', '01110'],
  P: ['11110', '10001', '10001', '11110', '10000', '10000', '10000'],
  Q: ['01110', '10001', '10001', '10001', '10101', '10010', '01101'],
  R: ['11110', '10001', '10001', '11110', '10100', '10010', '10001'],
  S: ['01111', '10000', '10000', '01110', '00001', '00001', '11110'],
  T: ['11111', '00100', '00100', '00100', '00100', '00100', '00100'],
  U: ['10001', '10001', '10001', '10001', '10001', '10001', '01110'],
  V: ['10001', '10001', '10001', '10001', '10001', '01010', '00100'],
  W: ['10001', '10001', '10001', '10101', '10101', '11011', '10001'],
  X: ['10001', '10001', '01010', '00100', '01010', '10001', '10001'],
  Y: ['10001', '10001', '01010', '00100', '00100', '00100', '00100'],
  Z: ['11111', '00001', '00010', '00100', '01000', '10000', '11111'],
  '0': ['01110', '10001', '10011', '10101', '11001', '10001', '01110'],
  '1': ['00100', '01100', '00100', '00100', '00100', '00100', '01110'],
  '2': ['01110', '10001', '00001', '00110', '01000', '10000', '11111'],
  '3': ['11110', '00001', '00001', '01110', '00001', '00001', '11110'],
  '4': ['00010', '00110', '01010', '10010', '11111', '00010', '00010'],
  '5': ['11111', '10000', '11110', '00001', '00001', '10001', '01110'],
  '6': ['00110', '01000', '10000', '11110', '10001', '10001', '01110'],
  '7': ['11111', '00001', '00010', '00100', '01000', '01000', '01000'],
  '8': ['01110', '10001', '10001', '01110', '10001', '10001', '01110'],
  '9': ['01110', '10001', '10001', '01111', '00001', '00010', '01100'],
  '-': ['00000', '00000', '00000', '11111', '00000', '00000', '00000'],
  '+': ['00000', '00100', '00100', '11111', '00100', '00100', '00000'],
  '.': ['00000', '00000', '00000', '00000', '00000', '01100', '01100'],
  '!': ['00100', '00100', '00100', '00100', '00100', '00000', '00100'],
  '?': ['01110', '10001', '00001', '00110', '00100', '00000', '00100'],
  '&': ['01100', '10010', '10100', '01000', '10101', '10010', '01101'],
  '/': ['00001', '00001', '00010', '00100', '01000', '10000', '10000'],
  ':': ['00000', '01100', '01100', '00000', '01100', '01100', '00000'],
  ' ': ['000', '000', '000', '000', '000', '000', '000'],
};

/** 7x7 icons. */
export const ICONS: Record<string, { label: string; rows: string[] }> = {
  none: { label: 'None', rows: [] },
  diamond: { label: 'Diamond', rows: ['0001000', '0011100', '0111110', '1111111', '0111110', '0011100', '0001000'] },
  crown: { label: 'Crown', rows: ['1010101', '1010101', '1111111', '1111111', '0111110', '0111110', '0000000'] },
  plus: { label: 'Plus', rows: ['0011100', '0011100', '1111111', '1111111', '1111111', '0011100', '0011100'] },
  star: { label: 'Star', rows: ['0001000', '0001000', '1111111', '0111110', '0011100', '0110110', '0100010'] },
  heart: { label: 'Heart', rows: ['0110110', '1111111', '1111111', '1111111', '0111110', '0011100', '0001000'] },
  sword: { label: 'Sword', rows: ['0000011', '0000110', '1001100', '0111000', '0110000', '1011000', '1100000'] },
  bolt: { label: 'Bolt', rows: ['0001110', '0011100', '0111000', '1111110', '0011100', '0111000', '0110000'] },
  shield: { label: 'Shield', rows: ['1111111', '1111111', '1111111', '1111111', '0111110', '0011100', '0001000'] },
  skull: { label: 'Skull', rows: ['0111110', '1111111', '1011101', '1111111', '0111110', '0101010', '0101010'] },
  dot: { label: 'Dot', rows: ['0000000', '0011100', '0111110', '0111110', '0111110', '0011100', '0000000'] },
};

export interface PrefixStyle {
  text: string;
  icon: string;
  from: string;
  to: string;
  /** Gradient angle in degrees (90 = left to right). */
  angle: number;
  textColor: string;
  iconColor: string;
  outline: string;
  shadow: string;
  /** Size of one font pixel on the output image. */
  scale: number;
  border: boolean;
}

export const PRESETS: Record<string, PrefixStyle> = {
  Member: { text: 'Member', icon: 'diamond', from: '#14b8e8', to: '#19e07a', angle: 90, textColor: '#ffffff', iconColor: '#7fe9ff', outline: '#10304a', shadow: '#0a1d2e', scale: 3, border: false },
  Founder: { text: 'Founder', icon: 'crown', from: '#8a2be2', to: '#d61fd6', angle: 120, textColor: '#ffffff', iconColor: '#ffe27a', outline: '#2a0a45', shadow: '#16052a', scale: 3, border: false },
  Staff: { text: 'Staff', icon: 'plus', from: '#1b5cf0', to: '#19bff0', angle: 90, textColor: '#ffffff', iconColor: '#d6f4ff', outline: '#0a2a66', shadow: '#061a40', scale: 3, border: false },
  VIP: { text: 'VIP', icon: 'star', from: '#ffd21f', to: '#ff8a00', angle: 90, textColor: '#ffffff', iconColor: '#fff6c2', outline: '#7a3b00', shadow: '#4d2400', scale: 3, border: false },
};

function bitmap(ch: string): string[] {
  return GLYPHS[ch.toUpperCase()] ?? GLYPHS['?'];
}

/** Characters we can't draw are shown as `?`; used to warn in the UI. */
export function unsupported(text: string): string[] {
  return [...new Set([...text].filter((c) => !GLYPHS[c.toUpperCase()]))];
}

function measure(s: PrefixStyle) {
  const sc = s.scale;
  const chars = [...s.text.slice(0, 24)];
  let w = 0;
  chars.forEach((c, i) => { w += bitmap(c)[0].length + (i ? 1 : 0); });
  const hasIcon = s.icon !== 'none' && ICONS[s.icon]?.rows.length;
  const iconW = hasIcon ? 7 + 2 : 0;
  const padX = Math.round(2.5 * sc);
  // In game the badge is drawn 9 pixels tall, so make its height a multiple of 9 and every pixel stays sharp.
  const height = Math.ceil((11 * sc) / 9) * 9;
  const padY = Math.floor((height - 7 * sc) / 2);
  return { sc, chars, textW: w, iconW, hasIcon: !!hasIcon, padX, padY, width: (w + iconW) * sc + padX * 2 + (sc > 1 ? 2 : 0), height };
}

export function renderPrefix(canvas: HTMLCanvasElement, s: PrefixStyle) {
  const m = measure(s);
  canvas.width = Math.max(m.width, 8);
  canvas.height = m.height;
  const g = canvas.getContext('2d')!;
  g.imageSmoothingEnabled = false;

  const a = ((s.angle - 90) * Math.PI) / 180;
  const cx = canvas.width / 2, cy = canvas.height / 2;
  const dx = Math.cos(a) * canvas.width / 2, dy = Math.sin(a) * canvas.width / 2;
  const grad = g.createLinearGradient(cx - dx, cy - dy, cx + dx, cy + dy);
  grad.addColorStop(0, s.from);
  grad.addColorStop(1, s.to);
  g.fillStyle = grad;
  g.fillRect(0, 0, canvas.width, canvas.height);
  if (s.border) {
    g.strokeStyle = 'rgba(255,255,255,0.45)';
    g.lineWidth = 1;
    g.strokeRect(0.5, 0.5, canvas.width - 1, canvas.height - 1);
  }

  // Collect every lit pixel with its colour, then stamp: shadow, outline, fill.
  const lit: { x: number; y: number; color: string }[] = [];
  let x = 0;
  if (m.hasIcon) {
    ICONS[s.icon].rows.forEach((row, y) => [...row].forEach((v, i) => v === '1' && lit.push({ x: i, y, color: s.iconColor })));
    x = 9;
  }
  m.chars.forEach((c, i) => {
    const b = bitmap(c);
    if (i) x += 1;
    b.forEach((row, y) => [...row].forEach((v, k) => v === '1' && lit.push({ x: x + k, y, color: s.textColor })));
    x += b[0].length;
  });

  const ox = Math.round((canvas.width - m.textW * m.sc - m.iconW * m.sc) / 2);
  const oy = m.padY;
  const stamp = (dxp: number, dyp: number, color?: string) => {
    for (const p of lit) {
      g.fillStyle = color ?? p.color;
      g.fillRect(ox + p.x * m.sc + dxp, oy + p.y * m.sc + dyp, m.sc, m.sc);
    }
  };
  const t = Math.max(1, Math.round(m.sc / 3));
  stamp(m.sc > 1 ? Math.ceil(m.sc * 0.66) : 1, m.sc > 1 ? Math.ceil(m.sc * 0.66) : 1, s.shadow);
  for (const [ax, ay] of [[-t, 0], [t, 0], [0, -t], [0, t], [-t, -t], [t, -t], [-t, t], [t, t]]) stamp(ax, ay, s.outline);
  stamp(0, 0);
}

export function renderToFile(s: PrefixStyle): Promise<File> {
  const canvas = document.createElement('canvas');
  renderPrefix(canvas, s);
  return new Promise((resolve, reject) =>
    canvas.toBlob((b) => (b ? resolve(new File([b], `rank-${s.text.toLowerCase().replace(/[^a-z0-9]+/g, '-') || 'badge'}.png`, { type: 'image/png' })) : reject(new Error('Could not make the image'))), 'image/png'),
  );
}

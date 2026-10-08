// Minecraft colour codes (& and &#RRGGBB) to styled spans, for previews that match the game.
export type Span = { text: string; color: string; bold: boolean; italic: boolean; underline: boolean; strike: boolean };

export const COLORS: Record<string, string> = {
  '0': '#000000', '1': '#0000aa', '2': '#00aa00', '3': '#00aaaa', '4': '#aa0000', '5': '#aa00aa', '6': '#ffaa00', '7': '#aaaaaa',
  '8': '#555555', '9': '#5555ff', a: '#55ff55', b: '#55ffff', c: '#ff5555', d: '#ff55ff', e: '#ffff55', f: '#ffffff',
};

export function spans(src: string, base = '#ffffff'): Span[] {
  const out: Span[] = [];
  const fresh = (color = base) => ({ color, bold: false, italic: false, underline: false, strike: false });
  let st = fresh();
  const re = /&#([0-9a-fA-F]{6})|&([0-9a-fk-orA-FK-OR])/g;
  let last = 0, m: RegExpExecArray | null;
  const push = (t: string) => { if (t) out.push({ text: t, ...st }); };
  while ((m = re.exec(src))) {
    push(src.slice(last, m.index)); last = re.lastIndex;
    if (m[1]) st = fresh('#' + m[1]);
    else {
      const c = m[2].toLowerCase();
      if (COLORS[c]) st = fresh(COLORS[c]);
      else if (c === 'l') st = { ...st, bold: true };
      else if (c === 'o') st = { ...st, italic: true };
      else if (c === 'n') st = { ...st, underline: true };
      else if (c === 'm') st = { ...st, strike: true };
      else if (c === 'r') st = fresh();
    }
  }
  push(src.slice(last));
  return out;
}

// Render Minecraft § formatting codes as HTML-safe segments.
const COLORS: Record<string, string> = {
  '0': '#000000', '1': '#0000AA', '2': '#00AA00', '3': '#00AAAA', '4': '#AA0000', '5': '#AA00AA', '6': '#FFAA00', '7': '#AAAAAA',
  '8': '#555555', '9': '#5555FF', a: '#55FF55', b: '#55FFFF', c: '#FF5555', d: '#FF55FF', e: '#FFFF55', f: '#FFFFFF',
};

export interface Segment { text: string; color?: string; bold?: boolean; italic?: boolean; underline?: boolean; strike?: boolean }

export function parseMotd(input: string): Segment[] {
  const out: Segment[] = [];
  let cur: Segment = { text: '' };
  for (let i = 0; i < input.length; i++) {
    const ch = input[i];
    if (ch === '§' && i + 1 < input.length) {
      const code = input[i + 1].toLowerCase();
      if (cur.text) out.push(cur);
      const base: Segment = { ...cur, text: '' };
      if (code === '#' && /^[0-9a-f]{6}$/i.test(input.slice(i + 2, i + 8))) {
        cur = { text: '', color: '#' + input.slice(i + 2, i + 8) };
        i += 7;
        continue;
      }
      if (COLORS[code]) cur = { text: '', color: COLORS[code] };
      else if (code === 'l') cur = { ...base, bold: true };
      else if (code === 'o') cur = { ...base, italic: true };
      else if (code === 'n') cur = { ...base, underline: true };
      else if (code === 'm') cur = { ...base, strike: true };
      else if (code === 'r') cur = { text: '' };
      else cur = base;
      i++;
      continue;
    }
    cur.text += ch;
  }
  if (cur.text) out.push(cur);
  return out;
}

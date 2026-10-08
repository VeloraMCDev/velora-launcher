// Minecraft (1.13+) key names used in options.txt, and friendly labels.

export interface Binding { id: string; label: string; default: string }

export const BIND_GROUPS: { title: string; binds: Binding[] }[] = [
  { title: 'SCOPENET Companion · Fabric 26.3', binds: [
    {id:'key.scopenet.hub',label:'Open SCOPENET',default:'key.keyboard.k'},
    ...[['vaults','Vaults and kits'],['map','World map'],['guild','Guild hall'],['casino','Casino'],['market','Marketplace'],['quests','Quest journal'],['travel','Travel'],['friends','Friends'],['catalog','Content collection'],['hud','HUD preview']].map(([id,label])=>({id:`key.scopenet.${id}`,label,default:'key.keyboard.unknown'})),
  ] },
  {
    title: 'Movement',
    binds: [
      { id: 'key.forward', label: 'Walk forwards', default: 'key.keyboard.w' },
      { id: 'key.left', label: 'Strafe left', default: 'key.keyboard.a' },
      { id: 'key.back', label: 'Walk backwards', default: 'key.keyboard.s' },
      { id: 'key.right', label: 'Strafe right', default: 'key.keyboard.d' },
      { id: 'key.jump', label: 'Jump', default: 'key.keyboard.space' },
      { id: 'key.sneak', label: 'Sneak', default: 'key.keyboard.left.shift' },
      { id: 'key.sprint', label: 'Sprint', default: 'key.keyboard.left.control' },
    ],
  },
  {
    title: 'Gameplay',
    binds: [
      { id: 'key.attack', label: 'Attack / destroy', default: 'key.mouse.left' },
      { id: 'key.use', label: 'Use item / place block', default: 'key.mouse.right' },
      { id: 'key.pickItem', label: 'Pick block', default: 'key.mouse.middle' },
      { id: 'key.drop', label: 'Drop item', default: 'key.keyboard.q' },
      { id: 'key.swapOffhand', label: 'Swap to offhand', default: 'key.keyboard.f' },
    ],
  },
  {
    title: 'Interface',
    binds: [
      { id: 'key.inventory', label: 'Inventory', default: 'key.keyboard.e' },
      { id: 'key.chat', label: 'Open chat', default: 'key.keyboard.t' },
      { id: 'key.command', label: 'Open command', default: 'key.keyboard.slash' },
      { id: 'key.playerlist', label: 'Player list', default: 'key.keyboard.tab' },
      { id: 'key.advancements', label: 'Advancements', default: 'key.keyboard.l' },
      { id: 'key.socialInteractions', label: 'Social interactions', default: 'key.keyboard.p' },
      { id: 'key.screenshot', label: 'Screenshot', default: 'key.keyboard.f2' },
      { id: 'key.togglePerspective', label: 'Toggle perspective', default: 'key.keyboard.f5' },
      { id: 'key.fullscreen', label: 'Toggle fullscreen', default: 'key.keyboard.f11' },
    ],
  },
  {
    title: 'Hotbar',
    binds: Array.from({ length: 9 }, (_, i) => ({ id: `key.hotbar.${i + 1}`, label: `Hotbar slot ${i + 1}`, default: `key.keyboard.${i + 1}` })),
  },
];

const SPECIAL: Record<string, string> = {
  Space: 'space', ShiftLeft: 'left.shift', ShiftRight: 'right.shift', ControlLeft: 'left.control', ControlRight: 'right.control',
  AltLeft: 'left.alt', AltRight: 'right.alt', Tab: 'tab', Enter: 'enter', Backspace: 'backspace', CapsLock: 'caps.lock',
  ArrowUp: 'up', ArrowDown: 'down', ArrowLeft: 'left', ArrowRight: 'right', Backquote: 'grave.accent', Minus: 'minus',
  Equal: 'equal', BracketLeft: 'left.bracket', BracketRight: 'right.bracket', Backslash: 'backslash', Semicolon: 'semicolon',
  Quote: 'apostrophe', Comma: 'comma', Period: 'period', Slash: 'slash', Insert: 'insert', Delete: 'delete', Home: 'home',
  End: 'end', PageUp: 'page.up', PageDown: 'page.down', NumpadAdd: 'keypad.add', NumpadSubtract: 'keypad.subtract',
  NumpadMultiply: 'keypad.multiply', NumpadDivide: 'keypad.divide', NumpadEnter: 'keypad.enter', NumpadDecimal: 'keypad.decimal',
};

/** KeyboardEvent.code → Minecraft key name (null for unsupported keys). */
export function fromKeyboard(code: string): string | null {
  if (/^Key[A-Z]$/.test(code)) return `key.keyboard.${code.slice(3).toLowerCase()}`;
  if (/^Digit\d$/.test(code)) return `key.keyboard.${code.slice(5)}`;
  if (/^F\d{1,2}$/.test(code)) return `key.keyboard.${code.toLowerCase()}`;
  if (/^Numpad\d$/.test(code)) return `key.keyboard.keypad.${code.slice(6)}`;
  return SPECIAL[code] ? `key.keyboard.${SPECIAL[code]}` : null;
}

export function fromMouse(button: number): string {
  return ['key.mouse.left', 'key.mouse.middle', 'key.mouse.right', 'key.mouse.4', 'key.mouse.5'][button] ?? `key.mouse.${button + 1}`;
}

/** Human label for a Minecraft key name. */
export function keyLabel(key: string): string {
  if (!key || key === 'key.keyboard.unknown') return 'Not bound';
  const mouse = key.match(/^key\.mouse\.(.+)$/);
  if (mouse) return { left: 'Left click', right: 'Right click', middle: 'Middle click' }[mouse[1]] ?? `Mouse ${mouse[1]}`;
  const k = key.replace('key.keyboard.', '');
  const names: Record<string, string> = {
    'left.shift': 'Left Shift', 'right.shift': 'Right Shift', 'left.control': 'Left Ctrl', 'right.control': 'Right Ctrl',
    'left.alt': 'Left Alt', 'right.alt': 'Right Alt', 'caps.lock': 'Caps Lock', 'page.up': 'Page Up', 'page.down': 'Page Down',
    'grave.accent': '`', 'left.bracket': '[', 'right.bracket': ']', slash: '/', backslash: '\\', semicolon: ';', apostrophe: "'",
    comma: ',', period: '.', minus: '-', equal: '=', up: '↑', down: '↓', left: '←', right: '→',
  };
  if (names[k]) return names[k];
  if (k.startsWith('keypad.')) return `Num ${k.slice(7)}`;
  return k.length === 1 ? k.toUpperCase() : k.charAt(0).toUpperCase() + k.slice(1);
}

export function defaults(): Record<string, string> {
  return Object.fromEntries(BIND_GROUPS.flatMap((g) => g.binds.map((b) => [b.id, b.default])));
}

// Svelte runes ($state, $derived, $effect…) only compile in .svelte and .svelte.ts files. In a plain .ts file they throw at
// import time and the whole app renders as an empty window, which svelte-check does not catch. Run by `npm run check:runes`.
import { readdirSync, readFileSync, statSync } from 'node:fs';
import { join } from 'node:path';

const bad = [];
(function walk(dir) {
  for (const name of readdirSync(dir)) {
    const path = join(dir, name);
    if (statSync(path).isDirectory()) walk(path);
    else if (/\.(ts|js)$/.test(name) && !/\.svelte\.(ts|js)$/.test(name) && /\$(state|derived|effect|props|bindable)\b/.test(readFileSync(path, 'utf8'))) bad.push(path);
  }
})('src');
if (bad.length) {
  console.error('Runes used outside .svelte / .svelte.ts files (rename them to *.svelte.ts):\n' + bad.join('\n'));
  process.exit(1);
}

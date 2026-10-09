// Packs the Iconify icon sets for the panel: `npm install && node build.mjs` writes dist/<pack>.json.gz and dist/packs.json.
// The panel reads them from VELORA_ICONS_DIR (default ./panel/icons/dist). Icons stay on the server; clients fetch one SVG at a time.
import { readdirSync, readFileSync, writeFileSync, mkdirSync, rmSync } from 'node:fs';
import { gzipSync } from 'node:zlib';
import { join, dirname } from 'node:path';
import { fileURLToPath } from 'node:url';

const here = dirname(fileURLToPath(import.meta.url));
const root = join(here, 'node_modules', '@iconify-json');
const out = join(here, 'dist');
rmSync(out, { recursive: true, force: true });
mkdirSync(out, { recursive: true });

const packs = [];
for (const id of readdirSync(root).sort()) {
  const raw = JSON.parse(readFileSync(join(root, id, 'icons.json'), 'utf8'));
  const slim = { prefix: raw.prefix, width: raw.width, height: raw.height, icons: {}, aliases: {} };
  for (const [name, icon] of Object.entries(raw.icons)) {
    const { body, width, height, left, top } = icon;
    slim.icons[name] = { body, ...(width && { width }), ...(height && { height }), ...(left && { left }), ...(top && { top }) };
  }
  for (const [name, alias] of Object.entries(raw.aliases ?? {})) if (!alias.hFlip && !alias.vFlip && !alias.rotate) slim.aliases[name] = { parent: alias.parent };
  writeFileSync(join(out, `${id}.json.gz`), gzipSync(JSON.stringify(slim), { level: 9 }));
  let info = raw.info ?? {};
  try { info = JSON.parse(readFileSync(join(root, id, 'info.json'), 'utf8')); } catch {}
  packs.push({
    id,
    label: info.name ?? id,
    palette: !!info.palette,
    license: info.license?.spdx ?? info.license?.title ?? '',
    homepage: info.author?.url ?? '',
    total: Object.keys(slim.icons).length + Object.keys(slim.aliases).length,
  });
}
writeFileSync(join(out, 'packs.json'), JSON.stringify(packs));
console.log(`${packs.length} packs, ${packs.reduce((n, p) => n + p.total, 0)} icons`);

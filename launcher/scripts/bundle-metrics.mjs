import { readFileSync, readdirSync, statSync } from 'node:fs';
import { resolve, relative, dirname } from 'node:path';
import { pathToFileURL } from 'node:url';
import { gzipSync } from 'node:zlib';

export function bundleMetrics(directory) {
  const root = resolve(directory);
  const html = readFileSync(resolve(root, 'index.html'), 'utf8');
  const entry = html.match(/<script\b[^>]*\bsrc="([^"]+\.js)"/);
  if (!entry) throw Error('No JavaScript entry in build output');
  const resolveAsset = asset => {
    const filename = resolve(root, asset.replace(/^\//, ''));
    const within = relative(root, filename);
    if (within.startsWith('..') || !within) throw Error('Build asset escapes output directory');
    return filename;
  };
  const entryPath = resolveAsset(entry[1]);
  const staticAssets = new Set([entryPath]);
  for (const match of html.matchAll(/<link\b[^>]*rel="modulepreload"[^>]*href="([^"]+)"/g)) staticAssets.add(resolveAsset(match[1]));
  const manifestPath = resolve(root, '.vite/manifest.json');
  let manifest;
  try { manifest = JSON.parse(readFileSync(manifestPath, 'utf8')); }
  catch (error) { if (error.code !== 'ENOENT') throw error; }
  if (manifest) {
    const visited = new Set();
    const collect = key => {
      if (visited.has(key)) return;
      visited.add(key);
      const chunk = manifest[key];
      if (!chunk) throw Error('Missing static manifest import');
      if (chunk.file.endsWith('.js')) staticAssets.add(resolveAsset(chunk.file));
      for (const dependency of chunk.imports ?? []) collect(dependency);
    };
    for (const [key, chunk] of Object.entries(manifest)) if (chunk.isEntry) collect(key);
  }
  const assetDirectory = dirname(entryPath);
  const javascript = readdirSync(assetDirectory).filter(name => name.endsWith('.js')).map(name => resolve(assetDirectory, name));
  const sum = files => files.reduce((result, filename) => {
    const content = readFileSync(filename);
    return { raw: result.raw + content.length, gzip: result.gzip + gzipSync(content).length };
  }, { raw: 0, gzip: 0 });
  return {
    entry: { file: relative(root, entryPath), ...sum([entryPath]) },
    initialJavaScript: { files: staticAssets.size, ...sum([...staticAssets]) },
    allJavaScript: { files: javascript.length, ...sum(javascript) },
    deferredJavaScript: javascript.filter(filename => !staticAssets.has(filename)).map(filename => ({ file: relative(root, filename), bytes: statSync(filename).size })),
    graphScope: manifest ? 'Vite manifest static-import closure; excludes deferred imports' : 'HTML entry and modulepreloads; manifest unavailable',
  };
}

if (process.argv[1] && import.meta.url === pathToFileURL(resolve(process.argv[1])).href) {
  console.log(JSON.stringify(bundleMetrics(process.argv[2] ?? 'dist'), null, 2));
}

import { readFileSync, readdirSync, statSync } from 'node:fs';
import { resolve, relative, dirname, isAbsolute } from 'node:path';
import { pathToFileURL } from 'node:url';

export function checkLinks(root) {
  root = resolve(root);
  const files = [];
  function walk(directory) {
    for (const entry of readdirSync(directory, { withFileTypes:true })) {
      if (['.git','node_modules','dist','target','.runtime-checks','.wrangler','.gradle','build','.svelte-kit'].includes(entry.name)) continue;
      const path = resolve(directory, entry.name);
      if (entry.isDirectory()) walk(path);
      else if (entry.isFile() && path.endsWith('.md')) files.push(path);
    }
  }
  walk(root);
  const failures = [];
  let checked = 0;
  for (const file of files) {
    const text = readFileSync(file, 'utf8').replace(/(^|\n)[ \t]*(`{3,}|~{3,})[^\n]*\n[\s\S]*?\n[ \t]*\2[ \t]*(?=\n|$)/g, '\n');
    for (const match of text.matchAll(/\[[^\]]*\]\((<[^>]+>|[^)\s]+)(?:\s+[^)]*)?\)/g)) {
      const link = match[1].replace(/^<|>$/g, '');
      if (/^(?:[a-z][a-z0-9+.-]*:|#)/i.test(link)) continue;
      checked++;
      let target;
      try { target = resolve(dirname(file), decodeURIComponent(link.split('#')[0])); }
      catch { failures.push({ file:relative(root,file), reason:'invalid link encoding' }); continue; }
      const rel = relative(root,target);
      if (isAbsolute(rel) || rel === '..' || rel.startsWith('../') || rel.startsWith('..\\')) {
        failures.push({ file:relative(root,file), reason:'link escapes repository' }); continue;
      }
      try { if (!statSync(target).isFile() && !statSync(target).isDirectory()) throw Error(); }
      catch { failures.push({ file:relative(root,file), reason:`missing local target: ${rel.replaceAll('\\','/')}` }); }
    }
  }
  return { markdown_files:files.length, local_links_checked:checked, failures, scope:'Local Markdown link targets; external URLs, HTML links and heading anchors are not certified.' };
}

if (process.argv[1] && import.meta.url === pathToFileURL(process.argv[1]).href) {
  const result = checkLinks(process.argv[2] ?? '.');
  console.log(JSON.stringify(result,null,2));
  if (result.failures.length) process.exitCode = 1;
}

import { readFileSync, writeFileSync, mkdirSync, readdirSync, lstatSync, rmSync, existsSync } from 'node:fs';
import { resolve, relative, dirname, posix } from 'node:path';
import { createHash } from 'node:crypto';
import { execFileSync } from 'node:child_process';
import { pathToFileURL } from 'node:url';
import { marked } from 'marked';
import sanitizeHtml from 'sanitize-html';
import { checkLinks } from './check-links.mjs';

const hash = bytes => createHash('sha256').update(bytes).digest('hex');
const escape = text => text.replaceAll('&','&amp;').replaceAll('<','&lt;').replaceAll('>','&gt;').replaceAll('"','&quot;');
const destination = path => path === 'README.md' ? 'index.html' : path === '.github/assets' ? 'assets/README.html' : path.replace(/^\.github\/assets\//,'assets/').replace(/\.md$/,'.html');
const plain = html => sanitizeHtml(html,{allowedTags:[],allowedAttributes:{}}).replaceAll('&amp;','&').replaceAll('&lt;','<').replaceAll('&gt;','>').replaceAll('&quot;','"').replaceAll('&#39;',"'").replace(/\s+/g,' ').trim();
const REPOSITORY = 'VeloraMCDev/velora-launcher';

/** Renders sanitized HTML and records the page's headings for navigation and search. */
export function renderPage(markdown, sourcePath) {
  const rewrite = value => {
    if (/^(?:[a-z][a-z0-9+.-]*:|#|\/\/)/i.test(value)) return value;
    const [path, fragment] = value.split('#');
    const target = posix.normalize(posix.join(posix.dirname(sourcePath), path));
    return '/' + destination(target) + (fragment ? '#' + fragment : '');
  };
  const headings = [];
  const seen = new Map();
  const renderer = new marked.Renderer();
  renderer.heading = function({ tokens, depth }) {
    const text = this.parser.parseInline(tokens);
    const base = sanitizeHtml(text,{allowedTags:[],allowedAttributes:{}}).toLowerCase().replace(/[^\p{L}\p{N}\s_-]/gu,'').replace(/\s/g,'-');
    const count = seen.get(base) ?? 0;
    seen.set(base,count + 1);
    const id = base + (count ? '-' + count : '');
    headings.push({ depth, id, text: plain(text) });
    const anchor = depth > 1 ? `<a class="anchor" href="#${escape(id)}" aria-label="Link to this section">#</a>` : '';
    return `<h${depth} id="${escape(id)}">${text}${anchor}</h${depth}>\n`;
  };
  const html = sanitizeHtml(marked.parse(markdown,{renderer}), {
    allowedTags: [...sanitizeHtml.defaults.allowedTags,'img'],
    allowedAttributes: { ...sanitizeHtml.defaults.allowedAttributes, h1:['id'],h2:['id'],h3:['id'],h4:['id'],h5:['id'],h6:['id'], img:['src','alt','width','height'],code:['class'], a:['href','name','target','class','aria-label'] },
    allowedSchemes: ['https','mailto'],
    allowProtocolRelative: false,
    transformTags: {
      a: (_,attributes) => ({tagName:'a',attribs:{...attributes,href:rewrite(attributes.href ?? '')}}),
      img: (_,attributes) => ({tagName:'img',attribs:{...attributes,src:rewrite(attributes.src ?? '')}})
    }
  });
  return { html, headings };
}

export function renderMarkdown(markdown, sourcePath) {
  return renderPage(markdown, sourcePath).html;
}

/** Splits a rendered page into heading-scoped records for the client-side search index. */
export function searchRecords(page) {
  const records = [];
  const parts = page.html.split(/(?=<h[1-3] id=")/);
  for (const part of parts) {
    const match = part.match(/^<h([1-3]) id="([^"]+)">([\s\S]*?)<\/h\1>/);
    const heading = match ? plain(match[3].replace(/<a class="anchor"[\s\S]*?<\/a>/,'')) : page.title;
    const text = plain(match ? part.slice(match[0].length) : part);
    if (!text && !match) continue;
    const anchor = match && match[1] !== '1' ? '#' + match[2] : '';
    records.push({ u: '/' + page.dest + anchor, p: page.title, h: heading, t: text, s: page.section });
  }
  return records;
}

const SECTIONS = [
  ['Start here', p => !p.includes('/') ],
  ['Deployment', p => p.startsWith('deployment/')],
  ['Security', p => p.startsWith('security/')],
  ['Authentication', p => p.startsWith('components/authentication/')],
  ['Panel components', p => p.startsWith('components/panel/')],
  ['Components', p => p.startsWith('components/')],
  ['Presentation', () => true],
];
const ORDER = ['README.md','APPLICATION.md','BOUNDARIES.md','VALIDATION.md','CONTRIBUTING.md','DEPLOYMENT_TARGETS.md','KNOWN_ISSUES.md','deployment/README.md'];
const sectionOf = path => SECTIONS.find(([, test]) => test(path))[0];

const css = `:root{color-scheme:dark;--bg:#0d0d17;--panel:#14131f;--soft:#1b1a29;--line:#2c2a40;--text:#dddbe8;--muted:#9c98b3;--head:#fff;--accent:#a98bff;--accent-soft:#a98bff22;--mark:#a98bff40;font:16px/1.7 Inter,ui-sans-serif,system-ui,-apple-system,"Segoe UI",sans-serif}
@media (prefers-color-scheme:light){:root{color-scheme:light;--bg:#fbfaff;--panel:#fff;--soft:#f3f1fb;--line:#e3e0f0;--text:#2b2838;--muted:#6b6780;--head:#14111f;--accent:#6c47e6;--accent-soft:#6c47e614;--mark:#6c47e630}}
*{box-sizing:border-box}html{scroll-padding-top:84px}body{margin:0;background:var(--bg);color:var(--text)}a{color:var(--accent);text-underline-offset:3px}
.top{position:sticky;top:0;z-index:20;display:flex;align-items:center;gap:16px;height:64px;padding:0 20px;background:color-mix(in srgb,var(--bg) 85%,transparent);backdrop-filter:blur(12px);border-bottom:1px solid var(--line)}
.brand{display:flex;align-items:center;gap:10px;color:var(--head);font-weight:700;letter-spacing:-.01em;text-decoration:none;white-space:nowrap}.brand i{width:26px;height:26px;border-radius:8px;background:linear-gradient(135deg,#8f6bff,#e46fb7);display:inline-block}.brand small{color:var(--muted);font-weight:500}
.search{flex:1;max-width:520px;margin-left:auto;position:relative}.search input{width:100%;height:40px;padding:0 70px 0 14px;border-radius:10px;border:1px solid var(--line);background:var(--soft);color:var(--text);font:inherit;font-size:15px;outline:none}.search input:focus{border-color:var(--accent);box-shadow:0 0 0 3px var(--accent-soft)}
.search input::-webkit-search-cancel-button{display:none}.search input:not(:placeholder-shown)+kbd{display:none}.search kbd{position:absolute;right:10px;top:9px;font:12px ui-monospace,monospace;color:var(--muted);border:1px solid var(--line);border-radius:6px;padding:1px 6px;background:var(--panel)}
.results{position:absolute;top:48px;left:0;right:0;max-height:min(70vh,560px);overflow:auto;background:var(--panel);border:1px solid var(--line);border-radius:12px;box-shadow:0 18px 50px #0007;padding:6px;display:none}.results.open{display:block}
.results a{display:block;padding:10px 12px;border-radius:8px;text-decoration:none;color:var(--text)}.results a:hover,.results a.active{background:var(--accent-soft)}.results b{display:block;color:var(--head);font-weight:600}.results small{color:var(--muted);font-size:12px}.results p{margin:4px 0 0;font-size:14px;color:var(--muted);line-height:1.5}.results mark{background:var(--mark);color:inherit;border-radius:3px;padding:0 1px}.results .empty{padding:14px;color:var(--muted)}
.menu{display:none;background:none;border:1px solid var(--line);color:var(--text);border-radius:8px;height:36px;width:40px;font-size:18px;cursor:pointer}
.layout{display:grid;grid-template-columns:270px minmax(0,1fr) 220px;max-width:1440px;margin:0 auto}
nav.side{position:sticky;top:64px;height:calc(100vh - 64px);overflow:auto;padding:24px 16px 40px 20px;border-right:1px solid var(--line)}nav.side h4{margin:22px 0 6px;font-size:12px;text-transform:uppercase;letter-spacing:.08em;color:var(--muted)}nav.side h4:first-child{margin-top:0}
nav.side a{display:block;padding:5px 10px;border-radius:7px;color:var(--text);text-decoration:none;font-size:14.5px;line-height:1.45}nav.side a:hover{background:var(--soft)}nav.side a[aria-current]{background:var(--accent-soft);color:var(--accent);font-weight:600}
main{min-width:0;padding:40px 48px 80px}article{max-width:780px}.crumb{font-size:13px;color:var(--muted);margin-bottom:8px}
h1,h2,h3,h4{color:var(--head);line-height:1.25;letter-spacing:-.015em}h1{font-size:2.3rem;margin:.2em 0 .6em}h2{font-size:1.5rem;margin-top:2.2em;padding-bottom:10px;border-bottom:1px solid var(--line)}h3{font-size:1.15rem;margin-top:1.8em}
.anchor{margin-left:8px;opacity:0;text-decoration:none;color:var(--muted)}h2:hover .anchor,h3:hover .anchor,h4:hover .anchor{opacity:1}
img{max-width:100%;height:auto;border-radius:10px}pre{overflow:auto;padding:16px 18px;background:var(--soft);border:1px solid var(--line);border-radius:10px;line-height:1.55}code{font-family:"JetBrains Mono",ui-monospace,monospace;font-size:.87em}:not(pre)>code{background:var(--soft);border:1px solid var(--line);padding:1px 6px;border-radius:6px}
table{display:block;overflow:auto;border-collapse:collapse;margin:1.4em 0;font-size:15px}td,th{padding:9px 12px;border:1px solid var(--line);text-align:left;vertical-align:top}th{background:var(--soft);color:var(--head)}
blockquote{margin:1.4em 0;border-left:3px solid var(--accent);padding:2px 18px;color:var(--muted);background:var(--soft);border-radius:0 8px 8px 0}
aside.toc{position:sticky;top:64px;height:calc(100vh - 64px);overflow:auto;padding:40px 20px 40px 0;font-size:13.5px}aside.toc b{display:block;color:var(--muted);font-size:12px;text-transform:uppercase;letter-spacing:.08em;margin-bottom:8px}aside.toc a{display:block;color:var(--muted);text-decoration:none;padding:3px 0;line-height:1.4}aside.toc a:hover{color:var(--accent)}aside.toc a.d3{padding-left:12px}
.pager{display:flex;gap:12px;justify-content:space-between;margin-top:56px}.pager a{flex:1;border:1px solid var(--line);border-radius:12px;padding:14px 16px;text-decoration:none;color:var(--head);font-weight:600}.pager a:hover{border-color:var(--accent)}.pager small{display:block;color:var(--muted);font-weight:500}.pager .next{text-align:right}
footer{margin-top:48px;padding-top:20px;border-top:1px solid var(--line);color:var(--muted);font-size:13px;overflow-wrap:anywhere}footer a{color:var(--muted)}
@media(max-width:1180px){.layout{grid-template-columns:250px minmax(0,1fr)}aside.toc{display:none}}
@media(max-width:820px){.layout{display:block}.menu{display:inline-block}nav.side{display:none;position:fixed;inset:64px 0 0 0;z-index:15;background:var(--bg);height:auto}body.nav-open nav.side{display:block}main{padding:28px 18px 64px}.brand small{display:none}h1{font-size:1.8rem}}`;

const js = `(() => {
  const input = document.getElementById('q'), box = document.getElementById('results');
  let index = null, active = -1, items = [];
  const load = () => index ?? (index = fetch('/search-index.json').then(r => r.json()));
  const words = q => q.toLowerCase().split(/[^\\p{L}\\p{N}_]+/u).filter(w => w.length > 1);
  const esc = s => s.replace(/[&<>"]/g, c => ({'&':'&amp;','<':'&lt;','>':'&gt;','"':'&quot;'}[c]));
  const mark = (text, terms) => { let html = esc(text); for (const t of terms) html = html.replace(new RegExp('(' + t.replace(/[.*+?^\${}()|[\\]\\\\]/g, '\\\\$&') + ')', 'gi'), '<mark>$1</mark>'); return html; };
  function score(r, terms) {
    let total = 0; const h = r.h.toLowerCase(), p = r.p.toLowerCase(), t = r.t.toLowerCase();
    for (const term of terms) {
      let s = 0;
      if (h.includes(term)) s += h.split(/\\W+/).includes(term) ? 12 : 8;
      if (p.includes(term)) s += 5;
      const at = t.indexOf(term); if (at >= 0) s += 2 + Math.min(4, t.split(term).length - 1);
      if (!s) return 0; total += s;
    }
    return total;
  }
  function snippet(text, terms) {
    const lower = text.toLowerCase(); const at = Math.max(0, Math.min(...terms.map(t => { const i = lower.indexOf(t); return i < 0 ? 1e9 : i; })) - 60);
    const cut = at > 1e8 ? 0 : at; return (cut ? '…' : '') + text.slice(cut, cut + 180) + (text.length > cut + 180 ? '…' : '');
  }
  function select(i) { active = i; [...box.querySelectorAll('a')].forEach((a, n) => a.classList.toggle('active', n === i)); box.querySelectorAll('a')[i]?.scrollIntoView({block: 'nearest'}); }
  async function run() {
    const q = input.value.trim(), terms = words(q);
    if (!terms.length) { box.classList.remove('open'); return; }
    const data = await load();
    items = data.map(r => [score(r, terms), r]).filter(([s]) => s).sort((a, b) => b[0] - a[0]).slice(0, 12).map(([, r]) => r);
    box.innerHTML = items.length ? items.map(r => '<a href="' + esc(r.u) + '"><small>' + esc(r.s + ' › ' + r.p) + '</small><b>' + mark(r.h, terms) + '</b><p>' + mark(snippet(r.t, terms), terms) + '</p></a>').join('') : '<div class="empty">No results for “' + esc(q) + '”</div>';
    box.classList.add('open'); select(items.length ? 0 : -1);
  }
  input.addEventListener('focus', () => { load(); if (input.value) run(); });
  input.addEventListener('input', run);
  input.addEventListener('keydown', e => {
    if (e.key === 'ArrowDown') { e.preventDefault(); select(Math.min(active + 1, items.length - 1)); }
    else if (e.key === 'ArrowUp') { e.preventDefault(); select(Math.max(active - 1, 0)); }
    else if (e.key === 'Enter' && items[active]) location.href = items[active].u;
    else if (e.key === 'Escape') { input.blur(); box.classList.remove('open'); }
  });
  document.addEventListener('keydown', e => {
    if ((e.key === '/' && !/input|textarea/i.test(document.activeElement.tagName)) || (e.key.toLowerCase() === 'k' && (e.ctrlKey || e.metaKey))) { e.preventDefault(); input.focus(); input.select(); }
  });
  document.addEventListener('click', e => { if (!e.target.closest('.search')) box.classList.remove('open'); });
  document.querySelector('.menu')?.addEventListener('click', () => document.body.classList.toggle('nav-open'));
})();`;

function layout({ page, nav, commit, prev, next }) {
  const toc = page.headings.filter(h => h.depth === 2 || h.depth === 3);
  const sideNav = nav.map(([section, pages]) => `<h4>${escape(section)}</h4>` + pages.map(p =>
    `<a href="/${p.dest}"${p.dest === page.dest ? ' aria-current="page"' : ''}>${escape(p.title)}</a>`).join('')).join('');
  const pager = (prev || next) ? `<div class="pager">${prev ? `<a href="/${prev.dest}"><small>Previous</small>${escape(prev.title)}</a>` : '<span></span>'}${next ? `<a class="next" href="/${next.dest}"><small>Next</small>${escape(next.title)}</a>` : '<span></span>'}</div>` : '';
  return `<!doctype html>
<html lang="en"><head><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1"><title>${escape(page.title)} · Velora</title><meta name="description" content="${escape(page.description)}"><link rel="icon" href="/assets/favicon.svg" type="image/svg+xml"><link rel="stylesheet" href="/assets/docs.css"><script src="/assets/search.js" defer></script></head>
<body><header class="top"><button class="menu" aria-label="Menu">☰</button><a class="brand" href="/"><i></i>Velora <small>Documentation</small></a>
<div class="search"><input id="q" type="search" placeholder="Search the documentation" autocomplete="off" aria-label="Search"><kbd>Ctrl K</kbd><div id="results" class="results" role="listbox"></div></div></header>
<div class="layout"><nav class="side" aria-label="Documentation">${sideNav}</nav>
<main><article><div class="crumb">${escape(page.section)}</div>${page.html}${pager}
<footer>Built from <a href="https://github.com/${REPOSITORY}/tree/${commit}">${commit.slice(0, 12)}</a> · <a href="https://github.com/${REPOSITORY}/blob/main/docs/${page.source}">Edit this page on GitHub</a> · Deployment status is recorded in the deployment guide.</footer></article></main>
${toc.length > 1 ? `<aside class="toc"><b>On this page</b>${toc.map(h => `<a class="d${h.depth}" href="#${escape(h.id)}">${escape(h.text)}</a>`).join('')}</aside>` : '<aside class="toc"></aside>'}</div></body></html>\n`;
}

export function buildSite(root, output, commit, { siteUrl } = {}) {
  root = resolve(root); output = resolve(output);
  if (output !== resolve(root,'dist')) throw Error('Output must be the repository dist directory');
  if (!/^[a-f0-9]{40}$/.test(commit)) throw Error('A full source commit is required');
  if (siteUrl !== undefined && !/^https:\/\/[a-z0-9.-]+$/i.test(siteUrl)) throw Error('Site URL must be an https origin');
  const links = checkLinks(root);
  if (links.failures.length) throw Error(`Broken documentation links: ${JSON.stringify(links.failures)}`);
  const sources = [];
  const walk = directory => {
    for (const entry of readdirSync(directory,{withFileTypes:true})) {
      if (['.git','node_modules','dist','target','scripts','tests'].includes(entry.name)) continue;
      const path = resolve(directory,entry.name);
      if (entry.isDirectory()) {
        if (!entry.name.startsWith('.')) walk(path);
      } else if (entry.isFile() && entry.name.endsWith('.md') && entry.name !== 'AGENTS.md') sources.push(path);
    }
  };
  walk(root);
  const assetRoot = resolve(root,'.github/assets');
  if (existsSync(assetRoot)) for (const entry of readdirSync(assetRoot,{withFileTypes:true})) {
    if (entry.isFile() && /^(README\.md|banner\.(png|svg)|manifest\.json)$/.test(entry.name)) sources.push(resolve(assetRoot,entry.name));
  }
  if (existsSync(output)) {
    if (lstatSync(output).isSymbolicLink()) throw Error('Refusing symlink output');
    rmSync(output,{recursive:true});
  }
  mkdirSync(output,{recursive:true});
  const files = [];
  const write = (path,content) => {
    const target = resolve(output,path);
    if (relative(output,target).startsWith('..')) throw Error('Output path escapes dist');
    mkdirSync(dirname(target),{recursive:true});
    writeFileSync(target,content);
    const bytes = readFileSync(target);
    files.push({path,size:bytes.length,sha256:hash(bytes)});
  };
  const pages = [];
  for (const source of sources.sort()) {
    const path = relative(root,source).replaceAll('\\','/');
    if (path.endsWith('.md')) {
      const markdown = readFileSync(source,'utf8');
      const { html, headings } = renderPage(markdown,path);
      const title = headings.find(h => h.depth === 1)?.text ?? 'Velora Documentation';
      const description = plain(html.replace(/<h1[\s\S]*?<\/h1>/,'')).slice(0, 160);
      pages.push({ source: path, dest: destination(path), title, description, html, headings, section: sectionOf(path) });
    } else write(destination(path),path.endsWith('.json') ? readFileSync(source,'utf8').replaceAll('\r\n','\n') : readFileSync(source));
  }
  const rank = p => { const i = ORDER.indexOf(p.source); return i < 0 ? 100 : i; };
  pages.sort((a, b) => SECTIONS.findIndex(([s]) => s === a.section) - SECTIONS.findIndex(([s]) => s === b.section) || rank(a) - rank(b) || a.title.localeCompare(b.title,'en'));
  const nav = SECTIONS.map(([section]) => [section, pages.filter(p => p.section === section)]).filter(([, list]) => list.length);
  pages.forEach((page, i) => write(page.dest, layout({ page, nav, commit, prev: pages[i - 1], next: pages[i + 1] })));
  write('search-index.json', JSON.stringify(pages.flatMap(searchRecords)) + '\n');
  if (siteUrl) {
    write('sitemap.xml', `<?xml version="1.0" encoding="UTF-8"?>\n<urlset xmlns="http://www.sitemaps.org/schemas/sitemap/0.9">\n${pages.map(p => `  <url><loc>${escape(siteUrl + '/' + (p.dest === 'index.html' ? '' : p.dest))}</loc></url>`).join('\n')}\n</urlset>\n`);
    write('robots.txt', `User-agent: *\nAllow: /\nSitemap: ${siteUrl}/sitemap.xml\n`);
  }
  if (existsSync(resolve(root,'LICENSE'))) write('LICENSE',readFileSync(resolve(root,'LICENSE'),'utf8').replaceAll('\r\n','\n'));
  write('assets/docs.css',css + '\n');
  write('assets/search.js',js + '\n');
  write('assets/favicon.svg','<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 32 32"><defs><linearGradient id="g" x2="1" y2="1"><stop stop-color="#8f6bff"/><stop offset="1" stop-color="#e46fb7"/></linearGradient></defs><rect width="32" height="32" rx="9" fill="url(#g)"/></svg>\n');
  write('_headers',`/*\n  X-Content-Type-Options: nosniff\n  Referrer-Policy: strict-origin-when-cross-origin\n  X-Frame-Options: DENY\n  Content-Security-Policy: default-src 'self'; script-src 'self'; style-src 'self'; img-src 'self' https:; connect-src 'self'; frame-ancestors 'none'; base-uri 'none'; form-action 'none'\n`);
  files.sort((a,b) => a.path.localeCompare(b.path,'en'));
  const manifest = {schema_version:1,repository:REPOSITORY,commit_sha:commit,artifact_type:'static_bundle',files,content_manifest_sha256:hash(JSON.stringify(files))};
  writeFileSync(resolve(output,'artifact-manifest.json'),JSON.stringify(manifest,null,2) + '\n');
  return manifest;
}

if (process.argv[1] && import.meta.url === pathToFileURL(process.argv[1]).href) {
  const commit = process.env.GITHUB_SHA ?? execFileSync('git',['rev-parse','HEAD'],{encoding:'utf8'}).trim();
  const siteUrl = process.env.DOCS_SITE_URL || undefined;
  const manifest = buildSite('.', './dist',commit,{ siteUrl });
  console.log(JSON.stringify({commit_sha:commit,files:manifest.files.length,content_manifest_sha256:manifest.content_manifest_sha256}));
}

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

export function renderMarkdown(markdown, sourcePath) {
  const rewrite = value => {
    if (/^(?:[a-z][a-z0-9+.-]*:|#|\/\/)/i.test(value)) return value;
    const [path, fragment] = value.split('#');
    const target = posix.normalize(posix.join(posix.dirname(sourcePath), path));
    return '/' + destination(target) + (fragment ? '#' + fragment : '');
  };
  const headings = new Map();
  const renderer = new marked.Renderer();
  renderer.heading = function({ tokens, depth }) {
    const text = this.parser.parseInline(tokens);
    const base = sanitizeHtml(text,{allowedTags:[],allowedAttributes:{}}).toLowerCase().replace(/[^\p{L}\p{N}\s_-]/gu,'').replace(/\s/g,'-');
    const count = headings.get(base) ?? 0;
    headings.set(base,count + 1);
    return `<h${depth} id="${escape(base + (count ? '-' + count : ''))}">${text}</h${depth}>\n`;
  };
  return sanitizeHtml(marked.parse(markdown,{renderer}), {
    allowedTags: [...sanitizeHtml.defaults.allowedTags,'img'],
    allowedAttributes: { ...sanitizeHtml.defaults.allowedAttributes, h1:['id'],h2:['id'],h3:['id'],h4:['id'],h5:['id'],h6:['id'], img:['src','alt','width','height'],code:['class'] },
    allowedSchemes: ['https','mailto'],
    allowProtocolRelative: false,
    transformTags: {
      a: (_,attributes) => ({tagName:'a',attribs:{...attributes,href:rewrite(attributes.href ?? '')}}),
      img: (_,attributes) => ({tagName:'img',attribs:{...attributes,src:rewrite(attributes.src ?? '')}})
    }
  });
}

const css = `:root{color-scheme:dark;font:16px/1.65 system-ui,sans-serif;background:#10101c;color:#e4e4ef}*{box-sizing:border-box}body{margin:0}header{border-bottom:1px solid #39334e;padding:16px 24px;display:flex;gap:24px;flex-wrap:wrap}header strong{color:#fff}a{color:#c7afff;text-underline-offset:3px}main{max-width:980px;margin:auto;padding:32px 24px 72px}h1,h2,h3{line-height:1.25;color:#fff;scroll-margin-top:24px}h2{margin-top:2em;border-bottom:1px solid #39334e;padding-bottom:12px}img{max-width:100%;height:auto}pre{overflow:auto;padding:18px;background:#191726;border:1px solid #39334e;border-radius:8px}code{font-family:ui-monospace,monospace;font-size:.9em}table{display:block;overflow:auto;border-collapse:collapse}td,th{padding:10px;border:1px solid #39334e;text-align:left}blockquote{border-left:3px solid #9370db;padding-left:20px;color:#b8b4c8}footer{border-top:1px solid #39334e;padding:20px 24px;color:#aaa4bc;font-size:13px;overflow-wrap:anywhere}.notice{background:#241d38;padding:12px 16px;border-left:3px solid #9370db}@media(max-width:600px){main{padding:24px 16px}header{gap:12px;font-size:14px}}`;

export function buildSite(root, output, commit) {
  root = resolve(root); output = resolve(output);
  if (output !== resolve(root,'dist')) throw Error('Output must be the repository dist directory');
  if (!/^[a-f0-9]{40}$/.test(commit)) throw Error('A full source commit is required');
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
  for (const source of sources.sort()) {
    const path = relative(root,source).replaceAll('\\','/');
    if (path.endsWith('.md')) {
      const markdown = readFileSync(source,'utf8');
      const title = markdown.match(/^# (.+)$/m)?.[1] ?? 'Velora Documentation';
      write(destination(path),`<!doctype html>\n<html lang="en"><head><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1"><title>${escape(title)} · Velora</title><link rel="stylesheet" href="/assets/docs.css"></head><body><header><strong>Velora</strong><a href="/">Documentation</a><a href="/deployment/README.html">Deployment</a><a href="/APPLICATION.html">Application</a><a href="/BOUNDARIES.html">Architecture</a></header><main><p class="notice">Development documentation. Check deployment targets for hosting assignments and remaining production gates.</p>${renderMarkdown(markdown,path)}</main><footer>Source commit: ${commit} · Built documentation; deployment status is recorded in the deployment guide.</footer></body></html>\n`);
    } else write(destination(path),path.endsWith('.json') ? readFileSync(source,'utf8').replaceAll('\r\n','\n') : readFileSync(source));
  }
  if (existsSync(resolve(root,'LICENSE'))) write('LICENSE',readFileSync(resolve(root,'LICENSE'),'utf8').replaceAll('\r\n','\n'));
  write('assets/docs.css',css + '\n');
  write('_headers',`/*\n  X-Content-Type-Options: nosniff\n  Referrer-Policy: strict-origin-when-cross-origin\n  X-Frame-Options: DENY\n  Content-Security-Policy: default-src 'self'; script-src 'none'; style-src 'self'; img-src 'self' https:; frame-ancestors 'none'; base-uri 'none'; form-action 'none'\n`);
  files.sort((a,b) => a.path.localeCompare(b.path,'en'));
  const manifest = {schema_version:1,repository:'VeloraMCDev/velora-launcher',commit_sha:commit,artifact_type:'static_bundle',files,content_manifest_sha256:hash(JSON.stringify(files))};
  writeFileSync(resolve(output,'artifact-manifest.json'),JSON.stringify(manifest,null,2) + '\n');
  return manifest;
}

if (process.argv[1] && import.meta.url === pathToFileURL(process.argv[1]).href) {
  const commit = process.env.GITHUB_SHA ?? execFileSync('git',['rev-parse','HEAD'],{encoding:'utf8'}).trim();
  const manifest = buildSite('.', './dist',commit);
  console.log(JSON.stringify({commit_sha:commit,files:manifest.files.length,content_manifest_sha256:manifest.content_manifest_sha256}));
}

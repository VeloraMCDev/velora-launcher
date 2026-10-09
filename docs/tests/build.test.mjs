import { test } from 'node:test';
import assert from 'node:assert/strict';
import { mkdtempSync,writeFileSync,mkdirSync,readFileSync,rmSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { buildSite,renderMarkdown } from '../scripts/build.mjs';

test('static documentation strips executable content and rewrites local routes', () => {
  const html = renderMarkdown('# Hello\n[Guide](deployment/README.md)\n<script>alert(1)</script><img src="javascript:alert(1)" onerror="alert(1)"><a href="http://example.com">unsafe scheme</a>','README.md');
  assert.match(html,/id="hello"/);
  assert.match(html,/href="\/deployment\/README.html"/);
  assert.doesNotMatch(html,/script|onerror|javascript:|http:/);
  assert.match(renderMarkdown('[Home](../README.md)','deployment/README.md'),/href="\/index.html"/);
});

test('bundle is reproducible, source-bound and fails on broken links', () => {
  const root = mkdtempSync(join(tmpdir(),'velora-docs-'));
  try {
    writeFileSync(join(root,'README.md'),'# Test\n[Guide](deployment/README.md)');
    mkdirSync(join(root,'deployment'));
    writeFileSync(join(root,'deployment/README.md'),'# Guide');
    const first = buildSite(root,join(root,'dist'),'a'.repeat(40));
    writeFileSync(join(root,'dist','stale.html'),'stale');
    assert.deepEqual(buildSite(root,join(root,'dist'),'a'.repeat(40)),first);
    assert.ok(first.files.every(file => /^[a-f0-9]{64}$/.test(file.sha256)));
    assert.match(readFileSync(join(root,'dist/index.html'),'utf8'),/a{40}/);
    assert.notEqual(buildSite(root,join(root,'dist'),'b'.repeat(40)).content_manifest_sha256,first.content_manifest_sha256);
    assert.throws(() => buildSite(root,root,'a'.repeat(40)),/Output/);
    writeFileSync(join(root,'README.md'),'[Missing](missing.md)');
    assert.throws(() => buildSite(root,join(root,'dist'),'a'.repeat(40)),/Broken/);
  } finally { rmSync(root,{recursive:true,force:true}); }
});

test('search index covers every section and the sitemap lists every page', async () => {
  const { searchRecords } = await import('../scripts/build.mjs');
  const root = mkdtempSync(join(tmpdir(),'velora-docs-search-'));
  try {
    writeFileSync(join(root,'README.md'),'# Home\nWelcome.\n## Install the launcher\nDownload the **installer** & run it.\n### Linux\nUse the AppImage.');
    mkdirSync(join(root,'deployment'));
    writeFileSync(join(root,'deployment/README.md'),'# Deploy\n## Backups\nNightly <script>x</script> archives.');
    buildSite(root,join(root,'dist'),'a'.repeat(40),{siteUrl:'https://docs.example.org'});
    const index = JSON.parse(readFileSync(join(root,'dist/search-index.json'),'utf8'));
    const install = index.find(r => r.h === 'Install the launcher');
    assert.equal(install.u,'/index.html#install-the-launcher');
    assert.match(install.t,/Download the installer & run it/);
    assert.ok(index.some(r => r.u === '/deployment/README.html#backups' && r.s === 'Deployment' && !r.t.includes('script')));
    const sitemap = readFileSync(join(root,'dist/sitemap.xml'),'utf8');
    assert.match(sitemap,/<loc>https:\/\/docs\.example\.org\/<\/loc>/);
    assert.match(sitemap,/deployment\/README\.html/);
    const page = readFileSync(join(root,'dist/deployment/README.html'),'utf8');
    assert.match(page,/aria-current="page">Deploy</);
    assert.match(page,/<script src="\/assets\/search\.js" defer>/);
    assert.throws(() => buildSite(root,join(root,'dist'),'a'.repeat(40),{siteUrl:'http://insecure.example'}),/https/);
    assert.equal(searchRecords({html:'<h1 id="t">T</h1><p>x</p>',title:'T',dest:'t.html',section:'S'})[0].u,'/t.html');
  } finally { rmSync(root,{recursive:true,force:true}); }
});

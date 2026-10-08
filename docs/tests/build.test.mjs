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

import assert from 'node:assert/strict';
import { mkdtempSync, mkdirSync, writeFileSync, rmSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import test from 'node:test';
import { bundleMetrics } from '../scripts/bundle-metrics.mjs';

function fixture(run) {
  const root = mkdtempSync(join(tmpdir(), 'velora-bundle-fixture-'));
  try {
    mkdirSync(join(root, 'assets'));
    writeFileSync(join(root, 'index.html'), '<script type="module" src="/assets/main.js"></script>');
    writeFileSync(join(root, 'assets/main.js'), 'synthetic entry');
    writeFileSync(join(root, 'assets/shared.js'), 'synthetic shared');
    writeFileSync(join(root, 'assets/page.js'), 'synthetic deferred');
    run(root);
  } finally { rmSync(root, { recursive: true, force: true }); }
}

test('bundle metrics traverse cyclic static imports but exclude deferred page imports', () => fixture(root => {
  mkdirSync(join(root, '.vite'));
  writeFileSync(join(root, '.vite/manifest.json'), JSON.stringify({
    main: { file: 'assets/main.js', isEntry: true, imports: ['shared'], dynamicImports: ['page'] },
    shared: { file: 'assets/shared.js', imports: ['main'] },
    page: { file: 'assets/page.js' },
  }));
  const metrics = bundleMetrics(root);
  assert.equal(metrics.initialJavaScript.files, 2);
  assert.equal(metrics.initialJavaScript.raw, 31);
  assert.equal(metrics.allJavaScript.files, 3);
  assert.deepEqual(metrics.deferredJavaScript.map(chunk => chunk.file), ['assets/page.js']);
}));

test('baseline without manifest explicitly reports HTML-only scope and actual byte totals', () => fixture(root => {
  const metrics = bundleMetrics(root);
  assert.equal(metrics.initialJavaScript.raw, 15);
  assert.equal(metrics.allJavaScript.raw, 49);
  assert.match(metrics.graphScope, /manifest unavailable/);
}));

test('escaped build paths and missing imported chunks fail rather than undercount', () => fixture(root => {
  writeFileSync(join(root, 'index.html'), '<script src="../../outside.js"></script>');
  assert.throws(() => bundleMetrics(root), /escapes/);
  writeFileSync(join(root, 'index.html'), '<script src="/assets/main.js"></script>');
  mkdirSync(join(root, '.vite'));
  writeFileSync(join(root, '.vite/manifest.json'), JSON.stringify({ main: { file: 'assets/main.js', isEntry: true, imports: ['missing'] } }));
  assert.throws(() => bundleMetrics(root), /Missing static/);
}));

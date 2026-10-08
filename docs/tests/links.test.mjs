import test from 'node:test';
import assert from 'node:assert/strict';
import { mkdtempSync, writeFileSync, rmSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { checkLinks } from '../scripts/check-links.mjs';

test('docs gate detects broken/escaping links while allowing existing files and fenced examples', t => {
  const root = mkdtempSync(join(tmpdir(),'velora-doc-links-'));
  t.after(() => rmSync(root,{recursive:true,force:true}));
  writeFileSync(join(root,'target name.md'),'# Example\n');
  writeFileSync(join(root,'README.md'),'[Existing](target%20name.md#example)\n[External](https://example.invalid/)\n```md\n[Example only](not-a-file.md)\n```\n');
  assert.deepEqual(checkLinks(root).failures,[]);
  writeFileSync(join(root,'bad.md'),'[Broken](missing.md)\n[Escaping](../outside.md)\n[Invalid](%XX.md)\n');
  const result = checkLinks(root);
  assert.equal(result.failures.length,3);
  assert.ok(result.failures.some(f=>f.reason.includes('missing local target')));
  assert.ok(result.failures.some(f=>f.reason==='link escapes repository'));
});

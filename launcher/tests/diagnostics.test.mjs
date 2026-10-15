import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import test from 'node:test';
import { runInNewContext } from 'node:vm';
import { compile, parse } from 'svelte/compiler';
import ts from 'typescript';
import { createDiagnosticReport, DIAGNOSTIC_LIMITS } from '../src/lib/diagnostics.ts';

test('deterministic allowlisted report replaces original text and never traverses extensions', () => {
  const crash = { code: 1, tail: ['OutOfMemoryError synthetic-secret'], instance_id: 'synthetic-player', run_id: 'synthetic-session', crash_report: '/synthetic/private/report.txt' };
  Object.defineProperty(crash, 'extension', { enumerable: true, get() { throw new Error('extension must not be read'); } });
  const preparation = { state: 'error', stage: 'java', message: 'UnsupportedClassVersionError synthetic-token', label: 'synthetic-label', done: 999, account: { token: 'synthetic-token' } };
  const result = createDiagnosticReport(crash, preparation);
  assert.deepEqual(result, createDiagnosticReport(crash, preparation));
  assert.deepEqual(result.categories, ['memory', 'java-version']);
  assert.match(result.text, /Exit code: 1/);
  assert.match(result.text, /Preparation state: error/);
  assert.match(result.text, /Preparation stage: java/);
  assert.doesNotMatch(result.text, /synthetic-|999|report\.txt/);
  assert.match(result.text, /not correlated/);
});

test('synthetic secrets, URLs, paths, IPs, email and control characters never reach output', () => {
  const sensitive = [
    'Authorization: Bearer synthetic-bearer-token',
    '--accessToken synthetic-access-token --username synthetic-player',
    '{"refresh_token":"synthetic-refresh","session":"synthetic-session"}',
    'https://synthetic-user:synthetic-pass@server.invalid:25565/path?token=synthetic-query',
    'server.invalid:25565 192.0.2.10:25565 [2001:db8::1234]:25565',
    '/Users/synthetic-player/Minecraft/logs/latest.log C:\\Users\\synthetic-player\\game \\synthetic-share\\player',
    '../synthetic-private ~/.synthetic-store player@synthetic.invalid',
    '\u001b[31m\u0000synthetic-secret\u202e\u2066\u007f',
    'opaqueSyntheticSecretWithoutRecognizableSyntax',
  ];
  for (const secret of sensitive) {
    for (const prefix of ['', 'OutOfMemoryError ']) {
      const result = createDiagnosticReport({ tail: [prefix + secret], code: secret }, { state: secret, stage: secret, message: prefix + secret });
      assert.doesNotMatch(result.text, /synthetic|opaqueSynthetic|192\.0\.2|2001:db8|https:\/\/|Bearer|accessToken|refresh_token|@|\u001b|\u0000|\u202e|\u2066|\u007f/i);
      assert.match(result.text, /Exit code: unavailable/);
      assert.match(result.text, /Preparation state: unavailable/);
      assert.match(result.text, /Preparation stage: unavailable/);
    }
  }
});

test('oversize inputs inspect only a bounded recent tail and bounded line prefix', () => {
  const tail = ['UnsupportedClassVersionError', ...Array(100).fill('synthetic-secret'.repeat(1000)), 'OutOfMemoryError'];
  const result = createDiagnosticReport({ tail });
  assert.deepEqual(result.categories, ['memory']);
  assert.match(result.text, /Input omitted or truncated: yes/);
  assert.equal(result.text.split('[redacted unrecognized text]').length - 1, 39);
  assert.doesNotMatch(result.text, /synthetic-secret/);
  assert.ok(result.text.length <= DIAGNOSTIC_LIMITS.report);
  assert.deepEqual(createDiagnosticReport({ tail: [' '.repeat(512) + 'OutOfMemoryError'] }).categories, ['unknown']);
  const allSignals = 'OutOfMemoryError UnsupportedClassVersionError Invalid maximum heap size Mixin UnknownHostException Permission denied';
  assert.ok(createDiagnosticReport({ tail: Array(100).fill(allSignals) }).text.length <= DIAGNOSTIC_LIMITS.report);
});

test('empty and malformed snapshots return useful unknown guidance without coercing objects', () => {
  for (const empty of [undefined, null, {}, [], '', 12]) {
    const result = createDiagnosticReport(empty, empty);
    assert.deepEqual(result.categories, ['unknown']);
    assert.match(result.text, /No crash tail data available/);
    assert.match(result.text, /cause is not established/);
    assert.match(result.text, /Exit code: unavailable/);
  }
  const untrusted = { toString() { throw new Error('must not coerce'); }, nested: 'synthetic-token' };
  assert.doesNotThrow(() => createDiagnosticReport({ code: untrusted, tail: [null, 123, untrusted] }, { state: untrusted, stage: untrusted, message: untrusted }));
  for (const invalid of [NaN, Infinity, 0.5, 2147483648, -2147483648]) assert.match(createDiagnosticReport({ code: invalid }).text, /Exit code: unavailable/);
  assert.match(createDiagnosticReport({ code: 0 }).text, /Exit code: 0/);
});

test('structured signature categories give qualified, non-destructive advice', () => {
  const fixtures = [
    ['Java heap space', 'memory'],
    ['UnsupportedClassVersionError', 'java-version'],
    ['Could not reserve enough space', 'memory-configuration'],
    ['Incompatible mod set', 'mod-loader'],
    ['SocketTimeoutException', 'network'],
    ['AccessDeniedException', 'file-access'],
  ];
  for (const [signature, category] of fixtures) {
    const result = createDiagnosticReport({ tail: [signature, signature] });
    assert.deepEqual(result.categories, [category]);
    assert.equal(result.advice.length, 1);
    assert.doesNotMatch(result.advice.join(' '), /delete|remove|will fix|healthy|successfully launched/i);
    assert.match(result.text, /not a health check/);
  }
  assert.deepEqual(createDiagnosticReport({ tail: ['a mod requires synthetic-secret but something else'] }).categories, ['unknown']);
});

test('CrashDialog compiles and routes both explicit sharing actions through the safe report', async () => {
  const source = await readFile(new URL('../src/components/CrashDialog.svelte', import.meta.url), 'utf8');
  const compiled = compile(source, { filename: 'CrashDialog.svelte', generate: 'client' });
  assert.deepEqual(compiled.warnings, []);
  assert.match(source, /createDiagnosticReport\(crash, app\.launch\)/);
  assert.match(source, /await navigator\.clipboard\.writeText\(report\.text\)/);
  assert.match(source, /new Blob\(\[report\.text\]/);
  assert.match(source, /onclick=\{copyReport\}/);
  assert.match(source, /onclick=\{exportReport\}/);
  assert.doesNotMatch(source, /(?:crash|app\.crash)\??\.tail|fetch\(|invoke\(['"](?:diagnostics|export|upload)/);
  assert.match(source, /URL\.revokeObjectURL/);
});

async function sharingFixture(overrides = {}) {
  const source = await readFile(new URL('../src/components/CrashDialog.svelte', import.meta.url), 'utf8');
  const functions = parse(source, { modern: true }).instance.content.body
    .filter((node) => node.type === 'FunctionDeclaration' && ['copyReport', 'exportReport'].includes(node.id.name))
    .map((node) => source.slice(node.start, node.end)).join('\n');
  const compiled = ts.transpileModule(functions, { compilerOptions: { target: ts.ScriptTarget.ES2022 } }).outputText;
  const fixture = {
    crash: { code: 1, tail: ['OutOfMemoryError synthetic-private-token'] },
    report: createDiagnosticReport({ code: 1, tail: ['OutOfMemoryError synthetic-private-token'] }),
    copies: [], notices: [], blobs: [], downloads: [], timers: [], revoked: [],
  };
  const context = {
    crash: fixture.crash,
    report: fixture.report,
    navigator: { clipboard: { async writeText(text) { fixture.copies.push(text); } } },
    toast(text, kind) { fixture.notices.push({ text, kind }); },
    Blob,
    URL: {
      createObjectURL(blob) { fixture.blobs.push(blob); return 'blob:synthetic-report'; },
      revokeObjectURL(url) { fixture.revoked.push(url); },
    },
    document: { createElement(tag) {
      assert.equal(tag, 'a');
      return { href: '', download: '', click() { fixture.downloads.push({ href: this.href, filename: this.download }); } };
    } },
    setTimeout(callback, delay) { fixture.timers.push({ callback, delay }); },
    ...overrides,
  };
  const handlers = runInNewContext(`${compiled}\n({ copyReport, exportReport });`, context);
  return { fixture, handlers };
}

test('real dialog handlers share only on request, match preview, and clean up local export', async () => {
  const { fixture, handlers } = await sharingFixture();
  assert.equal(fixture.copies.length + fixture.blobs.length + fixture.downloads.length, 0);
  await handlers.copyReport();
  assert.deepEqual(fixture.copies, [fixture.report.text]);
  assert.deepEqual(fixture.notices, [{ text: 'Support report copied', kind: undefined }]);
  handlers.exportReport();
  assert.equal(await fixture.blobs[0].text(), fixture.report.text);
  assert.equal(fixture.blobs[0].type, 'text/plain;charset=utf-8');
  assert.deepEqual(fixture.downloads, [{ href: 'blob:synthetic-report', filename: 'velora-support-diagnostics.txt' }]);
  assert.doesNotMatch(fixture.copies[0], /synthetic-private-token/);
  assert.equal(fixture.timers.length, 1);
  fixture.timers[0].callback();
  assert.deepEqual(fixture.revoked, ['blob:synthetic-report']);
});

test('real copy handler waits for completion and does not expose clipboard errors', async () => {
  let rejectCopy;
  const pending = new Promise((resolve, reject) => { rejectCopy = reject; });
  const { fixture, handlers } = await sharingFixture({ navigator: { clipboard: { writeText() { return pending; } } } });
  const copying = handlers.copyReport();
  assert.deepEqual(fixture.notices, []);
  rejectCopy(new Error('synthetic-sensitive-platform-error'));
  await copying;
  assert.equal(fixture.notices.length, 1);
  assert.equal(fixture.notices[0].kind, 'error');
  assert.doesNotMatch(fixture.notices[0].text, /synthetic|copied/);
});

test('real handlers handle empty data and export failure without claims or leaked errors', async () => {
  const empty = await sharingFixture({ crash: null });
  await empty.handlers.copyReport();
  empty.handlers.exportReport();
  assert.equal(empty.fixture.copies.length + empty.fixture.blobs.length + empty.fixture.notices.length, 0);
  const denied = await sharingFixture({ document: { createElement() { throw new Error('synthetic-private-path'); } } });
  denied.handlers.exportReport();
  assert.equal(denied.fixture.notices[0].kind, 'error');
  assert.doesNotMatch(denied.fixture.notices[0].text, /synthetic-private-path/);
  assert.equal(denied.fixture.downloads.length, 0);
  denied.fixture.timers[0].callback();
  assert.deepEqual(denied.fixture.revoked, ['blob:synthetic-report']);
});

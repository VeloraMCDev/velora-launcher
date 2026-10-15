import assert from 'node:assert/strict';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import test from 'node:test';
import { fileURLToPath } from 'node:url';
import ts from 'typescript';

const launcher = fileURLToPath(new URL('..', import.meta.url));
const fixtures = JSON.parse(fs.readFileSync(new URL('../../crates/shared/test-fixtures/launcher-contracts.json', import.meta.url), 'utf8'));
const contracts = {
  config: 'ConnectionsConfig',
  start: 'DiscordStartResponse',
  connection: 'DiscordConnectionResponse',
  info: 'DiscordInfoResponse',
  poll: 'DiscordPollResponse',
  unlink: 'DiscordUnlinkResponse',
  ipc_poll: 'DiscordSignInPollResult',
};

function diagnostics(declarations) {
  const temporary = fs.mkdtempSync(path.join(os.tmpdir(), 'velora-api-contract-'));
  try {
    const source = path.join(temporary, 'contract.ts');
    const types = path.join(launcher, 'src/lib/types');
    fs.writeFileSync(source, `import type { ${Object.values(contracts).join(', ')} } from ${JSON.stringify(types)};\n${declarations}`);
    const config = ts.readConfigFile(path.join(launcher, 'tsconfig.json'), ts.sys.readFile);
    assert.equal(config.error, undefined);
    const parsed = ts.parseJsonConfigFileContent(config.config, ts.sys, launcher);
    const program = ts.createProgram([source], { ...parsed.options, noEmit: true });
    return ts.getPreEmitDiagnostics(program).map(diagnostic => ts.flattenDiagnosticMessageText(diagnostic.messageText, '\n'));
  } finally {
    fs.rmSync(temporary, { recursive: true, force: true });
  }
}

test('shared Rust wire fixtures satisfy actual launcher TypeScript response contracts', () => {
  const declarations = fixtures.map((fixture, index) => {
    assert.ok(contracts[fixture.type]);
    return `const fixture${index}: ${contracts[fixture.type]} = ${JSON.stringify(fixture.value)}; void fixture${index};`;
  }).join('\n');
  assert.deepEqual(diagnostics(declarations), []);
});

test('frontend token-free poll contract rejects raw auth and malformed required shapes', () => {
  for (const [contract, value] of [
    ['DiscordSignInPollResult', fixtures.find(fixture => fixture.type === 'poll' && fixture.value.token).value],
    ['DiscordStartResponse', { state: 'synthetic-only-state' }],
    ['DiscordConnectionResponse', { id: 7, name: 'Synthetic' }],
    ['DiscordPollResponse', { pending: false }],
  ]) {
    assert.ok(diagnostics(`const invalid: ${contract} = ${JSON.stringify(value)}; void invalid;`).length > 0);
  }
});

import test from 'node:test';
import assert from 'node:assert/strict';
import { existsSync, mkdtempSync, mkdirSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { spawnSync } from 'node:child_process';
import { dashboardUpdateStatus } from './actions.mjs';

const shell = process.platform === 'win32' ? 'C:/Program Files/Git/bin/bash.exe' : '/bin/sh';
const unixPath = value => value.replaceAll('\\', '/').replace(/^([A-Za-z]):/, (_, drive) => '/' + drive.toLowerCase());
const script = fileURLToPath(new URL('./self-update.sh', import.meta.url));

for (const scenario of ['success', 'pull-failed', 'unhealthy', 'rollback-failed']) {
  test(`detached dashboard update: ${scenario}`, { skip: !existsSync(shell) }, () => {
    const root = mkdtempSync(join(tmpdir(), 'velora-self-update-'));
    try {
      const bin = join(root, 'bin');
      mkdirSync(bin);
      writeFileSync(join(root, '.env'), 'OPS_IMAGE=new\n');
      writeFileSync(join(root, '.env.ops-rollback'), 'OPS_IMAGE=previous\n');
      writeFileSync(join(bin, 'sleep'), '#!/bin/sh\nexit 0\n', { mode: 0o755 });
      writeFileSync(join(bin, 'seq'), '#!/bin/sh\nprintf "1\\n2\\n3\\n"\n', { mode: 0o755 });
      writeFileSync(join(bin, 'docker'), `#!/bin/sh
if [ "$1" = compose ]; then
  shift 3
  case "$1" in
    pull) [ "$SCENARIO" != pull-failed ]; exit $? ;;
    up)
      phase=$(cat "$MOCK_ROOT/phase" 2>/dev/null || echo 0)
      echo $((phase + 1)) > "$MOCK_ROOT/phase"
      exit 0 ;;
    ps) echo fixture; exit 0 ;;
  esac
fi
if [ "$1" = inspect ]; then
  phase=$(cat "$MOCK_ROOT/phase")
  if [ "$SCENARIO" = rollback-failed ] || { [ "$SCENARIO" = unhealthy ] && [ "$phase" = 1 ]; }; then
    echo unhealthy
  else
    echo healthy
  fi
  exit 0
fi
exit 1
`, { mode: 0o755 });
      const result = spawnSync(shell, ['-c', 'export PATH="$1:$PATH"; shift; exec sh "$@"', '--', unixPath(bin), unixPath(script), 'fixture-project', unixPath(join(root, '.env')), unixPath(join(root, '.env.ops-rollback'))], {
        env: { ...process.env, SCENARIO: scenario, MOCK_ROOT: unixPath(root) }, encoding: 'utf8', timeout: 20_000,
      });
      assert.ifError(result.error);
      assert.equal(result.status, scenario === 'success' ? 0 : 1, result.stderr);
      const expected = scenario === 'success' ? 'succeeded' : scenario === 'rollback-failed' ? 'rollback-failed' : 'rolled-back';
      assert.equal(readFileSync(join(root, '.ops-update-result'), 'utf8').trim(), expected);
      assert.equal(dashboardUpdateStatus(root), expected);
      assert.equal(readFileSync(join(root, '.env'), 'utf8'), scenario === 'success' ? 'OPS_IMAGE=new\n' : 'OPS_IMAGE=previous\n');
      assert.equal(existsSync(join(root, '.env.ops-rollback')), scenario === 'rollback-failed');
    } finally { rmSync(root, { recursive: true, force: true }); }
  });
}

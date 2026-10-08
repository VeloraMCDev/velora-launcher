// Acceptance for the exact Panel runtime image: starts it on fresh synthetic storage,
// then checks identity, schema readiness, non-root execution and OCI labels.
import {execFileSync} from 'node:child_process';
const [image, commit] = process.argv.slice(2);
if (!image || !/^[a-f0-9]{40}$/.test(commit ?? '')) throw Error('Supply image and the exact expected Git SHA');
const name = `velora-panel-check-${process.pid}`;
const docker = (...args) => execFileSync('docker', args, {encoding: 'utf8', timeout: 60_000}).trim();
try {
  // Fresh anonymous volume; synthetic credentials only.
  docker('run', '--detach', '--name', name, '--read-only', '--tmpfs', '/tmp:rw,mode=1777,size=64m', '--cap-drop', 'ALL',
    '--security-opt', 'no-new-privileges:true', '-e', 'ADMIN_PASSWORD=Synthetic-Image-Check-7391!', '-p', '127.0.0.1::8080', image);
  const mapped = docker('port', name, '8080/tcp').split('\n')[0];
  const url = `http://${mapped.replace('0.0.0.0', '127.0.0.1')}/health`;
  let health;
  for (let attempt = 0; attempt < 30 && !health; attempt++) {
    try {
      const response = await fetch(url, {signal: AbortSignal.timeout(3000)});
      if (response.ok) health = await response.json();
    } catch { /* still starting */ }
    if (!health) await new Promise(resolve => setTimeout(resolve, 1000));
  }
  if (!health) throw Error(`Panel did not become ready. Logs:\n${docker('logs', name)}`);
  if (health.status !== 'ok' || health.service !== 'velora-panel') throw Error('Unexpected health identity');
  if (health.commit !== commit) throw Error(`Image reports commit ${health.commit}, expected ${commit}`);
  if (health.schema !== health.expected_schema) throw Error('Store schema does not match the code');
  const user = docker('inspect', '--format', '{{.Config.User}}', name);
  if (user !== '65532:65532') throw Error(`Image must run as 65532:65532, got ${user}`);
  const label = docker('inspect', '--format', '{{ index .Config.Labels "org.opencontainers.image.revision" }}', name);
  if (label !== commit) throw Error('OCI revision label and health commit disagree');
  const legacy = await fetch(url.replace('/health', '/healthz'), {signal: AbortSignal.timeout(3000)});
  if (!legacy.ok) throw Error('Legacy /healthz must keep working');
  console.log(`Panel image passes: commit ${commit}, schema ${health.schema}, non-root, read-only root.`);
} finally {
  try { docker('rm', '--force', '--volumes', name); } catch { /* already gone */ }
}

import { execFileSync } from 'node:child_process';
const [image, commit] = process.argv.slice(2);
if (!image || !/^[a-f0-9]{40}$/.test(commit ?? '')) throw Error('Supply image and exact expected Git SHA');
const name = `velora-probe-check-${process.pid}`;
const docker = (...args) => execFileSync('docker', args, { encoding:'utf8', timeout:30_000 }).trim();
try {
  docker('run', '--detach', '--name', name, '--network', 'none', '--read-only', '--cap-drop', 'ALL', '--security-opt', 'no-new-privileges:true', image);
  let ready = false;
  for (let attempt = 0; attempt < 10; attempt++) {
    try {
      docker('exec', name, 'node', '-e', `const fs=require('node:fs');if(process.getuid()===0)throw Error('root runtime');fetch('http://127.0.0.1:8080/health').then(async r=>{const h=await r.json();if(!r.ok||h.service!=='deployment-probe'||h.status!=='ok'||h.commit!==${JSON.stringify(commit)}||h.commit!==JSON.parse(fs.readFileSync('/app/build-info.json')).commit)throw Error('health identity mismatch')}).catch(()=>process.exit(1))`);
      ready = true; break;
    } catch { await new Promise(resolve => setTimeout(resolve, 1000)); }
  }
  if (!ready) throw Error('Probe container did not pass expected-SHA, unprivileged/read-only health acceptance');
  const revision = docker('inspect', '--format', '{{ index .Config.Labels "org.opencontainers.image.revision" }}', name);
  if (revision !== commit) throw Error('OCI label and health commit disagree');
  console.log('Probe container passes exact commit, non-root, read-only and no-network health acceptance.');
} finally { docker('rm', '--force', name); }

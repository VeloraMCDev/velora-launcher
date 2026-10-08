import { readFile, writeFile } from 'node:fs/promises';
import { createHash } from 'node:crypto';

// Wrangler's dry-run emits the single bundled ES module used for deployment.
const sha = process.env.GITHUB_SHA;
if (!/^[a-f0-9]{40}$/.test(sha ?? '')) throw Error('Full source SHA required');
const bytes = await readFile('control-plane/dist/control-plane/index.js');
await writeFile('control-plane/dist/control-plane/artifact-manifest.json', JSON.stringify({
  schema_version: 1, repository: 'VeloraMCDev/velora-launcher', source_sha: sha,
  service: 'velora-control-plane', artifact_type: 'worker',
  files: [{path:'index.js',bytes:bytes.length,sha256:createHash('sha256').update(bytes).digest('hex')}]
}, null, 2) + '\n');

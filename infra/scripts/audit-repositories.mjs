import { execFileSync } from 'node:child_process';
import { readFileSync, writeFileSync } from 'node:fs';
import { resolve, dirname } from 'node:path';
import { fileURLToPath } from 'node:url';
import { createHash } from 'node:crypto';

const ownRoot = resolve(dirname(fileURLToPath(import.meta.url)), '..');
const catalog = JSON.parse(readFileSync(resolve(ownRoot, 'deployment/repository-catalog.json'), 'utf8'));
const args = process.argv.slice(2);
const root = resolve(args[0] ?? resolve(ownRoot, '..'));
const output = resolve(args[1] ?? resolve(ownRoot, 'deployment/repository-audit.json'));
const guide = args[2] ? resolve(args[2]) : null;
const git = (cwd, ...command) => execFileSync('git', ['-C', cwd, ...command], { maxBuffer: 16 * 1024 * 1024 }).toString().trim();
const sha = bytes => createHash('sha256').update(bytes).digest('hex');
const repositories = [];
for (const entry of catalog.repositories) {
  const cwd = resolve(root, entry.name);
  const revision = git(cwd, 'rev-parse', 'HEAD');
  const repository = `VeloraMCDev/${entry.name}`;
  const remote = git(cwd, 'remote', 'get-url', 'origin');
  if (!new RegExp(`^https://github\\.com/${repository}(?:\\.git)?$`, 'i').test(remote)) throw Error(`Unexpected source owner: ${entry.name}`);
  const metadata = JSON.parse(execFileSync('gh', ['repo', 'view', repository, '--json', 'defaultBranchRef,isPrivate']).toString());
  const files = git(cwd, 'ls-files').split('\n').filter(path => !/^(?:migration\/legacy-source|vendor|java\/vendor|sdk-snapshot)\//.test(path));
  const baseline = entry.baseline_run === null ? null : JSON.parse(execFileSync('gh', ['run', 'view', String(entry.baseline_run), '--repo', repository, '--json', 'headSha,status,conclusion,url,attempt,jobs'], { maxBuffer: 4 * 1024 * 1024 }).toString());
  if (baseline && baseline.headSha !== revision) throw Error(`Baseline revision differs: ${entry.name}`);
  const evidence = entry.evidence_paths.map(path => ({ path, sha256: sha(execFileSync('git', ['-C', cwd, 'show', `${revision}:${path}`])) }));
  // Extract configuration NAMES from committed source, never environment values.
  const names = new Set();
  const secretNames = new Set();
  for (const path of files.filter(path => /\.(?:rs|mjs|py|toml|ya?ml)$/.test(path))) {
    const source = execFileSync('git', ['-C', cwd, 'show', `${revision}:${path}`], { maxBuffer: 4 * 1024 * 1024 }).toString();
    for (const match of source.matchAll(/\b(?:VELORA|SCOPENET|CLOUDFLARE|TAURI|GITHUB)_[A-Z][A-Z0-9_]+\b/g)) names.add(match[0]);
    for (const match of source.matchAll(/secrets\.([A-Z][A-Z0-9_]+)/g)) secretNames.add(match[1]);
  }
  repositories.push({ ...entry, repository, revision, default_branch: metadata.defaultBranchRef.name, visibility: metadata.isPrivate ? 'private' : 'public',
    status: 'PARTIAL', build_classification: baseline?.conclusion === 'success' ? 'buildable components; product incomplete' : baseline ? 'baseline validation in progress' : 'incomplete; no existing build/test pipeline',
    ci: files.filter(path => path.startsWith('.github/workflows/')),
    cloudflare_config: files.filter(path => /(?:wrangler|terraform|\.tf$|cloudflare)/i.test(path)),
    containers: files.filter(path => /(?:Dockerfile|compose\.(?:ya?ml)|docker-compose)/i.test(path)),
    api_schema_paths: files.filter(path => /(?:schema|openapi|protocol|contract|http-baseline)/i.test(path)),
    configuration_names: [...names].sort(), ci_secret_names: [...secretNames].sort(), evidence, baseline });
}
writeFileSync(output, JSON.stringify({ schema: 1, audited_at: new Date().toISOString(), guide_sha256: guide ? sha(readFileSync(guide)) : null,
  statuses: ['EXISTING', 'PARTIAL', 'MISSING', 'BLOCKED', 'DONE'], repositories,
  runtime_resources: { cloudflare_account: 'UNKNOWN; connection pending', zones: 'UNKNOWN', deployed_bindings: 'UNKNOWN', hermes: 'operator-confirmed Linux Mint with Docker; agent connection not yet enrolled', production: 'UNKNOWN' },
  scope: 'Exact committed migration heads, source/configuration names and unchanged GitHub build/test evidence. Does not certify main, remote live resources, full product composition, secret custody or production readiness.' }, null, 2) + '\n');
console.log(`Audited ${repositories.length} exact repository heads; wrote ${output}. No secret values collected.`);

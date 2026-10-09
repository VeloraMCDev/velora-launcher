import { parse } from 'yaml';
import { readFileSync, readdirSync } from 'node:fs';
import { resolve } from 'node:path';
import { pathToFileURL } from 'node:url';

export function workflowFindings(source) {
  const workflow = parse(source);
  const findings = [];
  if (!workflow || typeof workflow !== 'object' || !workflow.on || !workflow.jobs) return ['Missing workflow trigger/jobs'];
  if (workflow.permissions?.contents !== 'read' || workflow.permissions === 'write-all') findings.push('Default token must have contents: read');
  for (const [id, job] of Object.entries(workflow.jobs)) {
    if (!Number.isInteger(job['timeout-minutes']) || job['timeout-minutes'] < 1 || job['timeout-minutes'] > 60) findings.push(`${id}: bounded job timeout required`);
    if (job.permissions === 'write-all') findings.push(`${id}: write-all prohibited`);
    const developmentRegistration=id==='register-development'
      && job.if==="github.event_name != 'pull_request' && github.ref == 'refs/heads/main' && vars.VELORA_DEVELOPMENT_REGISTRATION_ENABLED == 'true'"
      && job.needs===(workflow.jobs['contracts-and-container']?'contracts-and-container':workflow.jobs.candidate?'candidate':'image')
      && Boolean(workflow.jobs[job.needs]) && job['runs-on']==='ubuntu-latest'
      && (job.needs!=='image' || ['scripts/check-panel-container.mjs','.github/scripts/test-panel-image.py']
        .every(check=>(workflow.jobs.image.steps ?? []).some(step=>step.run?.includes(check))))
      && job.permissions?.['id-token']==='write' && job.permissions?.contents==='read'
      && Object.keys(job.permissions).every(key=>['contents','id-token'].includes(key));
    if(id==='register-development' && !developmentRegistration) findings.push(`${id}: exact opt-in Development registration policy required`);
    if (job.permissions && Object.values(job.permissions).includes('write') && job.if !== "github.ref == 'refs/heads/main'" && !developmentRegistration) findings.push(`${id}: elevated token job requires main or the exact reviewed Development OIDC registration policy`);
    if (JSON.stringify(job['runs-on']).includes('self-hosted')) findings.push(`${id}: self-hosted trust isolation requires separate reviewed policy`);
    for (const step of job.steps ?? []) {
      if (!step.uses) continue;
      if (!step.uses.startsWith('./') && !/^[\w.-]+\/[\w./-]+@[a-f0-9]{40}$/.test(step.uses)) findings.push(`${id}: action must use a full commit SHA`);
      if (step.uses.startsWith('actions/checkout@') && step.with?.['persist-credentials'] !== false) findings.push(`${id}: checkout credential persistence must be disabled`);
      if (step.uses.startsWith('actions/upload-artifact@') && (!Number.isInteger(step.with?.['retention-days']) || step.with['retention-days'] < 1 || step.with['retention-days'] > 30)) findings.push(`${id}: transient artifact retention must be 1–30 days`);
    }
  }
  return findings;
}

if (process.argv[1] && import.meta.url === pathToFileURL(process.argv[1]).href) {
  const roots = process.argv.slice(2);
  if (!roots.length) roots.push('.');
  let failed = false;
  for (const root of roots) {
    const directory = resolve(root,'.github/workflows');
    for (const file of readdirSync(directory).filter(name=>/\.ya?ml$/.test(name))) {
      const findings = workflowFindings(readFileSync(resolve(directory,file),'utf8'));
      console.log(`${resolve(root)} / ${file}: ${findings.length ? findings.join('; ') : 'PASS'}`);
      if (findings.length) failed = true;
    }
  }
  if (failed) process.exitCode = 1;
}

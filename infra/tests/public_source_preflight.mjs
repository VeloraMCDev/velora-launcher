import {execFileSync} from 'node:child_process';
import {readFileSync,writeFileSync} from 'node:fs';
import {resolve,dirname,relative,isAbsolute,basename} from 'node:path';
import {createHash} from 'node:crypto';
import {fileURLToPath} from 'node:url';

const privateRepository = /(?:github\.com[/:]|githubusercontent\.com\/)VeloraMCDev\/experiences(?:[/.#?\s"']|$)/i;
const secretPatterns = [
 ['github-token',/\bgh[pousr]_[A-Za-z0-9]{36,}\b/],
 ['github-fine-grained-token',/\bgithub_pat_[A-Za-z0-9_]{70,}\b/],
 ['private-key-material',/-----BEGIN(?: RSA| EC| OPENSSH)? PRIVATE KEY-----\r?\n[A-Za-z0-9+/=\r\n]{100,}-----END(?: RSA| EC| OPENSSH)? PRIVATE KEY-----/]
];
const sha = value=>createHash('sha256').update(value).digest('hex');
function git(root,args,input) {
 return execFileSync('git',['-C',root,...args],{input,maxBuffer:256*1024*1024});
}
function inside(root,path) {
 const rel=relative(root,path);
 return rel!== '..' && !rel.startsWith('..\\') && !rel.startsWith('../') && !isAbsolute(rel);
}

export function auditRepository(argument) {
 const root=resolve(argument);
 const remote=git(root,['remote','get-url','origin']).toString().trim();
 if (privateRepository.test(remote)) throw Error('Private Experiences must not be a public audit input');
 const revision=git(root,['rev-parse','HEAD']).toString().trim();
 const findings=[];
 const add=(check,path,object)=>findings.push({check,path,object});
 if (/:\/\/[^/@\s]+@/.test(remote)) add('credential-bearing-remote','<origin>');
 const tracked=git(root,['ls-tree','-r','--name-only','HEAD']).toString().trim().split('\n').filter(Boolean);
 const artifacts=tracked.includes('PUBLIC_ARTIFACTS.json')?JSON.parse(git(root,['show','HEAD:PUBLIC_ARTIFACTS.json'])).files:[];
 if (!Array.isArray(artifacts)) throw Error('Artifact provenance must declare files');
 let manifests=0,snapshots=0;
 for (const path of tracked) {
  if (/(?:^|\/)(?:migration\/legacy-source|private-gameplay|private-experiences)(?:\/|$)/i.test(path)) add('private-implementation-path',path);
  if (/(?:^|\/)(?:\.env|auth\.sqlite(?:-wal|-shm)?|bootstrap-password|jwt\.key|signing\.key)$/.test(path)) add('operator-material-path',path);
  if (/\.(?:exe|dll|jar|pdb|msi|sqlite|sqlite3|db)$/i.test(path)) {
   const reviewed=artifacts.find(file=>file.path===path);
   if (!reviewed || reviewed.sha256!==sha(git(root,['show',`HEAD:${path}`])) || !tracked.includes(reviewed.license_path) || !/^https:\/\//.test(reviewed.upstream??'')) add('binary-or-database-needs-review',path);
  }
  if (/SNAPSHOT\.json$/.test(path)) {
   snapshots++;
   const manifest=JSON.parse(git(root,['show',`HEAD:${path}`]));
   if (privateRepository.test(manifest.repository??'')) add('private-snapshot-owner',path);
   if (!Array.isArray(manifest.files)||manifest.files.length===0) { add('empty-snapshot',path); continue; }
   for (const file of manifest.files) {
    const target=resolve(root,dirname(path),file.path);
    if (!inside(resolve(root,dirname(path)),target)) { add('snapshot-path-escape',path); continue; }
    const trackedPath=relative(root,target).replaceAll('\\','/');
    if (!tracked.includes(trackedPath)) { add('untracked-snapshot-file',trackedPath); continue; }
    if (sha(git(root,['show',`HEAD:${trackedPath}`]))!==file.sha256) add('snapshot-drift',trackedPath);
   }
  }
  if (!/(?:^|\/)(?:Cargo\.toml|package\.json|[^/]*\.gradle(?:\.kts)?)$/.test(path) && !/^\.github\/workflows\//.test(path)) continue;
  manifests++;
  const content=git(root,['show',`HEAD:${path}`]).toString();
  if (privateRepository.test(content)) add('private-build-or-workflow-dependency',path);
  if (basename(path)==='Cargo.toml') {
   for (const match of content.matchAll(/\bpath\s*=\s*"([^"\r\n]+)"/g)) {
    if (!inside(root,resolve(root,dirname(path),match[1]))) add('cargo-path-outside-checkout',path);
   }
   for (const group of content.matchAll(/\bmembers\s*=\s*\[([^\]]*)\]/g)) {
    for (const match of group[1].matchAll(/"([^"\r\n]+)"/g)) if (!inside(root,resolve(root,dirname(path),match[1]))) add('cargo-member-outside-checkout',path);
   }
  }
  if (basename(path)==='package.json') {
   const manifest=JSON.parse(content);
   for (const section of ['dependencies','devDependencies','optionalDependencies','peerDependencies']) {
    for (const spec of Object.values(manifest[section]??{})) {
     if (typeof spec==='string' && /^(?:file|link):/.test(spec) && !inside(root,resolve(root,dirname(path),spec.slice(5)))) add('npm-path-outside-checkout',path);
    }
   }
   const members=Array.isArray(manifest.workspaces)?manifest.workspaces:manifest.workspaces?.packages??[];
   for (const member of members) if (!inside(root,resolve(root,dirname(path),member))) add('npm-member-outside-checkout',path);
  }
 }
 // Scan each reachable blob once across local branches/tags/remote refs. No code
 // or candidate credential bytes are emitted, including on failure.
 const objects=git(root,['rev-list','--objects','--all']).toString().trim().split('\n').filter(Boolean);
 const aliases=new Map(objects.map(line=>{const [id,...path]=line.split(' ');return [id,path.join(' ')];}));
 const ids=[...aliases.keys()];
 const bytes=git(root,['cat-file','--batch'],ids.join('\n')+'\n');
 let offset=0,blobs=0;
 while (offset<bytes.length) {
  const end=bytes.indexOf(10,offset);
  if (end<0) throw Error('Invalid Git object framing');
  const [id,type,sizeText]=bytes.subarray(offset,end).toString().split(' ');
  const size=Number(sizeText);
  if (!Number.isSafeInteger(size)||size<0||end+1+size>=bytes.length) throw Error('Invalid Git object size');
  const body=bytes.subarray(end+1,end+1+size);
  offset=end+1+size+1;
  if (type!=='blob') continue;
  blobs++;
  const text=body.toString('utf8');
  for (const [check,pattern] of secretPatterns) if (pattern.test(text)) add(check,aliases.get(id),id);
 }
 return {repository:remote.replace(/\/\/[^/@\s]+@/g,'//[redacted]@').replace(/\.git$/,''),revision,tracked_files:tracked.length,reachable_blobs:blobs,dependency_workflow_files:manifests,snapshots,passed:findings.length===0,findings,
  scope:'Tracked HEAD dependencies/snapshots/artifact paths and reachable local Git blobs; high-confidence credential patterns only. Does not certify releases/LFS/remote refs, dependency licenses, minified bundles, public availability or full migration.'};
}

if (process.argv[1] && resolve(process.argv[1])===fileURLToPath(import.meta.url)) {
 const args=process.argv.slice(2);
 const outputIndex=args.indexOf('--output');
 if (outputIndex===args.length-1) throw Error('Supply an output path after --output');
 const output=outputIndex>=0?args.splice(outputIndex,2)[1]:undefined;
 if (!args.length) throw Error('Supply prepared public repository checkout paths; Experiences is refused');
 const repositories=args.map(auditRepository);
 const report={checker_sha256:sha(readFileSync(fileURLToPath(import.meta.url))),repositories,passed:repositories.every(repo=>repo.passed),release_gate_closed:false};
 const json=JSON.stringify(report,null,2)+'\n';
 if (output) writeFileSync(output,json);
 else process.stdout.write(json);
 if (!report.passed) process.exitCode=1;
}

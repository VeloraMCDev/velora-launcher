// Scan only tracked source, never ignored operator files. Reports stay temporary.
import {execFileSync,spawnSync} from 'node:child_process';
import {mkdtempSync,mkdirSync,copyFileSync,readFileSync,rmSync} from 'node:fs';
import {tmpdir} from 'node:os';
import {resolve,dirname,relative,isAbsolute} from 'node:path';
import {fileURLToPath,pathToFileURL} from 'node:url';
import {createHash} from 'node:crypto';

export const normalizedHash=bytes=>createHash('sha256').update(bytes.toString('utf8').replaceAll('\r\n','\n')).digest('hex');
export function reviewedFinding(finding,hash,allowlist){
 return allowlist.some(entry=>entry.rule===finding.RuleID&&entry.path===finding.File.replaceAll('\\','/')&&entry.line===finding.StartLine&&entry.sha256_lf===hash);
}

if(process.argv[1]&&import.meta.url===pathToFileURL(process.argv[1]).href){
 const root=resolve(dirname(fileURLToPath(import.meta.url)),'..');
 const temporary=mkdtempSync(resolve(tmpdir(),'velora-secret-scan-'));
 const checkout=resolve(temporary,'tracked');
 const report=resolve(temporary,'redacted.json');
 try{
  mkdirSync(checkout);
  const files=execFileSync('git',['ls-files','-z'],{cwd:root,encoding:'utf8'}).split('\0').filter(Boolean);
  for(const file of files){
   const target=resolve(checkout,file),rel=relative(checkout,target);
   if(isAbsolute(rel)||rel.startsWith('..'))throw Error('Invalid tracked path');
   mkdirSync(dirname(target),{recursive:true});copyFileSync(resolve(root,file),target);
  }
  const executable=process.env.GITLEAKS??'gitleaks';
  const scan=spawnSync(executable,['dir','--redact=100','--ignore-gitleaks-allow','--max-archive-depth=2','--no-banner','--log-level=error','--report-format=json','--report-path',report,'.'],{cwd:checkout,encoding:'utf8'});
  if(scan.error||![0,1].includes(scan.status))throw Error('Secret scanner did not complete; install the pinned Gitleaks version.');
  const findings=JSON.parse(readFileSync(report,'utf8'))??[];
  const reviewed=JSON.parse(readFileSync(resolve(root,'docs/security/reviewed-secret-findings.json'),'utf8'));
  const unknown=findings.filter(finding=>{
   const path=resolve(checkout,finding.File),rel=relative(checkout,path);
   if(isAbsolute(rel)||rel.startsWith('..'))return true;
   return !reviewedFinding(finding,normalizedHash(readFileSync(path)),reviewed);
  });
  console.log(JSON.stringify({tracked_files:files.length,reviewed_matches:findings.length-unknown.length,unreviewed_matches:unknown.map(f=>({rule:f.RuleID,path:f.File,line:f.StartLine}))},null,2));
  if(unknown.length)process.exitCode=1;
 }finally{
  // This is the exact private temp directory created above, never a computed user path.
  if(!temporary.startsWith(resolve(tmpdir())+'\\')&&!temporary.startsWith(resolve(tmpdir())+'/'))throw Error('Invalid temporary directory');
  rmSync(temporary,{recursive:true,force:true});
 }
}

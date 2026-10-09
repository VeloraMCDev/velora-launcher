import assert from 'node:assert/strict';
import {readFileSync,readdirSync} from 'node:fs';
import {resolve,dirname} from 'node:path';
import {fileURLToPath} from 'node:url';
import {execFileSync} from 'node:child_process';
import {existsSync} from 'node:fs';
import {homedir} from 'node:os';
const root=resolve(dirname(fileURLToPath(import.meta.url)),'..');
const fallback=resolve(homedir(),'.cargo','bin',process.platform==='win32'?'cargo.exe':'cargo');
const cargoCommand=process.env.CARGO ?? (existsSync(fallback)?fallback:'cargo');
const packages=[];
for(const manifest of ['Cargo.toml','packages/rust-platform-contracts/Cargo.toml','packages/packs-platform-utils/Cargo.toml']){
 const metadata=JSON.parse(execFileSync(cargoCommand,['metadata','--locked','--offline','--no-deps','--format-version','1','--manifest-path',resolve(root,manifest)],{cwd:root,encoding:'utf8',maxBuffer:8*1024*1024}));
 packages.push(...metadata.packages);
}
const key=path=>process.platform==='win32'?resolve(path).toLowerCase():resolve(path);
const byPath=new Map(packages.map(p=>[key(dirname(p.manifest_path)),p]));
function checkDependencies(p,seen=new Set()){
 if(seen.has(p.id))return;seen.add(p.id);
 assert(!/[/\\](private-gameplay|private-storage|panel[/\\]server)[/\\]/.test(p.manifest_path),'Reusable package reaches private/application composition: '+p.name);
 for(const dep of p.dependencies.filter(d=>d.path)){
  const next=byPath.get(key(dep.path));
  assert(next,'Unreviewed local dependency: '+p.name+' -> '+dep.name);
  checkDependencies(next,seen);
 }
}
for(const p of packages){
 if(/[/\\]crates[/\\](auth-|panel-|core[/\\]|shared[/\\]|platform-utils[/\\])/.test(p.manifest_path)||/[/\\]packages[/\\](rust-platform-contracts|packs-platform-utils)[/\\]/.test(p.manifest_path))checkDependencies(p);
}
for(const name of readdirSync(resolve(root,'crates')).filter(n=>n.startsWith('auth-')||n.startsWith('panel-')||['core','platform-utils'].includes(n))){
 const text=readFileSync(resolve(root,'crates',name,'Cargo.toml'),'utf8');
 assert(!/(?:private-gameplay|private-storage|velora-experiences-|velora-panel(?:["\s]|$))/.test(text),'Reusable library depends on private/application composition: '+name);
}
const cargo=readFileSync(resolve(root,'Cargo.toml'),'utf8');
assert(/exclude\s*=\s*\[[^\]]*"infra"/.test(cargo),'Infra must retain its independent Cargo workspace');
for(const dir of ['infra','docs','panel/web','launcher'])assert(JSON.parse(readFileSync(resolve(root,dir,'package.json'),'utf8')).private===true,'Application must prevent accidental npm registry publication: '+dir);
execFileSync(process.execPath,[resolve(root,'scripts/verify-identity.mjs')],{cwd:root,stdio:'inherit'});
console.log('Reusable Rust boundaries and independent Infra workspace verified.');

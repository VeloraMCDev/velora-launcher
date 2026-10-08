import {test} from 'node:test';
import assert from 'node:assert/strict';
import {mkdirSync,mkdtempSync,writeFileSync,unlinkSync} from 'node:fs';
import {execFileSync} from 'node:child_process';
import {resolve,dirname} from 'node:path';
import {fileURLToPath} from 'node:url';
import {createHash} from 'node:crypto';
import {auditRepository} from './public_source_preflight.mjs';
const scratch=resolve(dirname(fileURLToPath(import.meta.url)),'../.runtime-checks/source-preflight-tests');
mkdirSync(scratch,{recursive:true});
function git(root,args) {return execFileSync('git',['-C',root,...args],{stdio:'pipe'});}
function commit(root) {git(root,['add','.']);git(root,['-c','user.name=Velora synthetic test','-c','user.email=test@example.invalid','commit','-m','Synthetic public preflight fixture']);}
function fixture() {
 const root=mkdtempSync(resolve(scratch,'fixture-'));
 git(root,['init','--quiet']);git(root,['remote','add','origin','https://github.com/VeloraMCDev/synthetic-preflight.git']);
 writeFileSync(resolve(root,'README.md'),'Synthetic audit fixture; no production data.\n');commit(root);return root;
}
test('clean tracked snapshot passes and raw-byte drift fails',()=>{
 const root=fixture();mkdirSync(resolve(root,'vendor'));const content='synthetic public contract\n';
 writeFileSync(resolve(root,'vendor/schema.txt'),content);
 writeFileSync(resolve(root,'vendor/SNAPSHOT.json'),JSON.stringify({repository:'https://github.com/VeloraMCDev/sdk',files:[{path:'schema.txt',sha256:createHash('sha256').update(content).digest('hex')}]}));commit(root);
 assert.equal(auditRepository(root).passed,true);
 writeFileSync(resolve(root,'vendor/schema.txt'),'changed\n');commit(root);
 assert.ok(auditRepository(root).findings.some(row=>row.check==='snapshot-drift'));
});
test('deleted credential-shaped synthetic fixture remains detected in reachable history without emitting its value',()=>{
 const root=fixture();const token='gh'+'p_'+'A'.repeat(36);
 writeFileSync(resolve(root,'synthetic-candidate.txt'),token);commit(root);
 unlinkSync(resolve(root,'synthetic-candidate.txt'));commit(root);
 const result=auditRepository(root);
 assert.ok(result.findings.some(row=>row.check==='github-token'));
 assert.equal(JSON.stringify(result).includes(token),false);
});
test('outside-checkout dependency, private workflow dependency and unreviewed artifact fail',()=>{
 const root=fixture();writeFileSync(resolve(root,'Cargo.toml'),'[workspace]\nmembers = ["../private-member"]\n[dependencies]\noutside = { path = "../private" }\n');
 writeFileSync(resolve(root,'package.json'),JSON.stringify({workspaces:['../private-member'],dependencies:{outside:'link:../private'}}));
 mkdirSync(resolve(root,'.github/workflows'),{recursive:true});
 writeFileSync(resolve(root,'.github/workflows/private.yml'),'source: https://github.com/VeloraMCDev/experiences.git\n');
 writeFileSync(resolve(root,'synthetic.jar'),'fixture only; not a real binary');commit(root);
 const kinds=auditRepository(root).findings.map(row=>row.check);
 for(const check of ['cargo-path-outside-checkout','cargo-member-outside-checkout','npm-path-outside-checkout','npm-member-outside-checkout','private-build-or-workflow-dependency','binary-or-database-needs-review']) assert.ok(kinds.includes(check));
});
test('Experiences is refused and remote credential-shaped text is redacted',()=>{
 const root=fixture();git(root,['remote','set-url','origin','https://github.com/VeloraMCDev/experiences.git']);
 assert.throws(()=>auditRepository(root),/must not be a public audit input/);
 git(root,['remote','set-url','origin','https://synthetic-user:synthetic-password@github.com/VeloraMCDev/synthetic-preflight.git']);
 const result=auditRepository(root);assert.equal(result.passed,false);
 assert.equal(JSON.stringify(result).includes('synthetic-password'),false);
});
test('reviewed artifact requires exact bytes and a tracked license',()=>{
 const root=fixture();const content='synthetic build bootstrap; no executable content';
 writeFileSync(resolve(root,'wrapper.jar'),content);writeFileSync(resolve(root,'UPSTREAM_LICENSE'),'Synthetic fixture license\n');
 writeFileSync(resolve(root,'PUBLIC_ARTIFACTS.json'),JSON.stringify({files:[{path:'wrapper.jar',sha256:createHash('sha256').update(content).digest('hex'),license_path:'UPSTREAM_LICENSE',upstream:'https://example.invalid/synthetic-wrapper'}]}));commit(root);
 assert.equal(auditRepository(root).passed,true);
 writeFileSync(resolve(root,'wrapper.jar'),'unexpected bytes');commit(root);
 assert.ok(auditRepository(root).findings.some(row=>row.check==='binary-or-database-needs-review'));
});

import test from 'node:test';
import assert from 'node:assert/strict';
import {normalizedHash,reviewedFinding} from '../../scripts/check-secrets.mjs';

test('reviewed scanner fixtures cannot hide changed content or new findings',()=>{
 const text=Buffer.from('synthetic fixture\n');
 const finding={RuleID:'synthetic-rule',File:'tests/fixture.rs',StartLine:12};
 const hash=normalizedHash(text);
 const allowlist=[{rule:finding.RuleID,path:finding.File,line:12,sha256_lf:hash}];
 assert.equal(reviewedFinding(finding,hash,allowlist),true);
 assert.equal(reviewedFinding(finding,normalizedHash(Buffer.from('changed fixture\n')),allowlist),false);
 assert.equal(reviewedFinding({...finding,StartLine:13},hash,allowlist),false);
 assert.equal(reviewedFinding({...finding,RuleID:'new-secret'},hash,allowlist),false);
 assert.equal(reviewedFinding({...finding,File:'other/path.rs'},hash,allowlist),false);
 assert.equal(normalizedHash(Buffer.from('synthetic fixture\r\n')),hash);
});

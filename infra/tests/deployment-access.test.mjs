import { test } from 'node:test';
import assert from 'node:assert/strict';
import { SignJWT,generateKeyPair,exportJWK,createLocalJWKSet } from 'jose';
import { verifyBootstrapOperator } from '../control-plane/src/access.mjs';

test('bootstrap Access authorization requires signature, issuer, audience and the exact operator', async () => {
  const {privateKey,publicKey} = await generateKeyPair('RS256'); // Ephemeral synthetic fixture only.
  const publicJwk = await exportJWK(publicKey);
  const keys = createLocalJWKSet({keys:[{...publicJwk,kid:'fixture',alg:'RS256'}]});
  const config = {ACCESS_TEAM_DOMAIN:'https://velora-test.cloudflareaccess.com',ACCESS_AUD:'fixture-app',BOOTSTRAP_OPERATOR_EMAIL:'operator@example.com'};
  const token = async (claims={},settings={}) => new SignJWT({email:'operator@example.com',type:'app',...claims})
    .setProtectedHeader({alg:'RS256',kid:'fixture'}).setIssuer(config.ACCESS_TEAM_DOMAIN)
    .setAudience(settings.audience ?? 'fixture-app').setSubject('fixture-user').setIssuedAt()
    .setExpirationTime(settings.expiration ?? '5m').sign(privateKey);
  const request = value => new Request('https://deploy.example.com/api/v1/bootstrap/bindings-check',{headers:{'cf-access-jwt-assertion':value}});
  assert.deepEqual(await verifyBootstrapOperator(request(await token()),config,keys),{subject:'fixture-user',email:'operator@example.com'});
  for (const invalid of [await token({email:'other@example.com'}),await token({type:'org'}),await token({}, {audience:'other-app'}),await token({}, {expiration:1}),'forged.jwt.value']) {
    await assert.rejects(verifyBootstrapOperator(request(invalid),config,keys));
  }
  await assert.rejects(verifyBootstrapOperator(new Request('https://deploy.example.com'),config,keys));
  await assert.rejects(verifyBootstrapOperator(request(await token()),{...config,ACCESS_TEAM_DOMAIN:'https://attacker.example.com'},keys),/NOT_CONFIGURED/);
  await assert.rejects(verifyBootstrapOperator(request(await token()),{...config,ACCESS_TEAM_DOMAIN:'https://wrong-team.cloudflareaccess.com'},keys));
  await assert.rejects(verifyBootstrapOperator(request(await token()),{...config,ACCESS_AUD:''},keys),/NOT_CONFIGURED/);
});

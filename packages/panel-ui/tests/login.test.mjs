import {test} from 'node:test';
import assert from 'node:assert/strict';
import {readFileSync, writeFileSync, mkdirSync} from 'node:fs';
import {createHash} from 'node:crypto';
import ts from 'typescript';
import {compile} from 'svelte/compiler';
import {render} from 'svelte/server';

const root = new URL('../', import.meta.url);
const source = readFileSync(new URL('Login.svelte', root), 'utf8');
const script = source.match(/<script lang="ts">([\s\S]*?)<\/script>/)[1].replace(/^\s*import[^\n]+\n/gm, '');
const executable = ts.transpileModule(script + `
return {submit,register,recover,discordSignIn,
  set(values:any){if('username' in values)username=values.username;if('password' in values)password=values.password;if('email' in values)email=values.email;if('mode' in values)mode=values.mode;if('newPassword' in values)newPassword=values.newPassword;},
  get state(){return {username,password,email,error,busy,mode,notice,discordEnabled}}};`, {compilerOptions:{target:ts.ScriptTarget.ES2022,module:ts.ModuleKind.None}}).outputText;

function harness(options = {}) {
  const calls = [], tokens = [], delays = [], popups = [];
  const location = {hash: options.hash ?? '#/login'};
  const client = {
    auth: {
      login: async body => {calls.push(['login',body]); if(options.loginError) throw options.loginError; return options.loginResult ?? {token:'synthetic'};},
      register: async body => {calls.push(['register',body]); return options.registerResult ?? {token:'synthetic'};},
      forgotPassword: async email => {calls.push(['forgot',email]); return {ok:true,message:'Synthetic generic notice'};},
      resetPassword: async (token,password) => {calls.push(['reset',token,password]);},
    },
    transport: {get: async path => {
      calls.push(['get',path]);
      if(path.endsWith('/config'))return {discord_enabled:true};
      if(path.endsWith('/start'))return {url:'https://example.invalid/oauth',state:'synthetic &state'};
      return options.poll?.shift() ?? {token:'synthetic-discord'};
    }},
  };
  const window = {open(...args){popups.push(args);return options.blockPopup ? null : {close(){popups.push('closed');}};}};
  const page = new Function('$props','$state','location','window','setTimeout',executable)(
    () => ({brandName:'Velora',logo:null,client,onToken:token=>tokens.push(token)}), value => value, location, window,
    (fn,delay) => {delays.push(delay);fn();},
  );
  const event = {preventDefault(){calls.push(['preventDefault']);}};
  return {page,calls,tokens,delays,popups,location,event};
}

test('reviewed component, markup and styles match extraction provenance', () => {
  const p = JSON.parse(readFileSync(new URL('PROVENANCE.json',root)));
  const hash = text => createHash('sha256').update(text.replaceAll('\r\n','\n')).digest('hex');
  assert.equal(hash(source),p.output_sha256_lf);
  assert.equal(hash(source.slice(source.indexOf('</script>'))),p.markup_sha256_lf);
  assert.doesNotMatch(source,/\.\.\/lib\/|localStorage|session\.|experience\.modules/);
});

test('sign-in uses supplied credentials, token lifecycle and original denial text', async () => {
  const h=harness(); h.page.set({username:'Example',password:'input'});
  await h.page.submit(h.event);
  assert.ok(h.calls.some(x=>JSON.stringify(x)===JSON.stringify(['login',{username:'Example',password:'input'}])));
  assert.deepEqual(h.tokens,['synthetic']); assert.equal(h.page.state.busy,false);
  const denied=harness({loginError:new Error('account inactive')});
  await denied.page.submit(denied.event);
  assert.equal(denied.page.state.error,'account inactive');assert.equal(denied.page.state.busy,false);assert.deepEqual(denied.tokens,[]);
});

test('registration retains nullable email and pending approval clears only password', async () => {
  const h=harness({registerResult:{token:'',pending:true}});
  h.page.set({username:'Example',password:'input',email:'  ',mode:'register'});
  await h.page.register(h.event);
  assert.deepEqual(h.calls.at(-1),['register',{username:'Example',password:'input',email:null}]);
  assert.equal(h.page.state.mode,'login');assert.equal(h.page.state.password,'');assert.equal(h.page.state.username,'Example');
  assert.match(h.page.state.notice,/admin needs to approve/);assert.deepEqual(h.tokens,[]);
  const allowed=harness();allowed.page.set({email:' example@example.invalid '});await allowed.page.register(allowed.event);
  assert.equal(allowed.calls.at(-1)[1].email,'example@example.invalid');assert.deepEqual(allowed.tokens,['synthetic']);
});

test('recovery displays generic response; reset reads encoded hash token and returns to login', async () => {
  const h=harness();h.page.set({mode:'forgot',email:'example@example.invalid'});await h.page.recover(h.event);
  assert.equal(h.page.state.notice,'Synthetic generic notice');assert.deepEqual(h.calls.at(-1),['forgot','example@example.invalid']);
  const reset=harness({hash:'#/reset-password?token=synthetic%2Btoken'});reset.page.set({newPassword:'new input'});
  assert.equal(reset.page.state.mode,'reset');await reset.page.recover(reset.event);
  assert.deepEqual(reset.calls.at(-1),['reset','synthetic+token','new input']);assert.equal(reset.location.hash,'#/login');
  assert.equal(reset.page.state.mode,'login');assert.equal(reset.page.state.busy,false);
});

test('Discord pending polls retain timing, state encoding, popup identity and token callback', async () => {
  const h=harness({poll:[{pending:true},{token:'synthetic-discord'}]});await h.page.discordSignIn();
  assert.deepEqual(h.delays,[1500,1500]);assert.deepEqual(h.popups[0],['https://example.invalid/oauth','scopenet-discord','width=520,height=740']);
  assert.equal(h.calls.at(-1)[1],'/api/v1/auth/discord/poll?state=synthetic%20%26state');
  assert.equal(h.popups.at(-1),'closed');assert.deepEqual(h.tokens,['synthetic-discord']);
});

test('Discord blocked popup, approval result and bounded timeout retain notices', async () => {
  const blocked=harness({blockPopup:true});await blocked.page.discordSignIn();assert.match(blocked.page.state.error,/Allow popups/);assert.deepEqual(blocked.delays,[]);
  const pending=harness({poll:[{pending:false}]});await pending.page.discordSignIn();assert.match(pending.page.state.notice,/waiting for approval/);assert.deepEqual(pending.tokens,[]);
  const timeout=harness({poll:Array.from({length:120},()=>({pending:true}))});await timeout.page.discordSignIn();
  assert.equal(timeout.delays.length,120);assert.match(timeout.page.state.error,/timed out/);assert.deepEqual(timeout.tokens,[]);
});

test('actual compiled page retains reset/login fields, registration gate and escaping', async t => {
  const runtime=new URL('.test-runtime/',root);mkdirSync(runtime,{recursive:true});
  // Icons are presentation stubs; compile/render the actual page, all markup and CSS.
  writeFileSync(new URL('Icon.mjs',runtime),compile('<svg aria-hidden="true"></svg>',{generate:'server'}).js.code);
  const compiled=compile(source.replace("import { LogIn, LoaderCircle } from '@lucide/svelte';","import LogIn from './Icon.mjs'; import LoaderCircle from './Icon.mjs';"),{generate:'server',filename:'Login.svelte'});
  writeFileSync(new URL('Login.mjs',runtime),compiled.js.code);
  const Login=(await import(new URL('Login.mjs',runtime))).default;
  const old=globalThis.location;t.after(()=>{if(old===undefined)delete globalThis.location;else globalThis.location=old;});
  globalThis.location={hash:'#/login'};
  const client={transport:{get:async()=>({discord_enabled:false})}};
  const props={brandName:'<script>Velora</script>',logo:null,client,onToken(){},registration:'closed'};
  const login=render(Login,{props}).body;
  assert.match(login,/autocomplete="username"/);assert.match(login,/Forgot password/);assert.doesNotMatch(login,/New here\?/);
  assert.match(login,/&lt;script(?:>|&gt;)Velora/);assert.doesNotMatch(login,/<script>Velora/);
  assert.match(render(Login,{props:{...props,registration:'approval',app:true}}).body,/New here\?/);
  assert.doesNotMatch(render(Login,{props:{...props,app:true}}).body,/ADMIN_PASSWORD/);
  globalThis.location.hash='#/reset-password?token=synthetic';
  const reset=render(Login,{props}).body;assert.match(reset,/autocomplete="new-password"/);assert.doesNotMatch(reset,/autocomplete="username"/);
});

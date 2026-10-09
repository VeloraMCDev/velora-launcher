// Starts only a fresh loopback fixture. Never reuses or removes operator data.
import {spawn} from 'node:child_process';
import {mkdtempSync,existsSync} from 'node:fs';
import {tmpdir} from 'node:os';
import {resolve,join} from 'node:path';
import {randomBytes} from 'node:crypto';
import {createServer} from 'node:net';

const application=resolve(import.meta.dirname,'../..');
const binary=resolve(process.argv[2] ?? `target/debug/scopenet-panel${process.platform==='win32'?'.exe':''}`);
const web=resolve(process.argv[3] ?? 'panel/web/dist');
if(!existsSync(binary)||!existsSync(join(web,'index.html')))throw Error('Build the Panel backend and frontend first; optionally supply binary and web directory paths.');
const data=mkdtempSync(join(tmpdir(),'velora-synthetic-demo-'));
const listener=createServer();
await new Promise((ok,fail)=>listener.once('error',fail).listen(18080,'127.0.0.1',ok));
await new Promise(ok=>listener.close(ok));
const base='http://127.0.0.1:18080';
const env={};
for(const key of ['SystemRoot','WINDIR','PATH','TEMP','TMP','USERPROFILE','APPDATA','LOCALAPPDATA'])if(process.env[key])env[key]=process.env[key];
Object.assign(env,{VELORA_BIND:'127.0.0.1:18080',VELORA_DATA_DIR:data,VELORA_WEB_DIR:web,
 VELORA_ICONS_DIR:resolve(application,'panel/icons/dist'),ADMIN_USERNAME:'admin',ADMIN_PASSWORD:'demo-admin-pass',
 JWT_SECRET:randomBytes(48).toString('hex'),PUBLIC_URL:base,RUST_LOG:'error',DEMO_BASE:base,DEMO_DATA:data,VELORA_DEMO:'synthetic'});
const children=[];
let stopping=false;
function stop(){if(stopping)return;stopping=true;for(const child of children)child.kill();}
process.on('SIGINT',stop);process.on('SIGTERM',stop);process.on('exit',stop);
function start(command,args){const child=spawn(command,args,{cwd:application,env,windowsHide:true,stdio:'inherit'});children.push(child);child.on('error',()=>{stop();process.exitCode=1;});return child;}
async function completed(child){await new Promise((ok,fail)=>{child.once('error',fail);child.once('exit',code=>code===0?ok():fail(Error(`Demo child exited with code ${code}`)));});}
try{
 const panel=start(binary,[]);
 panel.once('exit',()=>stop());
 let ready=false;
 for(let i=0;i<100&&!stopping;i++){
  try{ready=(await fetch(base+'/healthz',{signal:AbortSignal.timeout(1000)})).ok;}catch{}
  if(ready)break;
  if(panel.exitCode!==null)throw Error('Panel exited before becoming ready');
  await new Promise(ok=>setTimeout(ok,200));
 }
 if(!ready)throw Error('Panel health did not become ready');
 await completed(start(process.execPath,['scripts/demo/seed.mjs']));
 start(process.execPath,['scripts/demo/seed.mjs','--heartbeat']);
 console.log(`Synthetic demo: ${base}\nDisposable data: ${data}\nAdmin: admin / demo-admin-pass\nPlayer: Alex_Miner / demo-pass-1234\nPress Ctrl+C to stop. Temporary data is retained for inspection; never use these credentials publicly.`);
}catch(error){stop();throw error;}

import {test} from 'node:test';
import assert from 'node:assert/strict';
import {readFileSync, mkdirSync, writeFileSync} from 'node:fs';
import {createHash} from 'node:crypto';
import ts from 'typescript';
import {compile} from 'svelte/compiler';
import {render} from 'svelte/server';

const root=new URL('../',import.meta.url);
const source=readFileSync(new URL('SkinView.svelte',root),'utf8');
const script=source.match(/<script lang="ts">([\s\S]*?)<\/script>/)[1];
const executable=ts.transpileModule(script+'\nreturn {parts,draw,setCanvas:(value:HTMLCanvasElement)=>canvas=value};',{compilerOptions:{target:ts.ScriptTarget.ES2022,module:ts.ModuleKind.None}}).outputText;
const flush=async()=>{for(let i=0;i<8;i++)await Promise.resolve();};
function harness(props={}){
  const effects=[],images=[],calls=[];
  const ctx={imageSmoothingEnabled:true,fillStyle:'',clearRect(...args){calls.push(['clear',...args]);},fillRect(...args){calls.push(['fill',this.fillStyle,...args]);},drawImage(...args){calls.push(['draw',...args]);},save(){calls.push(['save']);},restore(){calls.push(['restore']);},translate(...args){calls.push(['translate',...args]);},scale(...args){calls.push(['scale',...args]);}};
  class Image {constructor(){images.push(this);}width=64;height=64;}
  const component=new Function('$props','$state','$effect','Image',executable)(()=>({skin:'/synthetic/skin',...props}),x=>x,fn=>effects.push(fn),Image);
  component.setCanvas({width:128,height:256,getContext:type=>{assert.equal(type,'2d');return ctx;}});
  return {component,images,calls,ctx,render:()=>effects[0]()};
}

test('reviewed renderer matches provenance and contains no host identity/policy dependency',()=>{
  const p=JSON.parse(readFileSync(new URL('PROVENANCE.json',root)));
  assert.equal(createHash('sha256').update(source.replaceAll('\r\n','\n')).digest('hex'),p.output_sha256_lf);
  assert.doesNotMatch(source,/localStorage|api\/|session\.|guild|frontiers/i);
});

test('modern front geometry draws all base/overlay regions and slim arms',async()=>{
  const h=harness({slim:true});h.render();h.images[0].onload();await flush();
  const draws=h.calls.filter(x=>x[0]==='draw');assert.equal(draws.length,12);assert.equal(h.ctx.imageSmoothingEnabled,false);
  assert.deepEqual(draws[0].slice(2),[8,8,8,8,32,0,64,64]);
  assert.deepEqual(draws[2].slice(2),[44,20,3,12,8,64,24,96]);
  assert.deepEqual(draws[3].slice(2),[36,52,3,12,96,64,24,96]);
  assert.equal(h.calls.some(x=>x[0]==='scale'),false);
});

test('legacy 64x32 skin mirrors opposite arm/leg and omits modern overlays',async()=>{
  const h=harness();h.render();h.images[0].height=32;h.images[0].onload();await flush();
  assert.equal(h.calls.filter(x=>x[0]==='draw').length,7);
  assert.deepEqual(h.calls.filter(x=>x[0]==='scale'),[['scale',-1,1],['scale',-1,1]]);
  assert.deepEqual(h.calls.filter(x=>x[0]==='translate'),[['translate',128,64],['translate',96,160]]);
  assert.equal(h.calls.filter(x=>x[0]==='save').length,2);assert.equal(h.calls.filter(x=>x[0]==='restore').length,2);
});

test('back geometry overlays HD cape after skin with original cropping/placement',async()=>{
  const h=harness({side:'back',slim:true,cape:'/synthetic/cape'});h.render();h.images[0].onload();await flush();
  assert.equal(h.images.length,2);assert.equal(h.images[1].src,'/synthetic/cape');h.images[1].width=128;h.images[1].onload();await flush();
  const draws=h.calls.filter(x=>x[0]==='draw');assert.equal(draws.length,13);
  assert.deepEqual(draws[0].slice(2),[24,8,8,8,32,0,64,64]);
  assert.deepEqual(draws[2].slice(2),[51,20,3,12,96,64,24,96]);
  assert.deepEqual(draws.at(-1).slice(2),[2,2,20,32,24,64,80,128]);
  const front=harness({cape:'/synthetic/cape'});front.render();front.images[0].onload();await flush();assert.equal(front.images.length,1);
});

test('missing skin retains six-piece silhouettes and each host opacity',async()=>{
  for(const opacity of [0.07,0.08]){
    const h=harness({skin:null,placeholderOpacity:opacity,scale:2});h.render();await flush();
    assert.equal(h.images.length,0);const fills=h.calls.filter(x=>x[0]==='fill');assert.equal(fills.length,6);
    assert.deepEqual(fills[0],['fill',`rgba(255,255,255,${opacity})`,8,0,16,16]);
  }
});

test('failed and cancelled loads cannot paint stale skins or capes',async()=>{
  const denied=harness();denied.render();denied.images[0].onerror(new Error('synthetic unavailable'));await flush();
  assert.equal(denied.calls.filter(x=>x[0]==='draw').length,0);
  const stale=harness();const stop=stale.render();stop();stale.images[0].onload();await flush();assert.equal(stale.calls.filter(x=>x[0]==='draw').length,0);
  const cape=harness({side:'back',cape:'/synthetic/cape'});const cancel=cape.render();cape.images[0].onload();await flush();cancel();cape.images[1].onload();await flush();
  assert.equal(cape.calls.filter(x=>x[0]==='draw').length,12);
});

test('actual Svelte component compiles and retains scaled canvas/accessibility markup',async()=>{
  const runtime=new URL('.test-runtime/',root);mkdirSync(runtime,{recursive:true});
  writeFileSync(new URL('SkinView.mjs',runtime),compile(source,{generate:'server',filename:'SkinView.svelte'}).js.code);
  const SkinView=(await import(new URL('SkinView.mjs',runtime))).default;
  const html=render(SkinView,{props:{skin:null,side:'back',scale:3}}).body;
  assert.match(html,/width="48"/);assert.match(html,/height="96"/);assert.match(html,/aria-label="Skin back"/);
});

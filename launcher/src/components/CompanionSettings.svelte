<script lang="ts">
  import { app, saveSettings } from '../lib/store.svelte';
  import Toggle from './Toggle.svelte';
  const c = $derived(app.settings!.companion);
  const labels: Record<string,string> = {level:'Progression',balance:'Wallet',guild:'Faction',claim:'Territory',quests:'Quest tracker',clock:'Clock'};
  const examples: Record<string,string> = {level:'Level 27 · 62%',balance:'$12,840.50',guild:'[IRON] Iron Wolves',claim:'Iron Wolves territory',quests:'Mine iron · 42 / 64',clock:'21:43'};
  let preview: HTMLDivElement;
  let dragging = $state<string|null>(null);
  function move(e: PointerEvent) {
    if (!dragging) return;
    const w = c.widgets.find(w=>w.id===dragging); if (!w) return;
    const rect=preview.getBoundingClientRect();
    const card=preview.querySelector<HTMLElement>(`[data-widget="${w.id}"]`);
    const cardW=card?.offsetWidth??120, cardH=card?.offsetHeight??38;
    w.x=Math.max(0,Math.min(1,(e.clientX-rect.left-cardW/2)/Math.max(1,rect.width-cardW)));
    w.y=Math.max(0,Math.min(1,(e.clientY-rect.top-cardH/2)/Math.max(1,rect.height-cardH)));
  }
  function end() { if(dragging){dragging=null;saveSettings();} }
  function reset() { c.widgets.forEach((w,i)=>{const defaults=[[.02,.04],[.76,.04],[.02,.8],[.4,.04],[.76,.3],[.76,.8]];[w.x,w.y]=defaults[i]??[0,0];w.enabled=w.id!=='clock';w.scale=1;});c.scale=1;c.opacity=.88;saveSettings(); }
</script>

<svelte:window onpointermove={move} onpointerup={end} onpointercancel={end}/>
<h1>Velora Companion</h1>
<p class="lead">Build your in-game interface. Drag modules into place, choose what matters, and set quick actions in Controls.</p>
<div class="card glass col">
  <Toggle bind:checked={c.enabled} onchange={saveSettings} label="Enable companion interface" help="Requires the Velora Companion and Fabric API in a Fabric 26.3 instance."/>
  <Toggle bind:checked={c.notifications} onchange={saveSettings} label="In-game notifications" help="Level-ups, achievements and faction events."/>
  <Toggle bind:checked={c.claimBorders} onchange={saveSettings} label="Show nearby claim borders" help="Green outlines show allowed territory; red outlines show restricted claims."/>
  <label class="field"><span>HUD scale · {Math.round(c.scale*100)}%</span><input type="range" min=".5" max="2" step=".05" bind:value={c.scale} oninput={saveSettings}/></label>
  <label class="field"><span>Panel opacity · {Math.round(c.opacity*100)}%</span><input type="range" min=".2" max="1" step=".01" bind:value={c.opacity} oninput={saveSettings}/></label>
</div>
<div class="layout-heading"><h2>Your HUD layout</h2><button onclick={reset}>Reset layout</button></div>
<div class="hud-preview" bind:this={preview} style={`--hud-opacity:${c.opacity};--hud-accent:${app.manifest?.branding.colors.accent??'#d4ae65'}`}>
  <div class="scene-label">HUD layout preview · drag modules</div>
  <div class="crosshair">+</div><div class="hotbar">1 · 2 · 3 · 4 · 5 · 6 · 7 · 8 · 9</div>
  {#each c.widgets.filter(w=>w.enabled) as w (w.id)}
    <button class="hud-module" class:dragging={dragging===w.id} data-widget={w.id}
      style={`left:calc(${w.x*100}% - ${w.x*144*(w.scale??1)}px);top:calc(${w.y*100}% - ${w.y*42*(w.scale??1)}px);transform:scale(${w.scale??1});transform-origin:top left`}
      onpointerdown={(e)=>{e.preventDefault();dragging=w.id;(e.currentTarget as HTMLElement).setPointerCapture(e.pointerId);}}
      onkeydown={(e)=>{const d=.02;if(e.key.startsWith('Arrow')){e.preventDefault();w.x=Math.max(0,Math.min(1,w.x+(e.key==='ArrowRight'?d:e.key==='ArrowLeft'?-d:0)));w.y=Math.max(0,Math.min(1,w.y+(e.key==='ArrowDown'?d:e.key==='ArrowUp'?-d:0)));saveSettings();}}}
      aria-label={`${labels[w.id]} position. Drag or use arrow keys.`}>
      <span>{labels[w.id]}</span><strong>{examples[w.id]}</strong>
    </button>
  {/each}
</div>
<div class="modules card glass">
  {#each c.widgets as w (w.id)}
    <div class="module-row"><Toggle bind:checked={w.enabled} onchange={saveSettings} label={labels[w.id]}/>
      <label class="size" title="Size of this module"><span>{Math.round((w.scale??1)*100)}%</span><input type="range" min=".5" max="2.5" step=".05" value={w.scale??1} disabled={!w.enabled} oninput={(e)=>{w.scale=Number(e.currentTarget.value);saveSettings();}} aria-label={`${labels[w.id]} size`}/></label></div>
  {/each}
</div>
<p class="help">Brand name and accent follow your launcher theme. Preferences apply on the next launch. The companion also reloads this file while playing, so administrators can update presentation without changing gameplay rules.</p>

<style>
  .module-row{display:flex;align-items:center;justify-content:space-between;gap:1rem}.module-row .size{display:flex;align-items:center;gap:.5rem;font-size:.8rem;color:var(--muted,#8b8f9a)}.module-row .size input{width:140px}
  .layout-heading{display:flex;align-items:center;justify-content:space-between;margin:1.5rem 0 .7rem}.layout-heading h2{margin:0;font-size:1.1rem}
  .hud-preview{position:relative;height:340px;border:1px solid var(--border);background:#27312b;overflow:hidden;border-radius:8px;touch-action:none;}
  .scene-label{position:absolute;left:12px;bottom:12px;font-size:11px;color:#b6c3b8}.crosshair{position:absolute;left:50%;top:50%;color:#e6e9df;font:22px monospace}.hotbar{position:absolute;bottom:14px;left:50%;transform:translateX(-50%);border:2px solid #83867c;padding:8px;color:#ddd;background:#252923;font:13px monospace;white-space:nowrap}
  .hud-module{position:absolute;width:144px;height:42px;display:flex;flex-direction:column;align-items:flex-start;justify-content:center;gap:3px;padding:5px 8px;border:1px solid #777567;border-radius:0;background:rgb(23 27 32 / var(--hud-opacity));font:11px monospace;cursor:grab;color:#eee8dd;overflow:hidden;touch-action:none}
  .hud-module span{color:var(--hud-accent);text-transform:uppercase;font-size:9px}.hud-module strong{font-weight:normal;white-space:nowrap}.hud-module.dragging{cursor:grabbing;outline:2px solid var(--hud-accent);z-index:2}.modules{display:grid;grid-template-columns:repeat(2,1fr);gap:12px;margin-top:12px;padding:16px}
</style>

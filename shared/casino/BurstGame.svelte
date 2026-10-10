<script lang="ts">
 import { Zap } from '@lucide/svelte';
 import { untrack } from 'svelte';
 import BetInput from './BetInput.svelte'; import GameShell from './GameShell.svelte';
 import { cget,cpost,money,mult,type CasinoState,type BurstGame } from './casino'; import {errorText} from './host';
 let {st,balance,onbalance,onplayed}:{st:CasinoState;balance:number|null;onbalance:(v:number)=>void;onplayed:()=>void}=$props();
 let bet=$state(untrack(()=>st.config.burst.min_bet)), game=$state<BurstGame|null>(untrack(()=>st.burst)), busy=$state(false),error=$state('');
 let pending=$state<{path:string;body:Record<string,unknown>}|null>(null);
 const active=$derived(game?.status==='active');
 async function act(action:string){if(busy)return;if(!pending&&action==='start'&&(!Number.isFinite(bet)||bet<st.config.burst.min_bet||bet>st.config.burst.max_bet)){error='Choose a wager within the table limits.';return;}busy=true;error='';pending??={path:'/burst/'+action,body:{operation_id:crypto.randomUUID(),...(action==='start'?{bet}:{id:game?.id,expected_steps:game?.steps})}};
  try{const r=await cpost<{game:BurstGame;balance:number}>(st.server.id,pending.path,pending.body);game=r.game;pending=null;onbalance(r.balance);onplayed();}catch(e){error=errorText(e);}finally{busy=false;}}
 async function recover(){if(busy)return;busy=true;try{const saved=await cget<CasinoState>(st.server.id);game=saved.burst;pending=null;error='';if(saved.balance!=null)onbalance(saved.balance);onplayed();}catch(e){error=errorText(e);}finally{busy=false;}}
</script>
<GameShell title="Burst" icon={Zap}>
 {#snippet controls()}
  <BetInput bind:value={bet} min={st.config.burst.min_bet} max={st.config.burst.max_bet} {balance} disabled={busy||active||!!pending}/>
  {#if pending}<button onclick={()=>act('retry')} disabled={busy}>Retry same action</button>
  {:else if active}<button onclick={()=>act('advance')} disabled={busy||!st.config.burst.enabled}>Advance · {(100*(game?.survival??0)).toFixed(0)}% survival</button><button class="cash" onclick={()=>act('cashout')} disabled={busy}>Cash out {money(game?.cashout)}</button>
  {:else}<button onclick={()=>act('start')} disabled={busy||!st.config.burst.enabled}>Start ladder</button>{/if}
  <p>Each step risks the whole stake. Survive to climb, or cash out any time. The final step cashes out automatically. Leaving the page preserves your round.</p>
  {#if error}<p role="alert" class="error">{error}</p><button onclick={recover} disabled={busy}>Reload saved round</button>{/if}
 {/snippet}
 {#snippet stage()}
  <div class="stage"><span>THE RISK LADDER</span><strong>{game?mult(game.multiplier):'1×'}</strong><p>{game?.status==='lost'?'Burst! Your stake was lost.':game?.status==='cashed'?'Cashed out '+money(game.cashout):'Advance or take your winnings.'}</p>
  <div class="ladder">{#each Array.from({length:game?.max_steps??st.config.burst.max_steps},(_,i)=>i+1) as step}<div class:reached={step<=(game?.steps??0)}>{step}<span>{step<=(game?.steps??0)?'✓':'·'}</span></div>{/each}</div></div>
 {/snippet}
</GameShell>
<style>button{display:block;width:100%;padding:1rem;margin-top:1rem;border:1px solid #ba83ff44;border-radius:10px;color:white;background:#7850b0;font-weight:700}.cash{background:#17604d}p{color:#aaa;font-size:.85rem;line-height:1.6}.error{color:#ff9daa}.stage{padding:2rem;text-align:center;border-radius:24px;background:radial-gradient(ellipse at top,#67408c55,transparent 70%)}.stage>span{font-size:.7rem;color:#c29be4;letter-spacing:.2em}.stage strong{display:block;font-size:4rem;color:#e8c6ff;margin:1rem}.ladder{display:grid;grid-template-columns:repeat(4,1fr);gap:.7rem;margin-top:2rem}.ladder div{display:flex;justify-content:space-between;padding:1rem;border:1px solid #ffffff18;border-radius:10px;color:#8e819b;background:#21192a}.ladder div.reached{border-color:#9ae5ba;color:#9ae5ba;background:#16382b}</style>

<script lang="ts">
  import { CircleDollarSign } from '@lucide/svelte';
  import { untrack } from 'svelte';
  import BetInput from './BetInput.svelte';
  import GameShell from './GameShell.svelte';
  import { cpost, money, type CasinoState, type Round } from './casino';
  import { errorText } from './host';
  let { st, balance, onbalance, onplayed }: {st:CasinoState;balance:number|null;onbalance:(v:number)=>void;onplayed:()=>void}=$props();
  let bet=$state(untrack(()=>st.config.roulette.min_bet)), selection=$state('red'), number=$state(0), busy=$state(false), error=$state('');
  let result=$state<Round|null>(null);
  let pending=$state<{operation_id:string;bet:number;selection:string;number:number}|null>(null);
  const red=(n:number)=>[1,3,5,7,9,12,14,16,18,19,21,23,25,27,30,32,34,36].includes(n);
  async function spin(){if(busy)return;if(!pending&&(!Number.isFinite(bet)||bet<st.config.roulette.min_bet||bet>st.config.roulette.max_bet||(selection==='straight'&&(!Number.isInteger(number)||number<0||number>36)))){error='Choose a valid wager and a number from 0 to 36.';return;}busy=true;error='';pending??={operation_id:crypto.randomUUID(),bet,selection,number};
    try {result=await cpost<Round>(st.server.id,'/roulette',pending);pending=null;onbalance(result.balance);onplayed();}
    catch(e){error=errorText(e);}finally{busy=false;}}
</script>
<GameShell title="Roulette" icon={CircleDollarSign}>
 {#snippet controls()}
  <BetInput bind:value={bet} min={st.config.roulette.min_bet} max={st.config.roulette.max_bet} {balance} disabled={busy||!!pending}/>
  <label>Bet on<select bind:value={selection} disabled={busy||!!pending}><option value="straight">Single number · 36×</option><option value="red">Red · 2×</option><option value="black">Black · 2×</option><option value="odd">Odd · 2×</option><option value="even">Even · 2×</option><option value="low">1–18 · 2×</option><option value="high">19–36 · 2×</option><option value="first">1–12 · 3×</option><option value="second">13–24 · 3×</option><option value="third">25–36 · 3×</option></select></label>
  {#if selection==='straight'}<label>Number<input type="number" min="0" max="36" step="1" bind:value={number} disabled={busy||!!pending}/></label>{/if}
  <button class="wager-button" onclick={spin} disabled={busy||!st.config.roulette.enabled}>{busy?'Spinning…':pending?'Retry same spin':'Spin roulette'}</button>
  <p>European single-zero roulette. Multipliers include your stake. Zero loses every outside bet.</p>
  {#if error}<p role="alert" class="error">{error} Retrying uses the same wager.</p>{/if}
 {/snippet}
 {#snippet stage()}
  <div class="wheel" class:red={result&&red(result.result.number)} class:black={result&&result.result.number!==0&&!red(result.result.number)}><span>WINNING NUMBER</span><strong>{result?result.result.number:'?'}</strong><span>{result?money(result.payout)+' returned':'Place your bet'}</span></div>
  <div class="numbers">{#each Array.from({length:37},(_,i)=>i) as n}<button class:red={red(n)} class:zero={n===0} class:selected={selection==='straight'&&number===n} disabled={busy||!!pending} onclick={()=>{selection='straight';number=n;}}>{n}</button>{/each}</div>
 {/snippet}
</GameShell>
<style>label{display:grid;gap:.5rem;margin:1rem 0}select,input{padding:.7rem;border:1px solid #ffffff22;background:#171423;color:#eee;border-radius:8px}.wager-button{width:100%;padding:1rem;border:0;border-radius:12px;background:#9671dd;color:white;font-weight:700}p{font-size:.8rem;color:#aaa;line-height:1.6}.error{color:#ff9daa}.wheel{margin:1rem auto 2rem;width:220px;height:220px;border-radius:50%;border:12px double #b8965f;background:radial-gradient(circle,#174c37,#101d19);display:flex;flex-direction:column;align-items:center;justify-content:center;box-shadow:0 0 60px #9b77dc22}.wheel.red{background:radial-gradient(circle,#8d263b,#25131a)}.wheel.black{background:radial-gradient(circle,#34313b,#101016)}.wheel strong{font-size:5rem;color:white}.wheel span{font-size:.7rem;color:#d9c8ab;letter-spacing:.1em}.numbers{display:grid;grid-template-columns:repeat(9,1fr);gap:5px;max-width:620px;margin:auto}.numbers button{padding:.7rem .2rem;background:#25232d;border:1px solid #ffffff20;color:white;border-radius:5px}.numbers button.red{background:#75263b}.numbers button.zero{background:#176244}.numbers button.selected{outline:2px solid #e5c16b}</style>

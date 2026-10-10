<script lang="ts">
  import { untrack } from 'svelte';
  import { errorText, money } from './host';
  type Track = { tiers: number; max: number; next_price_cents: number | null };
  type View = { tracks: Record<string, Track>; claims_per_tier: number; members_per_tier: number; bank_cents: number; outposts: number; outpost_limit: number };
  let { guildId, canManage, get, buy, onchanged }: {
    guildId: string; canManage: boolean; get: (guild: string) => Promise<View>;
    buy: (guild: string, track: string, tiers: number, price: number) => Promise<unknown>; onchanged?: () => void;
  } = $props();
  let view = $state<View | null>(null), error = $state(''), busy = $state(false);
  let review = $state<{ track: string; tiers: number; price: number } | null>(null);
  let generation = 0;
  const names: Record<string, string> = { claims: 'Claim capacity', members: 'Member capacity', outposts: 'Outpost Flag', vault: 'Faction vault page' };
  async function load(id = guildId, current = generation) {
    try { const result = await get(id); if (current === generation) { view = result; error = ''; } }
    catch (e) { if (current === generation) error = errorText(e); }
  }
  $effect(() => { const id = guildId, current = ++generation; view = null; review = null; untrack(() => void load(id, current)); return () => { ++generation; }; });
  async function confirm() {
    if (!review || busy) return;
    const pending = review, id = guildId, current = generation;
    busy = true;
    try {
      await buy(id, pending.track, pending.tiers, pending.price);
      if (current === generation) { review = null; await load(id, current); onchanged?.(); }
    } catch (e) { if (current === generation) { review = null; await load(id, current); error = errorText(e); } }
    finally { busy = false; }
  }
</script>

<section class="market">
  <header><span class="eyebrow">VELORA SMP / FACTION MARKET</span><h2>Build your faction’s future</h2><p>Upgrades are paid from the faction bank. Leaders and roles with management permission can purchase them.</p></header>
  {#if error}<p class="error" role="alert">{error}</p><button disabled={busy} onclick={() => load()}>Reload</button>{/if}
  {#if view}<p>Bank: <strong>{money(view.bank_cents / 100)}</strong> · Placed outposts: {view.outposts} / {view.outpost_limit}</p>
    <div class="tracks">{#each Object.entries(view.tracks) as [track, terms]}
      <article><h3>{names[track] ?? track}</h3><p>{track === 'claims' ? `Add ${view.claims_per_tier} claim capacity` : track === 'members' ? `Add ${view.members_per_tier} member spaces` : track === 'vault' ? 'Add a shared vault page' : 'A flag delivered to your vault. Hold it and use /faction outpost place. Claims stay within 3 chunks of the flag, up to 12 chunks per outpost.'}</p><small>Purchased {terms.tiers} / {terms.max}</small>
        {#if terms.next_price_cents != null}<button disabled={busy || !canManage || !!review || terms.next_price_cents > view.bank_cents} onclick={() => review = { track, tiers: terms.tiers, price: terms.next_price_cents! }}>Review · {money(terms.next_price_cents / 100)}</button>{:else}<p>Maximum reached</p>{/if}
      </article>
    {/each}</div>
    {#if review}<aside><strong>Buy {names[review.track]} for {money(review.price / 100)} from the faction bank?</strong><div><button disabled={busy || !canManage} onclick={confirm}>{busy ? 'Purchasing…' : 'Confirm upgrade'}</button><button disabled={busy} onclick={() => review = null}>Cancel</button></div></aside>{/if}
  {:else if !error}<p>Loading upgrades…</p>{/if}
</section>

<style>
  .market { color: var(--text, #f0edf8); display: grid; gap: 1rem; } .eyebrow { color: #be9eea; font-size: .7rem; letter-spacing: .15em; }
  h2 { margin: .6rem 0; } h3 { margin: 0; } p, small { color: var(--muted, #a99bbf); line-height: 1.5; }
  .tracks { display: grid; grid-template-columns: repeat(auto-fit, minmax(min(100%, 220px), 1fr)); gap: 1rem; }
  article, aside { padding: 1.2rem; border: 1px solid #a386cd33; background: var(--surface, #171421); border-radius: 16px; } article { display: flex; flex-direction: column; gap: .8rem; } article p { flex: 1; margin: 0; }
  button { font: inherit; color: inherit; border: 1px solid #a386cd55; background: #a386cd18; border-radius: 8px; padding: .7rem; cursor: pointer; } button:disabled { opacity: .5; cursor: default; }
  aside { border-color: #be9eea; } aside div { margin-top: 1rem; display: flex; gap: .7rem; } .error { color: #f2a9b3; }
</style>

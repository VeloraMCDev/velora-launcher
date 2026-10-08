<script lang="ts">
  import { Settings, Plus, Check, LogOut, Home, Trophy, Target, Shield, Users, Terminal, Store, Dice5, Award } from '@lucide/svelte';
  import { navigation } from '@scopenet/experience';
  import { selectedInstance } from '../lib/store.svelte';
  import Avatar from './Avatar.svelte';
  import { abs, activeAccount, app, instances, removeAccount, selectInstance, switchAccount } from '../lib/store.svelte';

  const pages = $derived(navigation(selectedInstance()?.experience, [
    { id: 'home', label: 'Overview' }, { id: 'quests', label: 'Quests & Objectives' },
    { id: 'collections', label: 'Collection' }, { id: 'guilds', label: 'Guilds & Claims' },
    { id: 'market', label: 'Market & Auctions' }, { id: 'casino', label: 'Casino' },
    { id: 'commands', label: 'Command guide' }, { id: 'stats', label: 'Stats & Leaderboards' },
    ...(selectedInstance()?.experience?.widgets.length ? [{ id: 'experience', label: 'Experience' }] : []),
  ]));
  const icons: Record<string, typeof Home> = { home: Home, quests: Target, collections: Award, guilds: Shield, market: Store, casino: Dice5, commands: Terminal, stats: Trophy, experience: Home };
  let menu = $state(false);
  const kindLabel = { panel: 'Account', offline: 'Offline' };
  const initials = (name: string) => name.split(/\s+/).map((w) => w[0]).join('').slice(0, 2).toUpperCase();
</script>

<svelte:window onclick={(e) => menu && !(e.target as HTMLElement).closest('.account') && (menu = false)} />

<nav class="rail glass">
  <div class="list">
    {#each instances() as inst (inst.id)}
      <button
        class="inst"
        class:active={app.selected === inst.id}
        class:running={app.running.some((g) => g.instance_id === inst.id)}
        onclick={() => { selectInstance(inst.id); app.view = 'home'; }}
        aria-label={inst.name}
      >
        {#if inst.icon_url}<img src={abs(inst.icon_url)} alt="" />{:else}<span>{initials(inst.name)}</span>{/if}
        <em class="tip">{inst.name}</em>
      </button>
    {/each}
  </div>

  <div class="bottom">
    {#each pages as page (page.id)}
      {@const Icon = icons[page.id] ?? Home}
      <button class="ghost icon big" class:on={app.view === page.id} onclick={() => (app.view = page.id as typeof app.view)} aria-label={page.label} title={page.label}><Icon size={19} /></button>
    {/each}
    <button class="ghost icon big" class:on={app.view === 'social'} onclick={() => (app.view = 'social')} aria-label="Friends & Social" title="Friends & Social">
      <Users size={19} />
    </button>


    <button class="ghost icon big" class:on={app.view === 'settings'} onclick={() => (app.view = 'settings')} aria-label="Settings" title="Settings">
      <Settings size={20} />
      {#if app.update}<span class="dot"></span>{/if}
    </button>
    <div class="account">
      <button class="me" onclick={() => (menu = !menu)} aria-label="Accounts">
        <Avatar account={activeAccount()} size={2.3} />
      </button>
      {#if menu}
        <div class="menu glass">
          <span class="tiny muted head">Accounts</span>
          {#each app.accounts as a (a.id)}
            <div class="acc" class:current={a.id === app.active}>
              <button class="pick" onclick={() => { switchAccount(a.id); menu = false; }}>
                <Avatar account={a} size={1.9} />
                <span class="who"><strong>{a.username}</strong><span class="tiny muted">{kindLabel[a.kind]}</span></span>
                {#if a.id === app.active}<Check size={16} />{/if}
              </button>
              <button class="ghost icon" title="Sign out" aria-label="Sign out {a.username}" onclick={() => removeAccount(a.id)}><LogOut size={14} /></button>
            </div>
          {/each}
          <button class="add" onclick={() => { app.addAccount = true; menu = false; }}><Plus size={16} /> Add account</button>
        </div>
      {/if}
    </div>
  </div>
</nav>

<style>
  .rail { width: 4.5rem; display: flex; flex-direction: column; align-items: center; padding: 0.75rem 0 0.9rem; gap: 0.6rem; border: none; border-right: 1px solid var(--line); border-radius: 0; background: color-mix(in srgb, var(--surface) 60%, transparent); backdrop-filter: blur(14px); -webkit-backdrop-filter: blur(14px); position: relative; z-index: 20; flex-shrink: 0; }
  .list { display: flex; flex-direction: column; gap: 0.65rem; align-items: center; overflow-y: auto; overflow-x: visible; flex: 1; width: 100%; padding: 0.2rem 0; scrollbar-width: none; }
  .inst {
    width: 3rem; height: 3rem; padding: 0; border-radius: var(--radius); position: relative; flex-shrink: 0; overflow: visible;
    background: color-mix(in srgb, var(--text) 6%, transparent); border: 1px solid transparent; font-weight: 600; font-size: 0.85rem;
    color: color-mix(in srgb, var(--text) 75%, transparent); transition: transform 0.15s cubic-bezier(0.2, 0.8, 0.2, 1), background 0.15s, border-color 0.15s;
  }
  .inst img { width: 100%; height: 100%; object-fit: cover; border-radius: inherit; }
  .inst:hover { transform: scale(1.04); background: color-mix(in srgb, var(--text) 12%, transparent); color: var(--text); border-color: var(--line-strong); }
  .inst.active { background: color-mix(in srgb, var(--accent) 18%, transparent); color: var(--text); border-color: color-mix(in srgb, var(--accent) 55%, transparent); box-shadow: 0 0 16px color-mix(in srgb, var(--accent) 30%, transparent); }
  .inst.active::before { content: ''; position: absolute; left: -0.75rem; top: 0.65rem; bottom: 0.65rem; width: 3.5px; border-radius: 0 3px 3px 0; background: var(--accent); box-shadow: 0 0 8px var(--accent); }
  .inst.running::after { content: ''; position: absolute; right: -0.2rem; bottom: -0.2rem; width: 0.8rem; height: 0.8rem; border-radius: 50%; background: var(--success); border: 2px solid var(--bg); }
  .tip { position: fixed; left: 5.2rem; transform: translateY(0); font-style: normal; font-size: 0.82rem; font-weight: 600; max-width: 16rem; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; padding: 0.4rem 0.75rem; border-radius: 0.5rem; background: var(--surface); border: 1px solid var(--line-strong); box-shadow: 0 4px 16px rgba(0, 0, 0, 0.4); opacity: 0; pointer-events: none; transition: opacity 0.15s; z-index: 50; color: var(--text); }
  .inst:hover .tip { opacity: 1; }
  .bottom { display: flex; flex-direction: column; align-items: center; gap: 0.4rem; }
  .big { width: 2.6rem; height: 2.6rem; position: relative; border-radius: var(--radius-sm); transition: transform 0.12s; }
  .big:hover { transform: scale(1.05); }
  .big.on { color: var(--text); background: color-mix(in srgb, var(--text) 12%, transparent); border-color: var(--line-strong); }
  .dot { position: absolute; top: 0.55rem; right: 0.55rem; width: 0.5rem; height: 0.5rem; border-radius: 50%; background: var(--accent); }
  .account { position: relative; margin-top: 0.3rem; }
  .me { padding: 0.15rem; border-radius: 30%; background: transparent; border-color: transparent; }
  .me:hover { border-color: var(--line-strong); background: transparent; }
  .menu { position: absolute; left: calc(100% + 0.9rem); bottom: 0; width: 17rem; padding: 0.6rem; display: flex; flex-direction: column; gap: 0.25rem; background: var(--surface); backdrop-filter: blur(14px); -webkit-backdrop-filter: blur(14px); border: 1px solid var(--line-strong); border-radius: var(--radius); box-shadow: 0 0.8rem 2.5rem -0.5rem rgba(0, 0, 0, 0.6); animation: fade 0.12s ease; z-index: 60; }
  .head { padding: 0.2rem 0.5rem 0.4rem; text-transform: uppercase; letter-spacing: 0.08em; font-weight: 700; }
  .acc { display: flex; align-items: center; gap: 0.2rem; border-radius: var(--radius-sm); }
  .acc.current { background: color-mix(in srgb, var(--text) 6%, transparent); }
  .pick { flex: 1; justify-content: flex-start; background: transparent; border-color: transparent; padding: 0.45rem 0.5rem; gap: 0.65rem; min-width: 0; }
  .pick :global(svg) { color: var(--accent); }
  .who { display: flex; flex-direction: column; align-items: flex-start; flex: 1; min-width: 0; }
  .who strong { font-size: 0.9rem; overflow: hidden; text-overflow: ellipsis; max-width: 100%; }
  .add { margin-top: 0.3rem; border-style: dashed; background: transparent; }
</style>

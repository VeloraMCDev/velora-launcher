<script lang="ts">
  import SignIn from '../components/SignIn.svelte';
  import { abs, app } from '../lib/store.svelte';

  const b = $derived(app.manifest?.branding);
</script>

<div class="login">
  <section class="brand">
    {#if b?.logo_url}<img src={abs(b.logo_url)} alt="" class="logo" />{/if}
    <h1>{b?.name}</h1>
    <p>{b?.tagline}</p>
  </section>
  <section class="card glass">
    <h2>Welcome</h2>
    <p class="muted small">Sign in to start playing.</p>
    <SignIn />
  </section>
</div>

<style>
  .login { height: 100%; display: grid; grid-template-columns: 1fr 23rem; align-items: center; gap: 4rem; padding: 2rem 4.5rem 3rem; }
  .brand { display: flex; flex-direction: column; gap: 0.8rem; animation: fade 0.3s ease; }
  .logo { width: 3.5rem; height: 3.5rem; object-fit: contain; margin-bottom: 0.5rem; }
  h1 { font-size: clamp(2.4rem, 4.5vw, 3.4rem); font-weight: 700; letter-spacing: 0.02em; line-height: 1.05; }
  .brand p { font-size: 1rem; color: var(--muted); max-width: 26rem; line-height: 1.55; }
  .card { padding: 1.6rem; display: flex; flex-direction: column; gap: 0.35rem; background: var(--surface); animation: fade 0.3s ease both; }
  .card h2 { font-size: 1.2rem; font-weight: 600; }
  .card > :global(.signin) { margin-top: 0.8rem; }
  @media (max-width: 900px) { .login { grid-template-columns: 1fr; padding: 2rem; } .brand { display: none; } }
</style>

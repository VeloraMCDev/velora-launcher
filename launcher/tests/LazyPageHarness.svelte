<script lang="ts">
  import type { Component } from 'svelte';
  import LazyPage from '../src/components/LazyPage.svelte';
  import FixturePage from './LazyFixturePage.svelte';
  import type { LazyPageName } from '../src/lib/pages';

  let name = $state<LazyPageName>('stats');
  let mounted = $state(true);
  let events = $state<string[]>([]);
  const requests: { name: string; resolve: (module: { default: Component }) => void; reject: (error: Error) => void }[] = [];
  let requestCount = $state(0);
  function load(page: LazyPageName) {
    requestCount++;
    return new Promise<{ default: Component }>((resolve, reject) => requests.push({ name: page, resolve, reject }));
  }
  function resolveAt(index: number) {
    const request = requests[index];
    if (!request) return;
    const label = request.name;
    const SyntheticComponent: Component = (anchor, properties) => FixturePage(anchor, { ...properties, label, record: (event: string) => events.push(event) });
    request.resolve({ default: SyntheticComponent });
  }
</script>

<h1>Synthetic lazy-page fixture — no native launcher or live data</h1>
<button onclick={() => name = 'market'}>Switch to market</button>
<button onclick={() => name = 'stats'}>Switch to stats</button>
<button onclick={() => resolveAt(0)}>Resolve first request</button>
<button onclick={() => resolveAt(requests.length - 1)}>Resolve latest request</button>
<button onclick={() => requests.at(-1)?.reject(Error('synthetic import failure'))}>Reject latest request</button>
<button onclick={() => mounted = !mounted}>Toggle mounted page</button>
<output aria-label="Fixture requests">{requestCount}</output>
<output aria-label="Fixture lifecycle">{events.join(', ')}</output>
{#if mounted}<LazyPage {name} {load} />{/if}

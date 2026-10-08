<script lang="ts">
  import { itemEmoji, itemGradient, prettyItem } from './money-pages';
  // A tasteful stand-in for the launcher's texture tile: a gradient square from the item id, an emoji or initial, and the amount.
  let { id, amount = 1, size = 48 }: { id: string; amount?: number; size?: number } = $props();
  const emoji = $derived(itemEmoji(id));
  const initial = $derived(prettyItem(id).slice(0, 1));
</script>

<span class="tile" style:width="{size}px" style:height="{size}px" style:background={itemGradient(id)} style:font-size="{size * 0.5}px" title={prettyItem(id)}>
  {#if emoji}<span class="e">{emoji}</span>{:else}<b>{initial}</b>{/if}
  {#if amount > 1}<i>{amount > 999 ? Math.floor(amount / 1000) + 'k' : amount}</i>{/if}
</span>

<style>
  .tile { position: relative; display: inline-grid; place-items: center; border-radius: 28%; flex-shrink: 0; color: #fff; box-shadow: inset 0 0 0 1px rgba(255, 255, 255, 0.18), inset 0 -10px 18px -10px rgba(0, 0, 0, 0.5), 0 8px 16px -10px #000; }
  b { font-weight: 800; text-shadow: 0 2px 6px rgba(0, 0, 0, 0.4); }
  .e { filter: drop-shadow(0 2px 4px rgba(0, 0, 0, 0.4)); line-height: 1; }
  i { position: absolute; right: -4px; bottom: -4px; font-style: normal; font-size: 0.68rem; font-weight: 700; line-height: 1; padding: 3px 5px; border-radius: 7px; background: rgba(8, 10, 16, 0.88); border: 1px solid rgba(255, 255, 255, 0.18); }
</style>

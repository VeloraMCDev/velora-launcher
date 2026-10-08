import { defineConfig } from 'vite';
import { svelte } from '@sveltejs/vite-plugin-svelte';
import { fileURLToPath } from 'node:url';

// In dev, API calls are proxied to a locally running panel (cargo run -p scopenet-panel).
const target = process.env.PANEL_URL ?? 'http://localhost:8080';

export default defineConfig({
  plugins: [svelte()],
  // The map viewer is shared with the launcher (repo root: shared/map).
  resolve: { dedupe: ['svelte', '@lucide/svelte'], alias: { '@scopenet/experience': fileURLToPath(new URL('../../shared/experience', import.meta.url)), '@scopenet/map': fileURLToPath(new URL('../../shared/map', import.meta.url)), '@scopenet/casino': fileURLToPath(new URL('../../shared/casino', import.meta.url)), '@scopenet/commands': fileURLToPath(new URL('../../shared/commands', import.meta.url)), '@scopenet/board': fileURLToPath(new URL('../../shared/board', import.meta.url)) } },
  server: {
    fs: { allow: ['../..'] },
    port: 5173,
    proxy: {
      '/api': target,
      '/files': target,
      '/uploads': target,
    },
  },
  build: {
    target: 'es2022',
    cssCodeSplit: false,
    chunkSizeWarningLimit: 800,
  },
});

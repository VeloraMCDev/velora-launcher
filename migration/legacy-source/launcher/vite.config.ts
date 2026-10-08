import { defineConfig } from 'vite';
import { svelte } from '@sveltejs/vite-plugin-svelte';
import { fileURLToPath } from 'node:url';

// https://tauri.app/start/frontend/vite/
export default defineConfig({
  plugins: [svelte()],
  // The map viewer is shared with the admin panel (repo root: shared/map).
  resolve: { dedupe: ['svelte', '@lucide/svelte'], alias: { '@scopenet/experience': fileURLToPath(new URL('../shared/experience', import.meta.url)), '@scopenet/map': fileURLToPath(new URL('../shared/map', import.meta.url)), '@scopenet/casino': fileURLToPath(new URL('../shared/casino', import.meta.url)), '@scopenet/commands': fileURLToPath(new URL('../shared/commands', import.meta.url)), '@scopenet/board': fileURLToPath(new URL('../shared/board', import.meta.url)) } },
  clearScreen: false,
  server: { port: 1420, strictPort: true, watch: { ignored: ['**/src-tauri/**'] }, fs: { allow: ['..'] } },
  envPrefix: ['VITE_', 'TAURI_ENV_'],
  build: {
    target: 'es2022',
    cssCodeSplit: false,
    sourcemap: false,
  },
});

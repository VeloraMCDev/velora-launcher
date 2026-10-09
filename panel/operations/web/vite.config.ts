import { defineConfig } from 'vite';
import { svelte } from '@sveltejs/vite-plugin-svelte';
import { fileURLToPath } from 'node:url';

// In development, API calls go to a locally running dashboard server (npm start).
const target = process.env.OPS_URL ?? 'http://localhost:8080';

export default defineConfig({
  root: fileURLToPath(new URL('.', import.meta.url)),
  plugins: [svelte()],
  server: { port: 5174, proxy: { '/api': target } },
  build: { outDir: 'dist', emptyOutDir: true, target: 'es2022', cssCodeSplit: false },
});

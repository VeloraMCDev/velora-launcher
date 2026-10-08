import '@fontsource-variable/outfit';
import '@fontsource-variable/inter';
import '@fontsource-variable/space-grotesk';
import '@fontsource-variable/plus-jakarta-sans';
import '@fontsource-variable/jetbrains-mono';
import './app.css';
import { mount } from 'svelte';
import App from './App.svelte';

// The launcher is a desktop app: no browser context menu or text-drag.
document.addEventListener('contextmenu', (e) => {
  if (!(e.target as HTMLElement).closest('input, textarea, .selectable')) e.preventDefault();
});

mount(App, { target: document.getElementById('app')! });

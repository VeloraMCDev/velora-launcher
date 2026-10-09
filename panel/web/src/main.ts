import '@fontsource-variable/outfit';
import '@fontsource-variable/jetbrains-mono';
// Launcher fonts, so the design preview matches exactly.
import '@fontsource-variable/inter';
import '@fontsource-variable/space-grotesk';
import '@fontsource-variable/plus-jakarta-sans';
import './app.css';
import './lib/nozoom';
import { mount } from 'svelte';
import App from './App.svelte';
import { loadMc } from './lib/mc.svelte';

loadMc();

mount(App, { target: document.getElementById('app')! });

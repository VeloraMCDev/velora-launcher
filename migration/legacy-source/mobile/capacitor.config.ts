import type { CapacitorConfig } from '@capacitor/cli';

// The app is the player panel from your own panel server, wrapped in a native shell.
//   PANEL_URL   baked in at build time (the GitHub variable of the same name); defaults to this project's panel.
//   APP_NAME    the name under the icon (default "SCOPENET")
const DEFAULT_PANEL = 'https://scopenetmcpanel.scopedd.lol';
const url = ((process.env.PANEL_URL ?? '').trim() || DEFAULT_PANEL).replace(/\/+$/, '');

const config: CapacitorConfig = {
  appId: 'net.scopenet.player',
  appName: (process.env.APP_NAME ?? '').trim() || 'SCOPENET',
  webDir: 'www',
  backgroundColor: '#0d0e12',
  server: { url: `${url}/?app=1#/play`, cleartext: url.startsWith('http://') },
  ios: { contentInset: 'never' },
  android: { allowMixedContent: false },
};

export default config;

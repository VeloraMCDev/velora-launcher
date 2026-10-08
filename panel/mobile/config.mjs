export function createMobileConfig(env = process.env) {
  const raw = (env.VELORA_PANEL_URL ?? env.PANEL_URL ?? '').trim().replace(/\/+$/, '');
  if (!raw) throw new Error('Set VELORA_PANEL_URL to the hosted player panel URL.');
  const endpoint = new URL(raw);
  const local = ['localhost', '127.0.0.1', '[::1]'].includes(endpoint.hostname);
  if (endpoint.protocol !== 'https:' && !(endpoint.protocol === 'http:' && local && env.VELORA_ALLOW_LOCAL_HTTP === '1')) {
    throw new Error('Use HTTPS; local HTTP requires VELORA_ALLOW_LOCAL_HTTP=1.');
  }
  if (endpoint.username || endpoint.password || endpoint.search || endpoint.hash) {
    throw new Error('Panel URL must not include credentials, query, or fragment.');
  }
  return {
    appId: 'net.scopenet.player',
    appName: (env.APP_NAME ?? '').trim() || 'Velora',
    webDir: 'www',
    backgroundColor: '#0d0e12',
    server: {url: `${raw}/?app=1#/play`, cleartext: endpoint.protocol === 'http:'},
    ios: {contentInset: 'never'},
    android: {allowMixedContent: false},
  };
}

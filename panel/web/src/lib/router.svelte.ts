import { PLATFORM_PAGES } from '@velora/experience';
// Tiny hash router: #/instances/abc → { name: 'instances', params: ['abc'] }
function parse() {
  const parts = location.hash.replace(/^#\/?/, '').split('?')[0].split('/').filter(Boolean).map(decodeURIComponent);
  if (parts[0] === 'play' && parts[1] === 'instance' && parts[2]) return { instanceId: parts[2], name: 'play', params: parts.slice(3) };
  if (parts[0] === 'instance' && parts[1]) return { instanceId: parts[1], name: parts[2] ?? 'control', params: parts.slice(3) };
  return { instanceId: null as string | null, name: parts[0] ?? 'instances', params: parts.slice(1) };
}

export const route = $state(parse());

window.addEventListener('hashchange', () => {
  const r = parse();
  route.instanceId = r.instanceId;
  route.name = r.name;
  route.params = r.params;
  window.scrollTo({ top: 0 });
});

export function go(path: string) {
  const clean = path.replace(/^\//, '');
  const page = clean.split('/')[0];
  if (route.name === 'play' && route.instanceId && page === 'play' && !clean.startsWith('play/instance/')) { location.hash = `#/play/instance/${encodeURIComponent(route.instanceId)}/${clean.split('/').slice(1).join('/') || 'home'}`; return; }
  const prefix = route.instanceId && !PLATFORM_PAGES.has(page) && !['play', 'landing', 'login', 'reset-password', 'instance'].includes(page) ? `instance/${encodeURIComponent(route.instanceId)}/` : '';
  location.hash = '#/' + prefix + clean;
}

export function playPath(page = 'home') { return route.instanceId ? `#/play/instance/${encodeURIComponent(route.instanceId)}/${page}` : `#/play/${page}`; }

/** The hash for an admin page, keeping the current instance, so plain links behave like go(). */
export function hashFor(path: string) {
  const clean = path.replace(/^\//, '');
  const page = clean.split('/')[0];
  const prefix = route.instanceId && !PLATFORM_PAGES.has(page) && !['play', 'landing', 'login', 'reset-password', 'instance'].includes(page) ? `instance/${encodeURIComponent(route.instanceId)}/` : '';
  return '#/' + prefix + clean;
}

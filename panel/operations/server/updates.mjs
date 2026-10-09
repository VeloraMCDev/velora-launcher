// Release and image update discovery: GitHub releases, GHCR release tags and registry digests.
import { splitRef } from './docker.mjs';

export const REPOSITORY = process.env.VELORA_REPOSITORY ?? 'VeloraMCDev/velora-launcher';
const MANIFEST_TYPES = [
  'application/vnd.oci.image.index.v1+json',
  'application/vnd.docker.distribution.manifest.list.v2+json',
  'application/vnd.oci.image.manifest.v1+json',
  'application/vnd.docker.distribution.manifest.v2+json',
].join(', ');

/** Splits an image reference into registry host, repository path and tag. */
export function parseImage(ref) {
  const [name, tag] = splitRef(ref);
  const parts = name.split('/');
  let registry = 'registry-1.docker.io';
  if (parts.length > 1 && (parts[0].includes('.') || parts[0].includes(':') || parts[0] === 'localhost')) registry = parts.shift();
  if (registry === 'docker.io') registry = 'registry-1.docker.io';
  let repository = parts.join('/');
  if (registry === 'registry-1.docker.io' && !repository.includes('/')) repository = `library/${repository}`;
  return { registry, repository, tag: tag || 'latest', pinned: tag?.startsWith('sha256:') ?? false };
}

async function registryToken(registry, repository) {
  const probe = await fetch(`https://${registry}/v2/`, { signal: AbortSignal.timeout(10_000) });
  const header = probe.headers.get('www-authenticate') ?? '';
  if (probe.ok || !/^Bearer /i.test(header)) return null;
  const params = Object.fromEntries([...header.matchAll(/(\w+)="([^"]*)"/g)].map(m => [m[1], m[2]]));
  if (!params.realm?.startsWith('https://')) return null;
  const url = new URL(params.realm);
  if (params.service) url.searchParams.set('service', params.service);
  url.searchParams.set('scope', `repository:${repository}:pull`);
  const response = await fetch(url, { signal: AbortSignal.timeout(10_000) });
  if (!response.ok) throw new Error(`${registry} token request failed (${response.status})`);
  const body = await response.json();
  return body.token ?? body.access_token ?? null;
}

async function registryGet(registry, repository, path, { method = 'GET', accept } = {}) {
  const token = await registryToken(registry, repository);
  const response = await fetch(`https://${registry}/v2/${repository}/${path}`, {
    method,
    headers: { ...(token ? { authorization: `Bearer ${token}` } : {}), ...(accept ? { accept } : {}) },
    signal: AbortSignal.timeout(15_000),
  });
  if (!response.ok) throw new Error(`${registry}/${repository} ${path}: ${response.status}`);
  return response;
}

export async function remoteDigest(ref) {
  const { registry, repository, tag } = parseImage(ref);
  const response = await registryGet(registry, repository, `manifests/${tag}`, { method: 'HEAD', accept: MANIFEST_TYPES });
  return response.headers.get('docker-content-digest');
}

/** Compares a running container's image with the registry's current digest for the same tag. */
export async function imageUpdate(container, imageInfo) {
  const ref = container.Image;
  const parsed = parseImage(ref);
  if (parsed.pinned || /^sha256:/.test(ref)) return { image: ref, status: 'pinned' };
  const local = (imageInfo?.RepoDigests ?? []).map(d => d.split('@')[1]);
  if (!local.length) return { image: ref, status: 'local-build' };
  try {
    const latest = await remoteDigest(ref);
    return { image: ref, status: latest && !local.includes(latest) ? 'update-available' : 'current', latest, local: local[0] };
  } catch (e) {
    return { image: ref, status: 'unknown', error: e.message };
  }
}

/** Velora release images published by the release workflows: tags are sha-<commit>-<run>-<attempt>. */
export async function releaseImages(name) {
  const repository = `${REPOSITORY.split('/')[0].toLowerCase()}/${name}`;
  const response = await registryGet('ghcr.io', repository, 'tags/list?n=1000');
  const tags = (await response.json()).tags ?? [];
  const releases = tags
    .map(tag => tag.match(/^sha-([0-9a-f]{40})-(\d+)-(\d+)$/))
    .filter(Boolean)
    .map(([tag, commit, run, attempt]) => ({ tag, commit, run: Number(run), attempt: Number(attempt) }))
    .sort((a, b) => b.run - a.run || b.attempt - a.attempt)
    .slice(0, 10);
  for (const release of releases.slice(0, 5)) {
    const head = await registryGet('ghcr.io', repository, `manifests/${release.tag}`, { method: 'HEAD', accept: MANIFEST_TYPES });
    release.digest = head.headers.get('docker-content-digest');
    release.image = `ghcr.io/${repository}@${release.digest}`;
  }
  return releases.filter(r => r.digest);
}

export async function github(path, token) {
  const response = await fetch(`https://api.github.com/${path}`, {
    headers: { accept: 'application/vnd.github+json', 'user-agent': 'velora-operations', ...(token ? { authorization: `Bearer ${token}` } : {}) },
    signal: AbortSignal.timeout(15_000),
  });
  if (!response.ok) throw new Error(`GitHub ${path.split('?')[0]}: ${response.status}`);
  return response.json();
}

export async function launcherReleases(token) {
  const releases = await github(`repos/${REPOSITORY}/releases?per_page=20`, token);
  return releases
    .filter(r => !r.draft && r.tag_name.startsWith('launcher-v'))
    .map(r => ({
      tag: r.tag_name,
      version: r.tag_name.slice('launcher-v'.length),
      name: r.name,
      published_at: r.published_at,
      prerelease: r.prerelease,
      html_url: r.html_url,
      assets: r.assets.map(a => ({ name: a.name, size: a.size })),
      has_manifest: r.assets.some(a => a.name === 'release-manifest.json'),
    }));
}

const commits = new Map();
export async function commitInfo(sha, token) {
  if (commits.has(sha)) return commits.get(sha);
  const c = await github(`repos/${REPOSITORY}/commits/${sha}`, token).catch(() => null);
  const info = c ? { sha, message: c.commit.message.split('\n')[0], date: c.commit.committer?.date, url: c.html_url } : { sha };
  commits.set(sha, info);
  return info;
}

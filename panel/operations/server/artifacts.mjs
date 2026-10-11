// Discover downloadable output without automatically publishing or installing it.
import { github, REPOSITORY } from './updates.mjs';

export function trustedBuild(run) {
  return run?.conclusion === 'success' && run.status === 'completed' && run.head_branch === 'main'
    && ['push', 'workflow_dispatch'].includes(run.event)
    && run.repository?.full_name?.toLowerCase() === REPOSITORY.toLowerCase()
    && run.head_repository?.full_name?.toLowerCase() === REPOSITORY.toLowerCase();
}
const apiPath = path => `repos/${REPOSITORY}/${path}`;
export async function artifactCatalog(token, page = 1, get = github) {
  if (!Number.isSafeInteger(page) || page < 1 || page > 10000) throw Object.assign(new Error('Invalid catalog page'), { status: 400 });
  const items = [], errors = [];
  let moreReleases = false, moreBuilds = false;
  const [releases, artifacts] = await Promise.allSettled([
    get(apiPath(`releases?per_page=100&page=${page}`), token),
    get(apiPath(`actions/artifacts?per_page=100&page=${page}`), token),
  ]);
  if (releases.status === 'fulfilled') {
    moreReleases = releases.value.length === 100;
    for (const r of releases.value.filter(r => !r.draft)) for (const a of r.assets ?? []) {
      if (a.state !== 'uploaded') continue;
      items.push({ id: `release-${a.id}`, name: a.name, source: 'release', version: r.tag_name, product: r.name || r.tag_name,
        bytes: a.size, created_at: r.published_at, prerelease: r.prerelease, digest: a.digest ?? null,
        expired: false, expires_at: null, download: a.browser_download_url, url: r.html_url, commit: null });
    }
  } else errors.push(`Releases: ${releases.reason.message}`);
  if (artifacts.status === 'fulfilled') {
    moreBuilds = page * 100 < artifacts.value.total_count;
    const candidates = artifacts.value.artifacts.filter(a => a.workflow_run?.head_branch === 'main'
      && a.workflow_run.repository_id === a.workflow_run.head_repository_id);
    const runs = new Map();
    const ids = [...new Set(candidates.map(a => a.workflow_run.id))];
    // Bound concurrent provider requests; an incomplete source is reported explicitly.
    for (let offset = 0; offset < ids.length; offset += 8) await Promise.all(ids.slice(offset, offset + 8).map(async id => {
      try { runs.set(id, await get(apiPath(`actions/runs/${id}`), token)); }
      catch (e) { errors.push(`Build ${id}: ${e.message}`); }
    }));
    for (const a of candidates) {
      const r = runs.get(a.workflow_run.id);
      if (!trustedBuild(r) || a.workflow_run.head_sha !== r.head_sha) continue;
      items.push({ id: `artifact-${a.id}`, name: a.name, source: 'build', version: `run ${r.id}`, product: r.name,
        bytes: a.size_in_bytes, created_at: a.created_at, prerelease: false, digest: a.digest ?? null,
        expired: a.expired || Date.parse(a.expires_at) <= Date.now(), expires_at: a.expires_at,
        download: `/api/artifacts/${a.id}/download`, url: r.html_url, commit: r.head_sha });
    }
  } else errors.push(`Workflow artifacts: ${artifacts.reason.message}`);
  return { items: items.sort((a,b) => Date.parse(b.created_at) - Date.parse(a.created_at)), errors, page,
    has_more: moreReleases || moreBuilds, checked: new Date().toISOString(), token_set: Boolean(token) };
}

// Share bounded results across dashboard viewers rather than exhausting API limits.
export function createCatalogCache(get = artifactCatalog) {
  let credential, pages = new Map();
  return async (token, page = 1, now = Date.now()) => {
    if (credential !== token) { credential = token; pages.clear(); }
    const cached = pages.get(page);
    if (cached && now - cached.at < 300000) return cached.result;
    if (pages.size >= 20) pages.delete(pages.keys().next().value);
    const result = get(token, page);
    pages.set(page, { at: now, result });
    try { return await result; } catch (e) { if (pages.get(page)?.result === result) pages.delete(page); throw e; }
  };
}

export async function artifactDownload(id, token, get = github, fetcher = fetch) {
  if (!token) throw Object.assign(new Error('Configure a GitHub token with Actions read permission in Settings to download workflow archives'), { status: 503 });
  if (!/^\d+$/.test(String(id))) throw Object.assign(new Error('Invalid artifact'), { status: 400 });
  const a = await get(apiPath(`actions/artifacts/${id}`), token);
  if (a.expired || Date.parse(a.expires_at) <= Date.now()) throw Object.assign(new Error('This workflow artifact has expired. Use a published release or rebuild it.'), { status: 410 });
  const r = await get(apiPath(`actions/runs/${a.workflow_run?.id}`), token);
  if (!trustedBuild(r) || a.workflow_run?.head_sha !== r.head_sha) throw Object.assign(new Error('Only successful main builds from this repository can be downloaded'), { status: 403 });
  // Never forward the GitHub credential to the signed storage redirect.
  const response = await fetcher(`https://api.github.com/${apiPath(`actions/artifacts/${id}/zip`)}`, {
    redirect: 'manual', headers: { authorization: `Bearer ${token}`, accept: 'application/vnd.github+json', 'user-agent': 'velora-operations' }, signal: AbortSignal.timeout(15000),
  });
  const location = response.headers.get('location');
  const url = location && new URL(location);
  if (response.status !== 302 || url?.protocol !== 'https:' || url.username || url.password) throw Object.assign(new Error('GitHub could not prepare the artifact download'), { status: 502 });
  return location;
}

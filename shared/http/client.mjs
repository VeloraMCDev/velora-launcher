/** Generic API v1 transport. The application supplies live session/scope/lifecycle ports. */
export class ApiError extends Error {
  constructor(message, status) { super(message); this.status = status; }
}

export class ApiVersionError extends Error {
  constructor(actual) { super('Unsupported Velora API version'); this.name = 'ApiVersionError'; this.actual = actual; }
}
export function assertApiVersion(manifest) {
  if (manifest?.api_version !== 1) throw new ApiVersionError(manifest?.api_version);
  return manifest;
}

export function createHttpClient(configuration = {}) {
  const getToken = configuration.getToken ?? (() => undefined);
  const getInstance = configuration.getInstance ?? (() => undefined);
  const fetcher = configuration.fetch ?? ((...args) => globalThis.fetch(...args));
  const timeout = configuration.timeoutMs ?? 30_000;
  if (!Number.isFinite(timeout) || timeout < 0 || timeout > 2_147_483_647) throw new TypeError('Invalid HTTP timeout');
  const scopeHeader = configuration.instanceHeader ?? 'X-SCOPENET-Instance';
  if (!['X-SCOPENET-Instance', 'X-Velora-Instance'].includes(scopeHeader)) throw new TypeError('Unsupported instance header');
  let base = '';
  if (configuration.baseUrl) {
    let parsed;
    try { parsed = new URL(configuration.baseUrl); } catch { throw new TypeError('Invalid HTTP base URL'); }
    if (!['http:', 'https:'].includes(parsed.protocol) || parsed.username || parsed.password || parsed.search || parsed.hash)
      throw new TypeError('HTTP base URL must have a host and no credentials, query or fragment');
    base = parsed.toString().replace(/\/+$/, '');
  }
  async function api(path, options = {}) {
    // API paths cannot redirect a scoped bearer request to a request-selected origin.
    if (!path.startsWith('/') || path.startsWith('//') || path.includes('\\') || /[\x00-\x1f\x7f]/.test(path))
      throw new TypeError('API path must be an origin-relative path');
    const headers = {};
    const instance = getInstance();
    if (instance) headers[scopeHeader] = instance;
    const token = getToken();
    if (token) headers.Authorization = `Bearer ${token}`;
    let body;
    if (options.form) body = options.form;
    else if (options.body !== undefined) { headers['Content-Type'] = 'application/json'; body = JSON.stringify(options.body); }
    const controller = timeout || options.signal ? new AbortController() : undefined;
    const abort = () => controller.abort(options.signal.reason);
    if (options.signal?.aborted) abort();
    else options.signal?.addEventListener('abort', abort, {once: true});
    const timer = timeout ? setTimeout(() => controller.abort(new DOMException('HTTP request timed out', 'TimeoutError')), timeout) : undefined;
    try {
      const response = await fetcher(base + path, {
        method: options.method ?? (body ? 'POST' : 'GET'), headers, body,
        ...(controller ? {signal: controller.signal} : {}),
      });
      const text = await response.text();
      let data = null;
      try { data = text ? JSON.parse(text) : null; } catch { data = text; }
      if (!response.ok) {
        // Retain the legacy *live* session check after the request completes.
        if (response.status === 401 && getToken()) configuration.onUnauthorized?.();
        throw new ApiError(data?.error ?? `Request failed (${response.status})`, response.status);
      }
      return data;
    } finally {
      if (timer !== undefined) clearTimeout(timer);
      options.signal?.removeEventListener('abort', abort);
    }
  }
  const get = path => api(path);
  return {
    api, get,
    post: (path, body) => api(path, {method: 'POST', body: body ?? {}}),
    put: (path, body) => api(path, {method: 'PUT', body}),
    patch: (path, body) => api(path, {method: 'PATCH', body}),
    del: path => api(path, {method: 'DELETE'}),
    manifest: async () => assertApiVersion(await get('/api/v1/launcher/manifest')),
  };
}

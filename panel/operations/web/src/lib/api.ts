let csrf = '';
export const setCsrf = (token: string) => (csrf = token);

export async function downloadInstaller(body: unknown): Promise<Blob> {
  const response = await fetch('/api/agents/installer', {
    method: 'POST', credentials: 'same-origin',
    headers: { 'content-type': 'application/json', 'x-csrf-token': csrf },
    body: JSON.stringify(body),
  });
  if (!response.ok) {
    if (response.status === 401) window.dispatchEvent(new CustomEvent('ops:signed-out'));
    const error = await response.json().catch(() => ({}));
    throw new ApiError(error.error ?? `Download failed (${response.status})`, response.status);
  }
  return response.blob();
}

export class ApiError extends Error {
  constructor(message: string, public status: number) {
    super(message);
  }
}

export async function api<T = unknown>(path: string, init: { method?: string; body?: unknown } = {}): Promise<T> {
  const method = init.method ?? 'GET';
  const response = await fetch(path, {
    method,
    credentials: 'same-origin',
    headers: {
      ...(init.body !== undefined ? { 'content-type': 'application/json' } : {}),
      ...(method !== 'GET' ? { 'x-csrf-token': csrf } : {}),
    },
    body: init.body !== undefined ? JSON.stringify(init.body) : undefined,
  });
  const data = await response.json().catch(() => ({}));
  if (!response.ok) {
    if (response.status === 401 && path !== '/api/login') window.dispatchEvent(new CustomEvent('ops:signed-out'));
    throw new ApiError((data as { error?: string }).error ?? `Request failed (${response.status})`, response.status);
  }
  return data as T;
}

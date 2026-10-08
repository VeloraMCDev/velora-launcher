// Frozen generic transport fixture; see PROVENANCE.json. No application/private implementation.
export function createLegacy(route, session, logout, fetch) {
    class ApiError extends Error {
        status;
        constructor(message, status) {
            super(message);
            this.status = status;
        }
    }
    async function api(path, opts = {}) {
        const headers = {};
        if (route.instanceId)
            headers['X-SCOPENET-Instance'] = route.instanceId;
        if (session.token)
            headers.Authorization = `Bearer ${session.token}`;
        let body;
        if (opts.form)
            body = opts.form;
        else if (opts.body !== undefined) {
            headers['Content-Type'] = 'application/json';
            body = JSON.stringify(opts.body);
        }
        const res = await fetch(path, { method: opts.method ?? (body ? 'POST' : 'GET'), headers, body });
        const text = await res.text();
        let data = null;
        try {
            data = text ? JSON.parse(text) : null;
        }
        catch {
            data = text;
        }
        if (!res.ok) {
            if (res.status === 401 && session.token)
                logout();
            throw new ApiError(data?.error ?? `Request failed (${res.status})`, res.status);
        }
        return data;
    }
    const get = (p) => api(p);
    const post = (p, body) => api(p, { method: 'POST', body: body ?? {} });
    const put = (p, body) => api(p, { method: 'PUT', body });
    const patch = (p, body) => api(p, { method: 'PATCH', body });
    const del = (p) => api(p, { method: 'DELETE' });
    return { api, get, post, put, patch, del, ApiError };
}

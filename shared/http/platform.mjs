import {assertApiVersion, createHttpClient} from './client.mjs';

// URL segments are values, never routes. Dot segments normalize even when encoded.
function segment(value) {
  if (typeof value !== 'string' || !value || value === '.' || value === '..') {
    throw new TypeError('Expected a non-empty instance ID other than a dot segment');
  }
  return encodeURIComponent(value);
}

/** Platform API v1 endpoints; all lifecycle, identity and scope remain caller-owned. */
export function createPlatformClient(configuration = {}) {
  const transport = createHttpClient(configuration);
  const request = (path, method, body, options) => transport.api(path, {method, body, signal: options?.signal});
  return {
    transport,
    launcher: {
      manifest: (options) => request('/api/v1/launcher/manifest', 'GET', undefined, options)
        .then(assertApiVersion),
      instanceManifest: (id, options) => request(`/api/v1/launcher/instances/${segment(id)}`, 'GET', undefined, options),
      event: (body, options) => request('/api/v1/launcher/events', 'POST', body, options),
      latestUpdate: (options) => request('/api/v1/launcher/update', 'GET', undefined, options),
    },
    auth: {
      login: (body, options) => request('/api/v1/auth/login', 'POST', body, options),
      register: (body, options) => request('/api/v1/auth/register', 'POST', body, options),
      me: (options) => request('/api/v1/auth/me', 'GET', undefined, options),
      forgotPassword: (email, options) => request('/api/v1/auth/forgot-password', 'POST', {email}, options),
      resetPassword: (token, password, options) => request('/api/v1/auth/reset-password', 'POST', {token, password}, options),
    },
    account: {
      profile: (options) => request('/api/v1/account/profile', 'GET', undefined, options),
      setUsername: (username, password, options) => request('/api/v1/account/username', 'PUT', {username, password}, options),
      uploadSkin: (file, model = 'classic', options) => {
        const form = new FormData();
        form.append('file', file);
        form.append('model', model);
        return transport.api('/api/v1/account/skin', {method: 'POST', form, signal: options?.signal});
      },
      deleteSkin: (options) => request('/api/v1/account/skin', 'DELETE', undefined, options),
      setSkinModel: (model, options) => request('/api/v1/account/skin/model', 'PUT', {model}, options),
      setCape: (cape_id, options) => request('/api/v1/account/cape', 'PUT', {cape_id}, options),
    },
  };
}

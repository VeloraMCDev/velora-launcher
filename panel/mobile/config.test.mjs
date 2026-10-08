import {test} from 'node:test';
import assert from 'node:assert/strict';
import {createMobileConfig} from './config.mjs';
test('retains installed identity, player routing and host branding', () => {
  const config = createMobileConfig({VELORA_PANEL_URL: ' https://panel.example.com/community/// ', APP_NAME: 'Example community'});
  assert.equal(config.appId, 'net.scopenet.player');
  assert.equal(config.appName, 'Example community');
  assert.equal(config.server.url, 'https://panel.example.com/community/?app=1#/play');
  assert.equal(config.server.cleartext, false);
  assert.equal(config.android.allowMixedContent, false);
});
test('new setting overrides the compatible legacy setting', () => {
  assert.equal(createMobileConfig({VELORA_PANEL_URL: 'https://new.example.com', PANEL_URL: 'https://old.example.com'}).server.url, 'https://new.example.com/?app=1#/play');
  assert.equal(createMobileConfig({PANEL_URL: 'https://old.example.com'}).appName, 'Velora');
});
test('no silent operator origin or insecure transport fallback', () => {
  for (const url of ['', 'http://panel.example.com', 'javascript:alert(1)', 'https://user:pass@example.com', 'https://example.com/?token=secret', 'https://example.com/#play']) {
    assert.throws(() => createMobileConfig({VELORA_PANEL_URL: url}));
  }
});
test('only explicit loopback development can use HTTP', () => {
  assert.throws(() => createMobileConfig({VELORA_PANEL_URL: 'http://localhost:8080'}));
  assert.equal(createMobileConfig({VELORA_PANEL_URL: 'http://localhost:8080', VELORA_ALLOW_LOCAL_HTTP: '1'}).server.cleartext, true);
  assert.throws(() => createMobileConfig({VELORA_PANEL_URL: 'http://panel.example.com', VELORA_ALLOW_LOCAL_HTTP: '1'}));
});

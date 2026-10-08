import {ApiError, createHttpClient, assertApiVersion} from '../client.mjs';
const client = createHttpClient({getToken: () => 'synthetic', getInstance: () => undefined, timeoutMs: 0});
const record: Promise<{id: number}> = client.get('/api/example');
const request: Promise<unknown> = client.api('/api/example', {body: null, signal: new AbortController().signal});
const version = assertApiVersion({api_version: 1, field: 'synthetic'});
const error: number = new ApiError('example', 403).status;
void [record, request, version.field, error];
// @ts-expect-error caller-defined remote scope headers are not a supported contract
createHttpClient({instanceHeader: 'X-Private-Gameplay'});

import {createPlatformClient, type AuthResponse, type PlayerProfile, type InstanceManifest} from 'velora-platform-http/platform';
const platform = createPlatformClient();
const account: Promise<AuthResponse> = platform.auth.login({username:'synthetic',password:'example'});
const skin: Promise<PlayerProfile> = platform.account.setCape(null);
const instance: Promise<InstanceManifest> = platform.launcher.instanceManifest('synthetic');
void [account, skin, instance];
// @ts-expect-error username mutations require password reauthentication
platform.account.setUsername('synthetic');
// @ts-expect-error skin model uses existing wire values
platform.account.setSkinModel('alex');
// @ts-expect-error manifest instances are structured, not unknown blobs
platform.launcher.manifest().then(value => value.instances[0].server?.autojoin);
platform.launcher.manifest().then(value => value.instances[0].server?.auto_join);

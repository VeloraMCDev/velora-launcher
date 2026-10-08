import { createServer } from 'node:http';
import { readFileSync } from 'node:fs';
import { pathToFileURL } from 'node:url';

export function probeServer(identity) {
  if (!/^[a-f0-9]{40}$/.test(identity.commit) || !/^\d+\.\d+\.\d+$/.test(identity.version)
      || !/^[a-zA-Z0-9_.-]{1,128}$/.test(identity.build)) throw Error('Invalid immutable probe build identity');
  const health = JSON.stringify({ status: 'ok', service: 'deployment-probe', version: identity.version, commit: identity.commit, build: identity.build });
  return createServer((request, response) => {
    response.setHeader('Content-Type', 'application/json');
    response.setHeader('Cache-Control', 'no-store');
    response.setHeader('X-Content-Type-Options', 'nosniff');
    if (!['/health', '/health/live', '/health/ready'].includes(request.url)) {
      response.writeHead(404).end('{"error":"NOT_FOUND"}');
    } else if (request.method !== 'GET') {
      response.setHeader('Allow', 'GET');
      response.writeHead(405).end('{"error":"METHOD_NOT_ALLOWED"}');
    } else response.writeHead(200).end(health);
  });
}

if (process.argv[1] && import.meta.url === pathToFileURL(process.argv[1]).href) {
  const server = probeServer(JSON.parse(readFileSync(new URL('./build-info.json', import.meta.url), 'utf8')));
  server.listen(8080, '0.0.0.0');
  for (const signal of ['SIGTERM', 'SIGINT']) process.on(signal, () => {
    server.close(() => process.exit(0));
    setTimeout(() => process.exit(1), 10_000).unref();
  });
}

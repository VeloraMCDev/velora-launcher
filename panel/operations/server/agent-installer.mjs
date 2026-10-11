import { readFileSync } from 'node:fs';
import { config } from '../agent/monitor.mjs';

// Store-only ZIP: bounded source/config files, no archive dependency or server-side secrets.
export function zipFiles(files) {
  const chunks = [], central = []; let offset = 0;
  for (const [name, value] of Object.entries(files)) {
    const filename = Buffer.from(name), data = Buffer.from(value);
    let crc = 0xffffffff;
    for (const byte of data) { crc ^= byte; for (let bit=0;bit<8;bit++) crc = (crc >>> 1) ^ ((crc & 1) ? 0xedb88320 : 0); }
    crc = (crc ^ 0xffffffff) >>> 0;
    const header = Buffer.alloc(30); header.writeUInt32LE(0x04034b50);
    header.writeUInt16LE(20,4); header.writeUInt16LE(0x21,12); // 1980-01-01
    header.writeUInt32LE(crc,14);header.writeUInt32LE(data.length,18);header.writeUInt32LE(data.length,22);header.writeUInt16LE(filename.length,26);
    const directory = Buffer.alloc(46);directory.writeUInt32LE(0x02014b50);directory.writeUInt16LE(0x0314,4);directory.writeUInt16LE(20,6);directory.writeUInt16LE(0x21,14);
    directory.writeUInt32LE(crc,16);directory.writeUInt32LE(data.length,20);directory.writeUInt32LE(data.length,24);directory.writeUInt16LE(filename.length,28);
    directory.writeUInt32LE(0x81800000,38); // Regular file, mode 0600 (including credential).
    directory.writeUInt32LE(offset,42);central.push(directory,filename);
    chunks.push(header,filename,data);offset += header.length+filename.length+data.length;
  }
  const directory = Buffer.concat(central), end = Buffer.alloc(22);end.writeUInt32LE(0x06054b50);end.writeUInt16LE(Object.keys(files).length,8);end.writeUInt16LE(Object.keys(files).length,10);end.writeUInt32LE(directory.length,12);end.writeUInt32LE(offset,16);
  return Buffer.concat([...chunks,directory,end]);
}

export function installerFiles(value, mode) {
  if (!['linux','desktop'].includes(mode)) throw Object.assign(new Error('Choose Linux Docker Engine or Docker Desktop'),{status:400});
  let cfg;
  try { cfg = config(value); } catch { throw Object.assign(new Error('Check the Ops HTTPS URL and monitoring targets'),{status:400}); }
  // Explicit fields only; this download never persists a plaintext credential.
  const configuration = {ops_url:cfg.ops_url,id:cfg.id,token:cfg.token,interval_seconds:cfg.interval_seconds,
    ...(cfg.ops_headers ? {ops_headers:cfg.ops_headers} : {}),services:cfg.services,disks:cfg.disks};
  const compose = `services:
  monitor:
    build: .
    restart: unless-stopped
    init: true
    user: "1000:1000"
    read_only: true
    cap_drop: [ALL]
    security_opt: [no-new-privileges:true]
    pids_limit: 64
    logging:
      driver: json-file
      options:
        max-size: "5m"
        max-file: "2"
${mode === 'linux' ? '    network_mode: host\n' : ''}    volumes:
      - type: bind
        source: ./velora-monitor-config.json
        target: /config/velora-monitor-config.json
        read_only: true
        bind:
          create_host_path: false
`;
  const source = name => readFileSync(new URL(`../agent/${name}`,import.meta.url));
  return {'compose.yaml':compose,'Dockerfile':source('Dockerfile'),'.dockerignore':source('.dockerignore'),
    'monitor.mjs':source('monitor.mjs'),'velora-monitor-config.json':JSON.stringify(configuration,null,2)+'\n',
    'README.md':source('README.md')};
}

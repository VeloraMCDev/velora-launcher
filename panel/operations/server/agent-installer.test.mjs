import test from 'node:test';
import assert from 'node:assert/strict';
import { installerFiles, zipFiles } from './agent-installer.mjs';
const cfg={ops_url:'https://ops.example.org',id:'a'.repeat(36),token:'synthetic'.repeat(5),services:[{name:'SMP',kind:'minecraft',host:'127.0.0.1',port:25565}]};
test('installer uses selected networking and isolates credentials from Docker build context',()=>{
  const linux=installerFiles(cfg,'linux'),desktop=installerFiles(cfg,'desktop');
  assert.match(linux['compose.yaml'],/network_mode: host/);assert.ok(!desktop['compose.yaml'].includes('network_mode: host'));assert.ok(!desktop['compose.yaml'].includes('extra_hosts')); // Preserve Desktop's built-in host.docker.internal DNS.
  assert.match(linux['compose.yaml'],/create_host_path: false/);assert.match(linux['compose.yaml'],/user: "1000:1000"/);
  assert.match(linux['.dockerignore'].toString(),/^\*\n!Dockerfile\n!monitor.mjs/m);
  assert.ok(!linux.Dockerfile.toString().includes('COPY .'));assert.equal(JSON.parse(linux['velora-monitor-config.json']).token,cfg.token);
  assert.throws(()=>installerFiles(cfg,'invalid'),{status:400});assert.throws(()=>installerFiles({...cfg,ops_url:'http://ops.example.org'},'linux'),{status:400});
});
test('ZIP preserves exact content, offsets and private file permissions',()=>{
  const files=installerFiles(cfg,'linux'),zip=zipFiles(files);let offset=0;const extracted={};
  while(zip.readUInt32LE(offset)===0x04034b50){const size=zip.readUInt32LE(offset+18),len=zip.readUInt16LE(offset+26),name=zip.subarray(offset+30,offset+30+len).toString();extracted[name]=zip.subarray(offset+30+len,offset+30+len+size);offset+=30+len+size;}
  assert.deepEqual(Object.keys(extracted),Object.keys(files));for(const [name,value]of Object.entries(files))assert.deepEqual(extracted[name],Buffer.from(value));
  assert.equal(zip.readUInt32LE(offset),0x02014b50);assert.equal(zip.readUInt32LE(offset+38),0x81800000);
  const end=zip.length-22;assert.equal(zip.readUInt32LE(end),0x06054b50);assert.equal(zip.readUInt32LE(end+16),offset);
});

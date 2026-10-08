import {readFile,mkdir,copyFile,writeFile} from 'node:fs/promises';
import {createHash} from 'node:crypto';
const sha=process.env.GITHUB_SHA;
if(!/^[a-f0-9]{40}$/.test(sha??''))throw Error('Full source SHA required');
const directory='dist/agent';await mkdir(directory,{recursive:true});
const sources={'velora-agent':'target/x86_64-unknown-linux-musl/release/velora-agent','velora-agent.service':'agent/velora-agent.service','install.sh':'agent/install.sh'};
const files=[];
for(const [path,source] of Object.entries(sources)){
  const bytes=await readFile(source);await copyFile(source,directory+'/'+path);
  files.push({path,bytes:bytes.length,sha256:createHash('sha256').update(bytes).digest('hex')});
}
await writeFile(directory+'/artifact-manifest.json',JSON.stringify({schema:1,repository:'VeloraMCDev/velora-launcher',service:'velora-agent',version:'0.1.0',source_sha:sha,build_run_id:process.env.GITHUB_RUN_ID,platform:'linux-amd64',target:'x86_64-unknown-linux-musl',protocol:1,capabilities:['REPORT_HEARTBEAT'],files},null,2)+'\n');

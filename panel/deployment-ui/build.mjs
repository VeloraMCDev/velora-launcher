import {mkdir,readFile,copyFile,writeFile} from 'node:fs/promises';
import {createHash} from 'node:crypto';
const sha=process.env.GITHUB_SHA;if(!/^[a-f0-9]{40}$/.test(sha??''))throw Error('Full source SHA required');
await mkdir('dist/deployment-ui',{recursive:true});
const files=[];for(const path of ['index.html','styles.css','client.mjs','probe-plan.mjs','app.mjs']){const data=await readFile('deployment-ui/'+path);await copyFile('deployment-ui/'+path,'dist/deployment-ui/'+path);files.push({path,bytes:data.length,sha256:createHash('sha256').update(data).digest('hex')});}
await writeFile('dist/deployment-ui/artifact-manifest.json',JSON.stringify({schema:1,repository:'VeloraMCDev/velora-launcher',service:'deployment-ui',source_sha:sha,build_run_id:process.env.GITHUB_RUN_ID,files},null,2)+'\n');

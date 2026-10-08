import Ajv from 'ajv';
import standaloneCode from 'ajv/dist/standalone/index.js';
import { readFileSync, writeFileSync } from 'node:fs';
const schema=JSON.parse(readFileSync('deployment/contracts/artifact.schema.json','utf8'));
const ajv=new Ajv({strict:true,allErrors:true,code:{source:true,esm:true,lines:true}});
const compiled=standaloneCode(ajv,ajv.compile(schema));
const output='// @ts-nocheck\n// Generated from the canonical artifact schema; no runtime compilation/eval.\n'
  +'import * as runtime from "ajv/dist/runtime/ucs2length.js";\n'
  +'const ucs2length = typeof runtime.default === "function" ? runtime.default : runtime.default.default;\n'
  +compiled.replaceAll('require("ajv/dist/runtime/ucs2length").default','ucs2length')+'\n';
if(output.includes('require('))throw Error('Unexpected standalone runtime dependency');
const target='control-plane/src/artifact-validator.generated.mjs';
if(process.argv.includes('--check')){
  if(readFileSync(target,'utf8').replace(/\r\n/g,'\n')!==output)throw Error('Regenerate the artifact validator');
}else writeFileSync(target,output);

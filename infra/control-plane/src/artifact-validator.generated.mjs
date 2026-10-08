// @ts-nocheck
// Generated from the canonical artifact schema; no runtime compilation/eval.
import * as runtime from "ajv/dist/runtime/ucs2length.js";
const ucs2length = typeof runtime.default === "function" ? runtime.default : runtime.default.default;
"use strict";
export const validate = validate10;
export default validate10;
const schema11 = {"$schema":"http://json-schema.org/draft-07/schema#","$id":"https://velora.invalid/contracts/artifact-v1","title":"Velora immutable artifact registration input v1","type":"object","additionalProperties":false,"required":["schema","repository","service_id","git_sha","git_ref","build_run_id","artifact_type","artifact_uri","sha256","version","build_status","test_status"],"properties":{"schema":{"const":1},"repository":{"type":"string","pattern":"^[A-Za-z0-9][A-Za-z0-9_.-]{0,99}/[A-Za-z0-9][A-Za-z0-9_.-]{0,99}$"},"service_id":{"type":"string","pattern":"^[a-z][a-z0-9-]{0,63}$"},"git_sha":{"type":"string","pattern":"^[a-f0-9]{40}$"},"git_ref":{"type":"string","pattern":"^refs/(heads|tags)/[A-Za-z0-9][A-Za-z0-9_./-]{0,199}$"},"build_run_id":{"type":"string","pattern":"^[1-9][0-9]{0,19}$"},"artifact_type":{"enum":["oci","binary","static_bundle","worker_bundle","jar","config_bundle"]},"artifact_uri":{"type":"string","minLength":1,"maxLength":1024},"sha256":{"type":"string","pattern":"^[a-f0-9]{64}$"},"oci_digest":{"type":"string","pattern":"^sha256:[a-f0-9]{64}$"},"version":{"type":"string","pattern":"^[0-9]+\\.[0-9]+\\.[0-9]+(?:-[0-9A-Za-z.-]+)?(?:\\+[0-9A-Za-z.-]+)?$","maxLength":128},"build_status":{"const":"PASSED"},"test_status":{"const":"PASSED"},"provenance_uri":{"type":"string","maxLength":1024},"sbom_uri":{"type":"string","maxLength":1024},"candidate_id":{"type":"string","pattern":"^rc_[a-f0-9]{64}$"}},"allOf":[{"if":{"properties":{"artifact_type":{"const":"oci"}}},"then":{"required":["oci_digest"],"properties":{"artifact_uri":{"pattern":"^ghcr\\.io/[a-z0-9][a-z0-9._/-]*@sha256:[a-f0-9]{64}$","type":"string"},"oci_digest":{"type":"string","pattern":"^sha256:[a-f0-9]{64}$"}}},"else":{"properties":{"artifact_uri":{"pattern":"^https://","type":"string"}},"not":{"required":["oci_digest"],"properties":{"oci_digest":{"type":"string","pattern":"^sha256:[a-f0-9]{64}$"}}}}}]};
const pattern0 = new RegExp("^ghcr\\.io/[a-z0-9][a-z0-9._/-]*@sha256:[a-f0-9]{64}$", "u");
const pattern1 = new RegExp("^sha256:[a-f0-9]{64}$", "u");
const pattern3 = new RegExp("^https://", "u");
const pattern4 = new RegExp("^[A-Za-z0-9][A-Za-z0-9_.-]{0,99}/[A-Za-z0-9][A-Za-z0-9_.-]{0,99}$", "u");
const pattern5 = new RegExp("^[a-z][a-z0-9-]{0,63}$", "u");
const pattern6 = new RegExp("^[a-f0-9]{40}$", "u");
const pattern7 = new RegExp("^refs/(heads|tags)/[A-Za-z0-9][A-Za-z0-9_./-]{0,199}$", "u");
const pattern8 = new RegExp("^[1-9][0-9]{0,19}$", "u");
const pattern9 = new RegExp("^[a-f0-9]{64}$", "u");
const pattern11 = new RegExp("^[0-9]+\\.[0-9]+\\.[0-9]+(?:-[0-9A-Za-z.-]+)?(?:\\+[0-9A-Za-z.-]+)?$", "u");
const pattern12 = new RegExp("^rc_[a-f0-9]{64}$", "u");
const func2 = Object.prototype.hasOwnProperty;
const func3 = ucs2length;

function validate10(data, {instancePath="", parentData, parentDataProperty, rootData=data}={}){
/*# sourceURL="https://velora.invalid/contracts/artifact-v1" */;
let vErrors = null;
let errors = 0;
const _errs2 = errors;
let valid1 = true;
const _errs3 = errors;
if(data && typeof data == "object" && !Array.isArray(data)){
if(data.artifact_type !== undefined){
if("oci" !== data.artifact_type){
const err0 = {};
if(vErrors === null){
vErrors = [err0];
}
else {
vErrors.push(err0);
}
errors++;
}
}
}
var _valid0 = _errs3 === errors;
errors = _errs2;
if(vErrors !== null){
if(_errs2){
vErrors.length = _errs2;
}
else {
vErrors = null;
}
}
let ifClause0;
if(_valid0){
const _errs5 = errors;
if(data && typeof data == "object" && !Array.isArray(data)){
if(data.oci_digest === undefined){
const err1 = {instancePath,schemaPath:"#/allOf/0/then/required",keyword:"required",params:{missingProperty: "oci_digest"},message:"must have required property '"+"oci_digest"+"'"};
if(vErrors === null){
vErrors = [err1];
}
else {
vErrors.push(err1);
}
errors++;
}
if(data.artifact_uri !== undefined){
let data1 = data.artifact_uri;
if(typeof data1 === "string"){
if(!pattern0.test(data1)){
const err2 = {instancePath:instancePath+"/artifact_uri",schemaPath:"#/allOf/0/then/properties/artifact_uri/pattern",keyword:"pattern",params:{pattern: "^ghcr\\.io/[a-z0-9][a-z0-9._/-]*@sha256:[a-f0-9]{64}$"},message:"must match pattern \""+"^ghcr\\.io/[a-z0-9][a-z0-9._/-]*@sha256:[a-f0-9]{64}$"+"\""};
if(vErrors === null){
vErrors = [err2];
}
else {
vErrors.push(err2);
}
errors++;
}
}
else {
const err3 = {instancePath:instancePath+"/artifact_uri",schemaPath:"#/allOf/0/then/properties/artifact_uri/type",keyword:"type",params:{type: "string"},message:"must be string"};
if(vErrors === null){
vErrors = [err3];
}
else {
vErrors.push(err3);
}
errors++;
}
}
if(data.oci_digest !== undefined){
let data2 = data.oci_digest;
if(typeof data2 === "string"){
if(!pattern1.test(data2)){
const err4 = {instancePath:instancePath+"/oci_digest",schemaPath:"#/allOf/0/then/properties/oci_digest/pattern",keyword:"pattern",params:{pattern: "^sha256:[a-f0-9]{64}$"},message:"must match pattern \""+"^sha256:[a-f0-9]{64}$"+"\""};
if(vErrors === null){
vErrors = [err4];
}
else {
vErrors.push(err4);
}
errors++;
}
}
else {
const err5 = {instancePath:instancePath+"/oci_digest",schemaPath:"#/allOf/0/then/properties/oci_digest/type",keyword:"type",params:{type: "string"},message:"must be string"};
if(vErrors === null){
vErrors = [err5];
}
else {
vErrors.push(err5);
}
errors++;
}
}
}
var _valid0 = _errs5 === errors;
valid1 = _valid0;
ifClause0 = "then";
}
else {
const _errs10 = errors;
const _errs11 = errors;
const _errs12 = errors;
if(data && typeof data == "object" && !Array.isArray(data)){
let missing0;
if((data.oci_digest === undefined) && (missing0 = "oci_digest")){
const err6 = {};
if(vErrors === null){
vErrors = [err6];
}
else {
vErrors.push(err6);
}
errors++;
}
else {
if(data.oci_digest !== undefined){
let data3 = data.oci_digest;
const _errs13 = errors;
if(errors === _errs13){
if(typeof data3 === "string"){
if(!pattern1.test(data3)){
const err7 = {};
if(vErrors === null){
vErrors = [err7];
}
else {
vErrors.push(err7);
}
errors++;
}
}
else {
const err8 = {};
if(vErrors === null){
vErrors = [err8];
}
else {
vErrors.push(err8);
}
errors++;
}
}
}
}
}
var valid4 = _errs12 === errors;
if(valid4){
const err9 = {instancePath,schemaPath:"#/allOf/0/else/not",keyword:"not",params:{},message:"must NOT be valid"};
if(vErrors === null){
vErrors = [err9];
}
else {
vErrors.push(err9);
}
errors++;
}
else {
errors = _errs11;
if(vErrors !== null){
if(_errs11){
vErrors.length = _errs11;
}
else {
vErrors = null;
}
}
}
if(data && typeof data == "object" && !Array.isArray(data)){
if(data.artifact_uri !== undefined){
let data4 = data.artifact_uri;
if(typeof data4 === "string"){
if(!pattern3.test(data4)){
const err10 = {instancePath:instancePath+"/artifact_uri",schemaPath:"#/allOf/0/else/properties/artifact_uri/pattern",keyword:"pattern",params:{pattern: "^https://"},message:"must match pattern \""+"^https://"+"\""};
if(vErrors === null){
vErrors = [err10];
}
else {
vErrors.push(err10);
}
errors++;
}
}
else {
const err11 = {instancePath:instancePath+"/artifact_uri",schemaPath:"#/allOf/0/else/properties/artifact_uri/type",keyword:"type",params:{type: "string"},message:"must be string"};
if(vErrors === null){
vErrors = [err11];
}
else {
vErrors.push(err11);
}
errors++;
}
}
}
var _valid0 = _errs10 === errors;
valid1 = _valid0;
ifClause0 = "else";
}
if(!valid1){
const err12 = {instancePath,schemaPath:"#/allOf/0/if",keyword:"if",params:{failingKeyword: ifClause0},message:"must match \""+ifClause0+"\" schema"};
if(vErrors === null){
vErrors = [err12];
}
else {
vErrors.push(err12);
}
errors++;
}
if(data && typeof data == "object" && !Array.isArray(data)){
if(data.schema === undefined){
const err13 = {instancePath,schemaPath:"#/required",keyword:"required",params:{missingProperty: "schema"},message:"must have required property '"+"schema"+"'"};
if(vErrors === null){
vErrors = [err13];
}
else {
vErrors.push(err13);
}
errors++;
}
if(data.repository === undefined){
const err14 = {instancePath,schemaPath:"#/required",keyword:"required",params:{missingProperty: "repository"},message:"must have required property '"+"repository"+"'"};
if(vErrors === null){
vErrors = [err14];
}
else {
vErrors.push(err14);
}
errors++;
}
if(data.service_id === undefined){
const err15 = {instancePath,schemaPath:"#/required",keyword:"required",params:{missingProperty: "service_id"},message:"must have required property '"+"service_id"+"'"};
if(vErrors === null){
vErrors = [err15];
}
else {
vErrors.push(err15);
}
errors++;
}
if(data.git_sha === undefined){
const err16 = {instancePath,schemaPath:"#/required",keyword:"required",params:{missingProperty: "git_sha"},message:"must have required property '"+"git_sha"+"'"};
if(vErrors === null){
vErrors = [err16];
}
else {
vErrors.push(err16);
}
errors++;
}
if(data.git_ref === undefined){
const err17 = {instancePath,schemaPath:"#/required",keyword:"required",params:{missingProperty: "git_ref"},message:"must have required property '"+"git_ref"+"'"};
if(vErrors === null){
vErrors = [err17];
}
else {
vErrors.push(err17);
}
errors++;
}
if(data.build_run_id === undefined){
const err18 = {instancePath,schemaPath:"#/required",keyword:"required",params:{missingProperty: "build_run_id"},message:"must have required property '"+"build_run_id"+"'"};
if(vErrors === null){
vErrors = [err18];
}
else {
vErrors.push(err18);
}
errors++;
}
if(data.artifact_type === undefined){
const err19 = {instancePath,schemaPath:"#/required",keyword:"required",params:{missingProperty: "artifact_type"},message:"must have required property '"+"artifact_type"+"'"};
if(vErrors === null){
vErrors = [err19];
}
else {
vErrors.push(err19);
}
errors++;
}
if(data.artifact_uri === undefined){
const err20 = {instancePath,schemaPath:"#/required",keyword:"required",params:{missingProperty: "artifact_uri"},message:"must have required property '"+"artifact_uri"+"'"};
if(vErrors === null){
vErrors = [err20];
}
else {
vErrors.push(err20);
}
errors++;
}
if(data.sha256 === undefined){
const err21 = {instancePath,schemaPath:"#/required",keyword:"required",params:{missingProperty: "sha256"},message:"must have required property '"+"sha256"+"'"};
if(vErrors === null){
vErrors = [err21];
}
else {
vErrors.push(err21);
}
errors++;
}
if(data.version === undefined){
const err22 = {instancePath,schemaPath:"#/required",keyword:"required",params:{missingProperty: "version"},message:"must have required property '"+"version"+"'"};
if(vErrors === null){
vErrors = [err22];
}
else {
vErrors.push(err22);
}
errors++;
}
if(data.build_status === undefined){
const err23 = {instancePath,schemaPath:"#/required",keyword:"required",params:{missingProperty: "build_status"},message:"must have required property '"+"build_status"+"'"};
if(vErrors === null){
vErrors = [err23];
}
else {
vErrors.push(err23);
}
errors++;
}
if(data.test_status === undefined){
const err24 = {instancePath,schemaPath:"#/required",keyword:"required",params:{missingProperty: "test_status"},message:"must have required property '"+"test_status"+"'"};
if(vErrors === null){
vErrors = [err24];
}
else {
vErrors.push(err24);
}
errors++;
}
for(const key0 in data){
if(!(func2.call(schema11.properties, key0))){
const err25 = {instancePath,schemaPath:"#/additionalProperties",keyword:"additionalProperties",params:{additionalProperty: key0},message:"must NOT have additional properties"};
if(vErrors === null){
vErrors = [err25];
}
else {
vErrors.push(err25);
}
errors++;
}
}
if(data.schema !== undefined){
if(1 !== data.schema){
const err26 = {instancePath:instancePath+"/schema",schemaPath:"#/properties/schema/const",keyword:"const",params:{allowedValue: 1},message:"must be equal to constant"};
if(vErrors === null){
vErrors = [err26];
}
else {
vErrors.push(err26);
}
errors++;
}
}
if(data.repository !== undefined){
let data6 = data.repository;
if(typeof data6 === "string"){
if(!pattern4.test(data6)){
const err27 = {instancePath:instancePath+"/repository",schemaPath:"#/properties/repository/pattern",keyword:"pattern",params:{pattern: "^[A-Za-z0-9][A-Za-z0-9_.-]{0,99}/[A-Za-z0-9][A-Za-z0-9_.-]{0,99}$"},message:"must match pattern \""+"^[A-Za-z0-9][A-Za-z0-9_.-]{0,99}/[A-Za-z0-9][A-Za-z0-9_.-]{0,99}$"+"\""};
if(vErrors === null){
vErrors = [err27];
}
else {
vErrors.push(err27);
}
errors++;
}
}
else {
const err28 = {instancePath:instancePath+"/repository",schemaPath:"#/properties/repository/type",keyword:"type",params:{type: "string"},message:"must be string"};
if(vErrors === null){
vErrors = [err28];
}
else {
vErrors.push(err28);
}
errors++;
}
}
if(data.service_id !== undefined){
let data7 = data.service_id;
if(typeof data7 === "string"){
if(!pattern5.test(data7)){
const err29 = {instancePath:instancePath+"/service_id",schemaPath:"#/properties/service_id/pattern",keyword:"pattern",params:{pattern: "^[a-z][a-z0-9-]{0,63}$"},message:"must match pattern \""+"^[a-z][a-z0-9-]{0,63}$"+"\""};
if(vErrors === null){
vErrors = [err29];
}
else {
vErrors.push(err29);
}
errors++;
}
}
else {
const err30 = {instancePath:instancePath+"/service_id",schemaPath:"#/properties/service_id/type",keyword:"type",params:{type: "string"},message:"must be string"};
if(vErrors === null){
vErrors = [err30];
}
else {
vErrors.push(err30);
}
errors++;
}
}
if(data.git_sha !== undefined){
let data8 = data.git_sha;
if(typeof data8 === "string"){
if(!pattern6.test(data8)){
const err31 = {instancePath:instancePath+"/git_sha",schemaPath:"#/properties/git_sha/pattern",keyword:"pattern",params:{pattern: "^[a-f0-9]{40}$"},message:"must match pattern \""+"^[a-f0-9]{40}$"+"\""};
if(vErrors === null){
vErrors = [err31];
}
else {
vErrors.push(err31);
}
errors++;
}
}
else {
const err32 = {instancePath:instancePath+"/git_sha",schemaPath:"#/properties/git_sha/type",keyword:"type",params:{type: "string"},message:"must be string"};
if(vErrors === null){
vErrors = [err32];
}
else {
vErrors.push(err32);
}
errors++;
}
}
if(data.git_ref !== undefined){
let data9 = data.git_ref;
if(typeof data9 === "string"){
if(!pattern7.test(data9)){
const err33 = {instancePath:instancePath+"/git_ref",schemaPath:"#/properties/git_ref/pattern",keyword:"pattern",params:{pattern: "^refs/(heads|tags)/[A-Za-z0-9][A-Za-z0-9_./-]{0,199}$"},message:"must match pattern \""+"^refs/(heads|tags)/[A-Za-z0-9][A-Za-z0-9_./-]{0,199}$"+"\""};
if(vErrors === null){
vErrors = [err33];
}
else {
vErrors.push(err33);
}
errors++;
}
}
else {
const err34 = {instancePath:instancePath+"/git_ref",schemaPath:"#/properties/git_ref/type",keyword:"type",params:{type: "string"},message:"must be string"};
if(vErrors === null){
vErrors = [err34];
}
else {
vErrors.push(err34);
}
errors++;
}
}
if(data.build_run_id !== undefined){
let data10 = data.build_run_id;
if(typeof data10 === "string"){
if(!pattern8.test(data10)){
const err35 = {instancePath:instancePath+"/build_run_id",schemaPath:"#/properties/build_run_id/pattern",keyword:"pattern",params:{pattern: "^[1-9][0-9]{0,19}$"},message:"must match pattern \""+"^[1-9][0-9]{0,19}$"+"\""};
if(vErrors === null){
vErrors = [err35];
}
else {
vErrors.push(err35);
}
errors++;
}
}
else {
const err36 = {instancePath:instancePath+"/build_run_id",schemaPath:"#/properties/build_run_id/type",keyword:"type",params:{type: "string"},message:"must be string"};
if(vErrors === null){
vErrors = [err36];
}
else {
vErrors.push(err36);
}
errors++;
}
}
if(data.artifact_type !== undefined){
let data11 = data.artifact_type;
if(!((((((data11 === "oci") || (data11 === "binary")) || (data11 === "static_bundle")) || (data11 === "worker_bundle")) || (data11 === "jar")) || (data11 === "config_bundle"))){
const err37 = {instancePath:instancePath+"/artifact_type",schemaPath:"#/properties/artifact_type/enum",keyword:"enum",params:{allowedValues: schema11.properties.artifact_type.enum},message:"must be equal to one of the allowed values"};
if(vErrors === null){
vErrors = [err37];
}
else {
vErrors.push(err37);
}
errors++;
}
}
if(data.artifact_uri !== undefined){
let data12 = data.artifact_uri;
if(typeof data12 === "string"){
if(func3(data12) > 1024){
const err38 = {instancePath:instancePath+"/artifact_uri",schemaPath:"#/properties/artifact_uri/maxLength",keyword:"maxLength",params:{limit: 1024},message:"must NOT have more than 1024 characters"};
if(vErrors === null){
vErrors = [err38];
}
else {
vErrors.push(err38);
}
errors++;
}
if(func3(data12) < 1){
const err39 = {instancePath:instancePath+"/artifact_uri",schemaPath:"#/properties/artifact_uri/minLength",keyword:"minLength",params:{limit: 1},message:"must NOT have fewer than 1 characters"};
if(vErrors === null){
vErrors = [err39];
}
else {
vErrors.push(err39);
}
errors++;
}
}
else {
const err40 = {instancePath:instancePath+"/artifact_uri",schemaPath:"#/properties/artifact_uri/type",keyword:"type",params:{type: "string"},message:"must be string"};
if(vErrors === null){
vErrors = [err40];
}
else {
vErrors.push(err40);
}
errors++;
}
}
if(data.sha256 !== undefined){
let data13 = data.sha256;
if(typeof data13 === "string"){
if(!pattern9.test(data13)){
const err41 = {instancePath:instancePath+"/sha256",schemaPath:"#/properties/sha256/pattern",keyword:"pattern",params:{pattern: "^[a-f0-9]{64}$"},message:"must match pattern \""+"^[a-f0-9]{64}$"+"\""};
if(vErrors === null){
vErrors = [err41];
}
else {
vErrors.push(err41);
}
errors++;
}
}
else {
const err42 = {instancePath:instancePath+"/sha256",schemaPath:"#/properties/sha256/type",keyword:"type",params:{type: "string"},message:"must be string"};
if(vErrors === null){
vErrors = [err42];
}
else {
vErrors.push(err42);
}
errors++;
}
}
if(data.oci_digest !== undefined){
let data14 = data.oci_digest;
if(typeof data14 === "string"){
if(!pattern1.test(data14)){
const err43 = {instancePath:instancePath+"/oci_digest",schemaPath:"#/properties/oci_digest/pattern",keyword:"pattern",params:{pattern: "^sha256:[a-f0-9]{64}$"},message:"must match pattern \""+"^sha256:[a-f0-9]{64}$"+"\""};
if(vErrors === null){
vErrors = [err43];
}
else {
vErrors.push(err43);
}
errors++;
}
}
else {
const err44 = {instancePath:instancePath+"/oci_digest",schemaPath:"#/properties/oci_digest/type",keyword:"type",params:{type: "string"},message:"must be string"};
if(vErrors === null){
vErrors = [err44];
}
else {
vErrors.push(err44);
}
errors++;
}
}
if(data.version !== undefined){
let data15 = data.version;
if(typeof data15 === "string"){
if(func3(data15) > 128){
const err45 = {instancePath:instancePath+"/version",schemaPath:"#/properties/version/maxLength",keyword:"maxLength",params:{limit: 128},message:"must NOT have more than 128 characters"};
if(vErrors === null){
vErrors = [err45];
}
else {
vErrors.push(err45);
}
errors++;
}
if(!pattern11.test(data15)){
const err46 = {instancePath:instancePath+"/version",schemaPath:"#/properties/version/pattern",keyword:"pattern",params:{pattern: "^[0-9]+\\.[0-9]+\\.[0-9]+(?:-[0-9A-Za-z.-]+)?(?:\\+[0-9A-Za-z.-]+)?$"},message:"must match pattern \""+"^[0-9]+\\.[0-9]+\\.[0-9]+(?:-[0-9A-Za-z.-]+)?(?:\\+[0-9A-Za-z.-]+)?$"+"\""};
if(vErrors === null){
vErrors = [err46];
}
else {
vErrors.push(err46);
}
errors++;
}
}
else {
const err47 = {instancePath:instancePath+"/version",schemaPath:"#/properties/version/type",keyword:"type",params:{type: "string"},message:"must be string"};
if(vErrors === null){
vErrors = [err47];
}
else {
vErrors.push(err47);
}
errors++;
}
}
if(data.build_status !== undefined){
if("PASSED" !== data.build_status){
const err48 = {instancePath:instancePath+"/build_status",schemaPath:"#/properties/build_status/const",keyword:"const",params:{allowedValue: "PASSED"},message:"must be equal to constant"};
if(vErrors === null){
vErrors = [err48];
}
else {
vErrors.push(err48);
}
errors++;
}
}
if(data.test_status !== undefined){
if("PASSED" !== data.test_status){
const err49 = {instancePath:instancePath+"/test_status",schemaPath:"#/properties/test_status/const",keyword:"const",params:{allowedValue: "PASSED"},message:"must be equal to constant"};
if(vErrors === null){
vErrors = [err49];
}
else {
vErrors.push(err49);
}
errors++;
}
}
if(data.provenance_uri !== undefined){
let data18 = data.provenance_uri;
if(typeof data18 === "string"){
if(func3(data18) > 1024){
const err50 = {instancePath:instancePath+"/provenance_uri",schemaPath:"#/properties/provenance_uri/maxLength",keyword:"maxLength",params:{limit: 1024},message:"must NOT have more than 1024 characters"};
if(vErrors === null){
vErrors = [err50];
}
else {
vErrors.push(err50);
}
errors++;
}
}
else {
const err51 = {instancePath:instancePath+"/provenance_uri",schemaPath:"#/properties/provenance_uri/type",keyword:"type",params:{type: "string"},message:"must be string"};
if(vErrors === null){
vErrors = [err51];
}
else {
vErrors.push(err51);
}
errors++;
}
}
if(data.sbom_uri !== undefined){
let data19 = data.sbom_uri;
if(typeof data19 === "string"){
if(func3(data19) > 1024){
const err52 = {instancePath:instancePath+"/sbom_uri",schemaPath:"#/properties/sbom_uri/maxLength",keyword:"maxLength",params:{limit: 1024},message:"must NOT have more than 1024 characters"};
if(vErrors === null){
vErrors = [err52];
}
else {
vErrors.push(err52);
}
errors++;
}
}
else {
const err53 = {instancePath:instancePath+"/sbom_uri",schemaPath:"#/properties/sbom_uri/type",keyword:"type",params:{type: "string"},message:"must be string"};
if(vErrors === null){
vErrors = [err53];
}
else {
vErrors.push(err53);
}
errors++;
}
}
if(data.candidate_id !== undefined){
let data20 = data.candidate_id;
if(typeof data20 === "string"){
if(!pattern12.test(data20)){
const err54 = {instancePath:instancePath+"/candidate_id",schemaPath:"#/properties/candidate_id/pattern",keyword:"pattern",params:{pattern: "^rc_[a-f0-9]{64}$"},message:"must match pattern \""+"^rc_[a-f0-9]{64}$"+"\""};
if(vErrors === null){
vErrors = [err54];
}
else {
vErrors.push(err54);
}
errors++;
}
}
else {
const err55 = {instancePath:instancePath+"/candidate_id",schemaPath:"#/properties/candidate_id/type",keyword:"type",params:{type: "string"},message:"must be string"};
if(vErrors === null){
vErrors = [err55];
}
else {
vErrors.push(err55);
}
errors++;
}
}
}
else {
const err56 = {instancePath,schemaPath:"#/type",keyword:"type",params:{type: "object"},message:"must be object"};
if(vErrors === null){
vErrors = [err56];
}
else {
vErrors.push(err56);
}
errors++;
}
validate10.errors = vErrors;
return errors === 0;
}


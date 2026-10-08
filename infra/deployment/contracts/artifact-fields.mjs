/** @param {string} value */
function httpsLocator(value) {
  try {
    const url = new URL(value);
    return url.protocol === 'https:' && url.hostname && !url.username && !url.password && !url.search && !url.hash;
  } catch { return false; }
}

/**
 * @typedef {{schema:1,repository:string,service_id:string,git_sha:string,git_ref:string,
 * build_run_id:string,artifact_type:'oci'|'binary'|'static_bundle'|'worker_bundle'|'jar'|'config_bundle',
 * artifact_uri:string,sha256:string,version:string,build_status:'PASSED',test_status:'PASSED',
 * oci_digest?:string,provenance_uri?:string,sbom_uri?:string,candidate_id?:string}} ArtifactInput
 */
/** Shared semantics after the canonical JSON schema validates shape.
 * @param {ArtifactInput} input
 * @param {(value:unknown)=>boolean} validate
 */
export function artifactFields(input,validate) {
  if (!validate(input)) throw Error('Invalid artifact contract');
  if (input.artifact_type === 'oci') {
    if (input.oci_digest !== `sha256:${input.sha256}` || !input.artifact_uri.endsWith(`@${input.oci_digest}`)) throw Error('Artifact digest and locator disagree');
    const segments = input.artifact_uri.slice('ghcr.io/'.length).split('@')[0].split('/');
    if (segments.length < 2 || segments.some(segment => !/^[a-z0-9]+(?:[._-][a-z0-9]+)*$/.test(segment))) throw Error('Invalid OCI image path');
  } else if (!httpsLocator(input.artifact_uri)) throw Error('Artifact locator must be HTTPS without credentials, query or fragment');
  for (const value of [input.provenance_uri,input.sbom_uri]) if (value !== undefined && !httpsLocator(value)) throw Error('Evidence locator must be HTTPS without credentials, query or fragment');
  const normalized={...input,repository:input.repository.toLowerCase()};
  return {normalized,identity:[normalized.repository,input.git_sha,input.service_id,input.artifact_type,input.sha256].join('\0')};
}

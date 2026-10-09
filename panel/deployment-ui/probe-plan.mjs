export function probeRequest(candidate,agent,now=Math.floor(Date.now()/1000)) {
  if(!candidate || !/^rc_[a-f0-9]{64}$/.test(candidate.candidate_id ?? '')
    || candidate.service_id!=='deployment-probe' || candidate.repository!=='veloramcdev/velora-launcher'
    || candidate.git_ref!=='refs/heads/main' || !/^[a-f0-9]{40}$/.test(candidate.git_sha ?? '')
    || candidate.artifact_type!=='oci' || candidate.build_status!=='PASSED' || candidate.test_status!=='PASSED'
    || !/^[a-f0-9]{64}$/.test(candidate.sha256 ?? '')
    || candidate.artifact_uri!=='ghcr.io/veloramcdev/deployment-probe@sha256:'+candidate.sha256
    || candidate.oci_digest!=='sha256:'+candidate.sha256)throw Error('Select a tested monorepo probe image.');
  if(!agent || !/^agt_[a-f0-9]{64}$/.test(agent.id ?? '') || agent.status!=='ONLINE'
    || agent.environment!=='development' || agent.heartbeat?.host?.os!=='linux' || agent.heartbeat?.host?.arch!=='x86_64'
    || !Number.isSafeInteger(agent.heartbeat_at) || agent.heartbeat_at<=now-120
    || agent.heartbeat_at>now+30)throw Error('Select an active machine with a recent heartbeat.');
  return {schema:1,candidate_id:candidate.candidate_id,agent_id:agent.id};
}

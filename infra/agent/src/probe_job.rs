//! The first execution policy permits only the stateless Development probe.
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use ed25519_dalek::{Signature, VerifyingKey};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ProbeJob {
    pub schema: u32,
    pub id: String,
    pub deployment_id: String,
    pub agent_id: String,
    pub environment: String,
    pub action: String,
    pub service_id: String,
    pub image: String,
    pub expected_commit: String,
    pub issued_at: u64,
    pub expires_at: u64,
}
#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Envelope {
    pub schema: u32,
    pub key_id: String,
    pub payload: String,
    pub signature: String,
}
fn hex_string(value: &str, length: usize) -> bool {
    value.len() == length && value.bytes().all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}
impl ProbeJob {
    pub fn validate(&self, agent_id: &str, now: u64) -> Result<(), &'static str> {
        let image_prefix = "ghcr.io/veloramcdev/deployment-probe@sha256:";
        if self.schema != 1
            || !super::valid_nonce(&self.id)
            || !super::valid_nonce(&self.deployment_id)
            || self.agent_id != agent_id
            || !agent_id.strip_prefix("agt_").is_some_and(|id| hex_string(id, 64))
            || self.environment != "development"
            || self.action != "DEPLOY_SERVICE"
            || self.service_id != "deployment-probe"
            || !self.image.strip_prefix(image_prefix).is_some_and(|digest| hex_string(digest, 64))
            || !hex_string(&self.expected_commit, 40)
            || self.issued_at == 0
            || self.issued_at > now.saturating_add(5)
            || self.expires_at <= now
            || self.expires_at <= self.issued_at
            || self.expires_at - self.issued_at > 300
        {
            return Err("Probe execution policy denied the job");
        }
        Ok(())
    }
    /// Only fixed settings and signed immutable image coordinates reach Compose.
    pub fn compose(&self) -> serde_json::Value {
        serde_json::json!({"services":{"probe":{
            "image":self.image,"platform":"linux/amd64","user":"65532:65532",
            "container_name":"velora-guide-development-probe",
            "labels":{"io.velora.managed":"development-probe-v1"},
            "read_only":true,"cap_drop":["ALL"],"security_opt":["no-new-privileges:true"],
            "mem_limit":"96m","cpus":0.25,"pids_limit":64,
            "ports":["127.0.0.1:18985:8080"],"restart":"unless-stopped",
            "tmpfs":["/tmp:rw,noexec,nosuid,size=8388608"]
        }}})
    }
}
pub fn verify(envelope: &Envelope, trusted_public_key: &str, agent_id: &str, now: u64) -> Result<ProbeJob, &'static str> {
    if envelope.schema != 1 || envelope.payload.len() > 4096 {
        return Err("Invalid probe job envelope");
    }
    let raw: [u8; 32] = URL_SAFE_NO_PAD
        .decode(trusted_public_key)
        .map_err(|_| "Invalid trusted probe key")?
        .try_into()
        .map_err(|_| "Invalid trusted probe key")?;
    if envelope.key_id != hex::encode(Sha256::digest(raw)) {
        return Err("Probe signing key is not trusted");
    }
    let bytes = URL_SAFE_NO_PAD.decode(&envelope.payload).map_err(|_| "Invalid probe payload")?;
    if URL_SAFE_NO_PAD.encode(&bytes) != envelope.payload {
        return Err("Invalid probe payload");
    }
    let mut message = b"velora-probe-job-v1\n".to_vec();
    message.extend_from_slice(&bytes);
    let signature = Signature::from_slice(&URL_SAFE_NO_PAD.decode(&envelope.signature).map_err(|_| "Invalid probe signature")?)
        .map_err(|_| "Invalid probe signature")?;
    VerifyingKey::from_bytes(&raw)
        .map_err(|_| "Invalid trusted probe key")?
        .verify_strict(&message, &signature)
        .map_err(|_| "Probe signature denied")?;
    let job: ProbeJob = serde_json::from_slice(&bytes).map_err(|_| "Invalid probe job")?;
    job.validate(agent_id, now)?;
    Ok(job)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn shared_worker_signature_and_strict_execution_policy() {
        let fixture: serde_json::Value = serde_json::from_str(include_str!("../../deployment/contracts/probe-job.fixture.json")).unwrap();
        let envelope: Envelope = serde_json::from_value(fixture["envelope"].clone()).unwrap();
        let key = fixture["public_key"].as_str().unwrap();
        let agent = fixture["agent_id"].as_str().unwrap();
        let now = fixture["now"].as_u64().unwrap();
        let mut job = verify(&envelope, key, agent, now).unwrap();
        assert!(verify(&envelope, key, agent, now + 300).is_err());
        assert!(verify(&envelope, key, &format!("agt_{}", "f".repeat(64)), now).is_err());
        let manifest = job.compose();
        assert_eq!(manifest["services"]["probe"]["image"], job.image);
        assert_eq!(manifest["services"]["probe"]["ports"][0], "127.0.0.1:18985:8080");
        assert!(manifest.get("networks").is_none(), "an internal network would make the published loopback port unreachable");
        assert!(manifest["services"]["probe"].get("volumes").is_none());
        assert!(manifest["services"]["probe"].get("command").is_none());
        job.image = "ghcr.io/veloramcdev/deployment-probe:latest".into();
        assert!(job.validate(agent, now).is_err());
        job.image = "ghcr.io/attacker/image@sha256:".to_owned() + &"b".repeat(64);
        assert!(job.validate(agent, now).is_err());
        job.environment = "production".into();
        assert!(job.validate(agent, now).is_err());
    }
}

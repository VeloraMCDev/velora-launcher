use serde_json::{json, Value};
use std::{
    io::{BufRead, IsTerminal, Read},
    path::Path,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};
use velora_agent::{new_nonce, Config, Identity};
use zeroize::Zeroizing;

const VERSION: &str = env!("CARGO_PKG_VERSION");
const SOURCE: &str = env!("VELORA_AGENT_BUILD_SHA");

fn epoch() -> Result<u64, &'static str> {
    Ok(SystemTime::now().duration_since(UNIX_EPOCH).map_err(|_| "Invalid system clock")?.as_secs())
}
fn send(config: &Config, identity: &Identity, path: &str, body: &[u8]) -> Result<Value, &'static str> {
    if SOURCE == "UNBUILT" {
        return Err("Network operation requires a source-stamped CI artifact");
    }
    let headers = identity.sign_request(path, &epoch()?.to_string(), &new_nonce()?, body)?;
    let client = reqwest::blocking::Client::builder()
        .https_only(true)
        .redirect(reqwest::redirect::Policy::none())
        .connect_timeout(Duration::from_secs(5))
        .timeout(Duration::from_secs(20))
        .build()
        .map_err(|_| "HTTPS client unavailable")?;
    let origin = reqwest::Url::parse(&config.control_origin).map_err(|_| "Invalid control origin")?;
    let endpoint = origin.join(path).map_err(|_| "Invalid endpoint")?;
    let response = client
        .post(endpoint)
        .header("content-type", "application/json")
        .header("velora-agent-id", headers.agent_id)
        .header("velora-key-id", headers.key_id)
        .header("velora-timestamp", headers.timestamp)
        .header("velora-nonce", headers.nonce)
        .header("velora-body-sha256", headers.body_sha256)
        .header("velora-signature", headers.signature)
        .body(body.to_vec())
        .send()
        .map_err(|_| "Control plane unavailable")?;
    if response.status().is_server_error() || response.status().as_u16() == 429 {
        return Err("Control plane temporarily unavailable");
    }
    if !response.status().is_success() {
        return Err("Control plane rejected the request");
    }
    let mut bytes = Vec::new();
    response.take(16385).read_to_end(&mut bytes).map_err(|_| "Response unavailable")?;
    if bytes.len() > 16384 {
        return Err("Response too large");
    }
    serde_json::from_slice(&bytes).map_err(|_| "Invalid control plane response")
}
fn heartbeat(config: &Config, identity: &Identity, started: Instant) -> Result<(), &'static str> {
    let body=serde_json::to_vec(&json!({"schema":1,"agent_version":VERSION,"build_sha":SOURCE,"uptime_seconds":started.elapsed().as_secs(),"capabilities":["REPORT_HEARTBEAT"],"host":{"os":std::env::consts::OS,"arch":std::env::consts::ARCH}})).map_err(|_|"Heartbeat encoding failed")?;
    let receipt = send(config, identity, "/api/v1/agents/heartbeat", &body)?;
    if receipt.get("agent_id").and_then(Value::as_str) != Some(identity.agent_id().as_str())
        || receipt.get("accepted") != Some(&Value::Bool(true))
    {
        return Err("Invalid heartbeat receipt");
    }
    Ok(())
}
fn run() -> Result<(), &'static str> {
    let args = std::env::args().skip(1).collect::<Vec<_>>();
    if args == ["--version"] {
        println!("velora-agent {VERSION} {SOURCE}");
        return Ok(());
    }
    if (args.len() == 7 && args[0] == "probe-poll" && args[1] == "--config" && args[3] == "--policy" && args[5] == "--job")
        || (args.len() == 5 && args[0] == "probe-report" && args[1] == "--config" && args[3] == "--receipt")
    {
        if SOURCE == "UNBUILT" || std::env::consts::OS != "linux" {
            return Err("Probe transport requires a source-stamped Linux CI artifact");
        }
        let config = Config::load(Path::new(&args[2]))?;
        let identity = Identity::load(&config.identity_path)?;
        if args[0] == "probe-poll" {
            let policy: velora_agent::probe_executor::Policy = velora_agent::probe_transport::read(Path::new(&args[4]))?;
            if policy.agent_id != identity.agent_id() {
                return Err("Probe transport policy denied");
            }
            let response = send(&config, &identity, "/api/v1/agents/probe/poll", br#"{"schema":1}"#)?;
            let value = response.get("job").ok_or("Invalid probe poll receipt")?;
            if value.is_null() {
                println!("No probe job available.");
                return Ok(());
            }
            let job = serde_json::from_value(value.clone()).map_err(|_| "Invalid signed probe job")?;
            velora_agent::probe_transport::stage(&policy, &job, &identity.agent_id(), Path::new(&args[6]), epoch()?)?;
            println!("Verified probe job staged; no host mutation performed.");
        } else {
            let receipt: velora_agent::probe_executor::Receipt = velora_agent::probe_transport::read(Path::new(&args[4]))?;
            let body = serde_json::to_vec(&json!({"schema":1,"receipt":receipt})).map_err(|_| "Probe result encoding failed")?;
            let response = send(&config, &identity, "/api/v1/agents/probe/result", &body)?;
            if response.get("accepted") != Some(&Value::Bool(true))
                || response.get("job_id").and_then(Value::as_str) != Some(receipt.job_id.as_str())
                || response.get("status").and_then(Value::as_str) != Some(receipt.status.as_str())
            {
                return Err("Invalid probe result receipt");
            }
            println!("Probe result accepted.");
        }
        return Ok(());
    }
    if args.len() == 5 && args[0] == "probe-execute" && args[1] == "--policy" && args[3] == "--job" {
        if SOURCE == "UNBUILT" || std::env::consts::OS != "linux" {
            return Err("Probe execution requires a source-stamped Linux CI artifact");
        }
        let result = velora_agent::probe_executor::from_files(Path::new(&args[2]), Path::new(&args[4]), epoch()?)?;
        println!("{}", serde_json::to_string(&result).map_err(|_| "Probe receipt encoding failed")?);
        return if result.status == "SUCCEEDED" { Ok(()) } else { Err("Probe deployment did not succeed; see bounded receipt") };
    }
    if args.len() != 3 || args[1] != "--config" || !["doctor", "identity-init", "enroll", "heartbeat", "daemon"].contains(&args[0].as_str())
    {
        return Err("Usage: velora-agent doctor|identity-init|enroll|heartbeat|daemon --config PATH");
    }
    let config = Config::load(Path::new(&args[2]))?;
    if args[0] == "doctor" {
        let ready = Identity::load(&config.identity_path).is_ok();
        println!(
            "{}",
            json!({"config_valid":true,"os":std::env::consts::OS,"arch":std::env::consts::ARCH,"identity_ready":ready,"systemd_available":Path::new("/run/systemd/system").is_dir(),"capabilities":["REPORT_HEARTBEAT"],"version":VERSION,"build_sha":SOURCE})
        );
        return if std::env::consts::OS == "linux" { Ok(()) } else { Err("Initial native agent target is Linux") };
    }
    if std::env::consts::OS != "linux" {
        return Err("Initial native agent target is Linux");
    }
    if args[0] == "identity-init" {
        Identity::create(&config.identity_path)?;
        println!("Identity created locally. Private key is never exported.");
        return Ok(());
    }
    let identity = Identity::load(&config.identity_path)?;
    if args[0] == "enroll" {
        eprintln!("Paste the one-use enrollment token, then press Enter. The token is never saved or logged.");
        let mut token = Zeroizing::new(String::new());
        if std::io::stdin().is_terminal() {
            *token = rpassword::read_password().map_err(|_| "Enrollment input unavailable")?;
        } else {
            std::io::stdin().lock().take(256).read_line(&mut token).map_err(|_| "Enrollment input unavailable")?;
        }
        let token = token.trim();
        if token.len() != 43 || !token.bytes().all(|byte| byte.is_ascii_alphanumeric() || b"_-".contains(&byte)) {
            return Err("Invalid enrollment token format");
        }
        let body = Zeroizing::new(
            serde_json::to_vec(
                &json!({"schema":1,"token":token,"public_key":identity.public_key(),"agent_version":VERSION,"build_sha":SOURCE}),
            )
            .map_err(|_| "Enrollment encoding failed")?,
        );
        let receipt = send(&config, &identity, "/api/v1/agents/enroll", &body)?;
        if receipt.get("agent_id").and_then(Value::as_str) != Some(identity.agent_id().as_str())
            || receipt.get("key_id").and_then(Value::as_str) != Some(identity.key_id().as_str())
            || receipt.get("environment").and_then(Value::as_str) != Some("development")
            || receipt.get("capabilities") != Some(&json!(["REPORT_HEARTBEAT"]))
        {
            return Err("Invalid enrollment receipt");
        }
        println!("Enrollment complete: {}", identity.agent_id());
        return Ok(());
    }
    let started = Instant::now();
    if args[0] == "heartbeat" {
        heartbeat(&config, &identity, started)?;
        println!("Heartbeat accepted.");
        return Ok(());
    }
    let mut backoff = config.heartbeat_seconds;
    loop {
        match heartbeat(&config, &identity, started) {
            Ok(()) => {
                backoff = config.heartbeat_seconds;
                println!("{}", json!({"event":"heartbeat_accepted"}));
            }
            Err(message) => {
                if !["Control plane unavailable", "Control plane temporarily unavailable", "Response unavailable"].contains(&message) {
                    return Err(message);
                }
                backoff = (backoff * 2).min(300);
                eprintln!("{}", json!({"event":"heartbeat_unavailable","retry_seconds":backoff}));
            }
        }
        let jitter = (epoch()? % 6).min(300 - backoff);
        std::thread::sleep(Duration::from_secs(backoff + jitter));
    }
}
fn main() {
    if let Err(message) = run() {
        eprintln!("{message}");
        // Systemd must not restart an identity rejected or revoked by the origin.
        let daemon = std::env::args().nth(1).as_deref() == Some("daemon");
        std::process::exit(if daemon { 78 } else { 1 });
    }
}

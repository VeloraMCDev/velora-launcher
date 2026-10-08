//! Minecraft Server List Ping: online players, MOTD, version, latency.

use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};
use std::time::{Duration, Instant};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;

#[derive(Debug, Clone, Serialize, Default)]
pub struct ServerStatus {
    pub online: bool,
    pub version: String,
    pub protocol: i64,
    pub players_online: i64,
    pub players_max: i64,
    pub sample: Vec<String>,
    /// MOTD using legacy `§` colour codes (rendered by the UI).
    pub motd: String,
    /// `data:image/png;base64,...`
    pub favicon: Option<String>,
    pub latency_ms: u64,
}

fn write_varint(buf: &mut Vec<u8>, mut value: i32) {
    loop {
        let mut byte = (value & 0x7f) as u8;
        value = ((value as u32) >> 7) as i32;
        if value != 0 {
            byte |= 0x80;
        }
        buf.push(byte);
        if value == 0 {
            break;
        }
    }
}

async fn read_varint(stream: &mut TcpStream) -> Result<i32> {
    let mut result = 0i32;
    for i in 0..5 {
        let byte = stream.read_u8().await?;
        result |= ((byte & 0x7f) as i32) << (7 * i);
        if byte & 0x80 == 0 {
            return Ok(result);
        }
    }
    bail!("varint too long")
}

fn packet(id: i32, body: &[u8]) -> Vec<u8> {
    let mut inner = Vec::new();
    write_varint(&mut inner, id);
    inner.extend_from_slice(body);
    let mut out = Vec::new();
    write_varint(&mut out, inner.len() as i32);
    out.extend(inner);
    out
}

#[derive(Deserialize)]
struct RawStatus {
    #[serde(default)]
    version: Option<RawVersion>,
    #[serde(default)]
    players: Option<RawPlayers>,
    #[serde(default)]
    description: Option<serde_json::Value>,
    #[serde(default)]
    favicon: Option<String>,
}
#[derive(Deserialize)]
struct RawVersion {
    #[serde(default)]
    name: String,
    #[serde(default)]
    protocol: i64,
}
#[derive(Deserialize)]
struct RawPlayers {
    #[serde(default)]
    max: i64,
    #[serde(default)]
    online: i64,
    #[serde(default)]
    sample: Vec<RawSample>,
}
#[derive(Deserialize)]
struct RawSample {
    #[serde(default)]
    name: String,
}

fn color_code(name: &str) -> Option<char> {
    Some(match name {
        "black" => '0',
        "dark_blue" => '1',
        "dark_green" => '2',
        "dark_aqua" => '3',
        "dark_red" => '4',
        "dark_purple" => '5',
        "gold" => '6',
        "gray" => '7',
        "dark_gray" => '8',
        "blue" => '9',
        "green" => 'a',
        "aqua" => 'b',
        "red" => 'c',
        "light_purple" => 'd',
        "yellow" => 'e',
        "white" => 'f',
        _ => return None,
    })
}

/// Flatten a chat component into a `§`-coded string.
pub fn flatten_chat(value: &serde_json::Value) -> String {
    let mut out = String::new();
    fn walk(v: &serde_json::Value, out: &mut String) {
        match v {
            serde_json::Value::String(s) => out.push_str(s),
            serde_json::Value::Array(items) => items.iter().for_each(|i| walk(i, out)),
            serde_json::Value::Object(map) => {
                if let Some(c) = map.get("color").and_then(|c| c.as_str()) {
                    if let Some(code) = color_code(c) {
                        out.push('§');
                        out.push(code);
                    } else if c.starts_with('#') && c.len() == 7 {
                        // Hex colours: emit a custom marker the UI understands.
                        out.push_str("§#");
                        out.push_str(&c[1..]);
                    }
                }
                for (key, code) in [("bold", 'l'), ("italic", 'o'), ("underlined", 'n'), ("strikethrough", 'm')] {
                    if map.get(key).and_then(|b| b.as_bool()) == Some(true) {
                        out.push('§');
                        out.push(code);
                    }
                }
                if let Some(t) = map.get("text").and_then(|t| t.as_str()) {
                    out.push_str(t);
                }
                if let Some(extra) = map.get("extra") {
                    walk(extra, out);
                }
                if map.contains_key("color") {
                    out.push_str("§r");
                }
            }
            _ => {}
        }
    }
    walk(value, &mut out);
    out
}

pub async fn ping(host: &str, port: u16) -> Result<ServerStatus> {
    tokio::time::timeout(Duration::from_secs(5), ping_inner(host, port)).await.context("server did not respond in time")?
}

async fn ping_inner(host: &str, port: u16) -> Result<ServerStatus> {
    let started = Instant::now();
    let mut stream = TcpStream::connect((host, port)).await.with_context(|| format!("connecting to {host}:{port}"))?;
    stream.set_nodelay(true).ok();
    let connect_ms = started.elapsed().as_millis() as u64;

    let mut body = Vec::new();
    write_varint(&mut body, -1); // protocol: "whatever you are"
    write_varint(&mut body, host.len() as i32);
    body.extend_from_slice(host.as_bytes());
    body.extend_from_slice(&port.to_be_bytes());
    write_varint(&mut body, 1); // next state: status
    stream.write_all(&packet(0x00, &body)).await?;
    stream.write_all(&packet(0x00, &[])).await?;

    let _len = read_varint(&mut stream).await?;
    let id = read_varint(&mut stream).await?;
    if id != 0x00 {
        bail!("unexpected packet {id}");
    }
    let str_len = read_varint(&mut stream).await? as usize;
    if str_len > 1 << 21 {
        bail!("status response too large");
    }
    let mut json = vec![0u8; str_len];
    stream.read_exact(&mut json).await?;

    // Latency via ping/pong (fallback: TCP connect time).
    let t = Instant::now();
    let mut latency_ms = connect_ms;
    if stream.write_all(&packet(0x01, &42i64.to_be_bytes())).await.is_ok() {
        if let Ok(Ok(_)) = tokio::time::timeout(Duration::from_secs(2), async {
            let _ = read_varint(&mut stream).await?;
            let _ = read_varint(&mut stream).await?;
            stream.read_i64().await.map_err(anyhow::Error::from)
        })
        .await
        {
            latency_ms = t.elapsed().as_millis() as u64;
        }
    }

    let raw: RawStatus = serde_json::from_slice(&json).context("invalid status JSON")?;
    let players = raw.players.unwrap_or(RawPlayers { max: 0, online: 0, sample: vec![] });
    let version = raw.version.unwrap_or(RawVersion { name: String::new(), protocol: 0 });
    Ok(ServerStatus {
        online: true,
        version: version.name,
        protocol: version.protocol,
        players_online: players.online,
        players_max: players.max,
        sample: players.sample.into_iter().map(|s| s.name).filter(|n| !n.is_empty()).take(12).collect(),
        motd: raw.description.as_ref().map(flatten_chat).unwrap_or_default(),
        favicon: raw.favicon,
        latency_ms,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn varints() {
        let mut b = Vec::new();
        write_varint(&mut b, 300);
        assert_eq!(b, vec![0xac, 0x02]);
        let mut b = Vec::new();
        write_varint(&mut b, -1);
        assert_eq!(b, vec![0xff, 0xff, 0xff, 0xff, 0x0f]);
    }

    #[test]
    fn flattens_motd() {
        let v: serde_json::Value =
            serde_json::from_str(r#"{"text":"","extra":[{"text":"Scope","color":"aqua","bold":true},{"text":"Net"}]}"#).unwrap();
        assert_eq!(flatten_chat(&v), "§b§lScope§rNet");
        assert_eq!(flatten_chat(&serde_json::json!("§aHello")), "§aHello");
    }

    #[tokio::test]
    async fn pings_a_fake_server() {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();
        tokio::spawn(async move {
            let (mut sock, _) = listener.accept().await.unwrap();
            // read handshake + request (we don't validate them here)
            let mut buf = [0u8; 256];
            let _ = sock.read(&mut buf).await.unwrap();
            let json = br#"{"version":{"name":"1.21.1","protocol":767},"players":{"max":100,"online":7,"sample":[{"name":"Steve","id":"x"}]},"description":"A server"}"#;
            let mut body = Vec::new();
            write_varint(&mut body, json.len() as i32);
            body.extend_from_slice(json);
            sock.write_all(&packet(0, &body)).await.unwrap();
            let mut ping = [0u8; 64];
            let _ = sock.read(&mut ping).await;
            sock.write_all(&packet(1, &42i64.to_be_bytes())).await.ok();
        });
        let s = ping("127.0.0.1", port).await.unwrap();
        assert!(s.online);
        assert_eq!(s.players_online, 7);
        assert_eq!(s.version, "1.21.1");
        assert_eq!(s.sample, vec!["Steve"]);
        assert_eq!(s.motd, "A server");
    }
}

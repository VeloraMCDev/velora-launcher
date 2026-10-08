//! Adds the instance's server to Minecraft's multiplayer list.

use anyhow::Result;
use fastnbt::Value;
use std::collections::HashMap;
use std::path::Path;

/// Insert (or update) `name`/`address` at the top of `servers.dat`,
/// preserving every other entry the player has added.
pub fn upsert(game_dir: &Path, name: &str, address: &str) -> Result<()> {
    let path = game_dir.join("servers.dat");
    let mut root: HashMap<String, Value> = match std::fs::read(&path) {
        Ok(bytes) => match fastnbt::from_bytes::<Value>(&bytes) {
            Ok(Value::Compound(map)) => map,
            _ => HashMap::new(),
        },
        Err(_) => HashMap::new(),
    };

    let mut servers = match root.remove("servers") {
        Some(Value::List(list)) => list,
        _ => Vec::new(),
    };

    let same = |v: &Value| match v {
        Value::Compound(m) => matches!(m.get("ip"), Some(Value::String(ip)) if ip.eq_ignore_ascii_case(address)),
        _ => false,
    };
    let existing = servers.iter().position(same);
    let mut entry = match existing {
        Some(i) => match servers.remove(i) {
            Value::Compound(m) => m,
            _ => HashMap::new(),
        },
        None => HashMap::new(),
    };
    entry.insert("name".into(), Value::String(name.to_string()));
    entry.insert("ip".into(), Value::String(address.to_string()));
    entry.entry("acceptTextures".into()).or_insert(Value::Byte(1));
    servers.insert(0, Value::Compound(entry));

    root.insert("servers".into(), Value::List(servers));
    std::fs::create_dir_all(game_dir)?;
    std::fs::write(&path, fastnbt::to_bytes(&Value::Compound(root))?)?;
    Ok(())
}

pub fn format_address(host: &str, port: u16) -> String {
    if port == 25565 {
        host.to_string()
    } else {
        format!("{host}:{port}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn names(dir: &Path) -> Vec<(String, String)> {
        let v: Value = fastnbt::from_bytes(&std::fs::read(dir.join("servers.dat")).unwrap()).unwrap();
        let Value::Compound(root) = v else { panic!() };
        let Some(Value::List(list)) = root.get("servers") else { panic!() };
        list.iter()
            .map(|s| match s {
                Value::Compound(m) => match (m.get("name"), m.get("ip")) {
                    (Some(Value::String(n)), Some(Value::String(i))) => (n.clone(), i.clone()),
                    _ => panic!(),
                },
                _ => panic!(),
            })
            .collect()
    }

    #[test]
    fn upserts_without_losing_player_servers() {
        let dir = tempfile::tempdir().unwrap();
        upsert(dir.path(), "Friend", "friend.net").unwrap();
        upsert(dir.path(), "SCOPENET", "play.scopenet.gg").unwrap();
        upsert(dir.path(), "SCOPENET SMP", "play.scopenet.gg").unwrap();
        assert_eq!(names(dir.path()), vec![("SCOPENET SMP".into(), "play.scopenet.gg".into()), ("Friend".into(), "friend.net".into())]);
        assert_eq!(format_address("a.b", 25565), "a.b");
        assert_eq!(format_address("a.b", 25570), "a.b:25570");
    }
}

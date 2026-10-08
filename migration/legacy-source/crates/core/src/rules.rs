//! Mojang's `rules` mini-language used by libraries and arguments.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Rule {
    pub action: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub os: Option<OsRule>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub features: Option<HashMap<String, bool>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct OsRule {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub arch: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
}

#[derive(Debug, Clone)]
pub struct Env {
    /// "windows" | "osx" | "linux"
    pub os: &'static str,
    /// "x86_64" | "x86" | "arm64"
    pub arch: &'static str,
    pub features: HashMap<String, bool>,
}

impl Env {
    pub fn current() -> Self {
        let os = if cfg!(windows) {
            "windows"
        } else if cfg!(target_os = "macos") {
            "osx"
        } else {
            "linux"
        };
        let arch = if cfg!(target_arch = "x86") {
            "x86"
        } else if cfg!(target_arch = "aarch64") {
            "arm64"
        } else {
            "x86_64"
        };
        Self { os, arch, features: HashMap::new() }
    }

    pub fn with_feature(mut self, name: &str, on: bool) -> Self {
        self.features.insert(name.to_string(), on);
        self
    }

    /// `${arch}` substitution used in legacy native classifiers.
    pub fn bits(&self) -> &'static str {
        if self.arch == "x86" {
            "32"
        } else {
            "64"
        }
    }
}

fn os_matches(os: &OsRule, env: &Env) -> bool {
    if let Some(name) = &os.name {
        if name != env.os {
            return false;
        }
    }
    if let Some(arch) = &os.arch {
        // Mojang only ever uses "x86" here, meaning 32-bit.
        let matches = match arch.as_str() {
            "x86" => env.arch == "x86",
            "x86_64" | "amd64" => env.arch == "x86_64",
            "arm64" | "aarch64" => env.arch == "arm64",
            _ => false,
        };
        if !matches {
            return false;
        }
    }
    // `version` is a regex against the OS version (e.g. "^10\\."). We can't
    // evaluate it reliably, and every use is a harmless JVM hint, so treat
    // it as matching.
    true
}

pub fn rule_matches(rule: &Rule, env: &Env) -> bool {
    if let Some(os) = &rule.os {
        if !os_matches(os, env) {
            return false;
        }
    }
    if let Some(features) = &rule.features {
        for (name, want) in features {
            if env.features.get(name).copied().unwrap_or(false) != *want {
                return false;
            }
        }
    }
    true
}

/// No rules → allowed. Otherwise start disallowed and let the last matching
/// rule decide.
pub fn allowed(rules: &[Rule], env: &Env) -> bool {
    if rules.is_empty() {
        return true;
    }
    let mut allow = false;
    for rule in rules {
        if rule_matches(rule, env) {
            allow = rule.action == "allow";
        }
    }
    allow
}

#[cfg(test)]
mod tests {
    use super::*;

    fn env(os: &'static str) -> Env {
        Env { os, arch: "x86_64", features: HashMap::new() }
    }

    #[test]
    fn evaluates_rules() {
        let rules: Vec<Rule> = serde_json::from_str(r#"[{"action":"allow"},{"action":"disallow","os":{"name":"osx"}}]"#).unwrap();
        assert!(allowed(&rules, &env("windows")));
        assert!(!allowed(&rules, &env("osx")));

        let only_win: Vec<Rule> = serde_json::from_str(r#"[{"action":"allow","os":{"name":"windows"}}]"#).unwrap();
        assert!(allowed(&only_win, &env("windows")));
        assert!(!allowed(&only_win, &env("linux")));

        let x86: Vec<Rule> = serde_json::from_str(r#"[{"action":"allow","os":{"arch":"x86"}}]"#).unwrap();
        assert!(!allowed(&x86, &env("windows")));

        let feat: Vec<Rule> = serde_json::from_str(r#"[{"action":"allow","features":{"has_custom_resolution":true}}]"#).unwrap();
        assert!(!allowed(&feat, &env("windows")));
        assert!(allowed(&feat, &env("windows").with_feature("has_custom_resolution", true)));
    }
}

//! Instance-owned presentation and capability contract. Platform identity is separate.
use crate::Branding;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct Experience {
    pub kind: String,
    pub branding: Option<Branding>,
    pub features: Vec<String>,
    pub navigation: Vec<ExperiencePage>,
    pub widgets: Vec<ExperienceWidget>,
    /// Module-owned configuration, including custom experience integrations.
    pub modules: BTreeMap<String, serde_json::Value>,
}

impl Default for Experience {
    fn default() -> Self {
        Self { kind: "generic".into(), branding: None, features: vec![], navigation: vec![], widgets: vec![], modules: BTreeMap::new() }
    }
}

impl Experience {
    pub fn enabled(&self, feature: &str) -> bool {
        self.features.iter().any(|f| f == feature)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ExperiencePage {
    /// Built-in page ID, or a registered experience component ID.
    pub id: String,
    pub label: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct ExperienceWidget {
    pub id: String,
    /// Registered component: text, links, or a locally implemented module widget.
    pub component: String,
    pub title: String,
    pub body: String,
    pub config: serde_json::Value,
}
impl Default for ExperienceWidget {
    fn default() -> Self {
        Self { id: String::new(), component: "text".into(), title: String::new(), body: String::new(), config: serde_json::json!({}) }
    }
}

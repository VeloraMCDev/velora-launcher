/// A Maven coordinate: `group:artifact:version[:classifier][@ext]`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Coord {
    pub group: String,
    pub artifact: String,
    pub version: String,
    pub classifier: Option<String>,
    pub ext: String,
}

impl Coord {
    pub fn parse(s: &str) -> Option<Self> {
        let s = s.trim().trim_start_matches('[').trim_end_matches(']');
        let (main, ext) = match s.split_once('@') {
            Some((m, e)) => (m, e.to_string()),
            None => (s, "jar".to_string()),
        };
        let parts: Vec<&str> = main.split(':').collect();
        if parts.len() < 3 {
            return None;
        }
        Some(Self {
            group: parts[0].into(),
            artifact: parts[1].into(),
            version: parts[2].into(),
            classifier: parts.get(3).map(|c| c.to_string()),
            ext,
        })
    }

    /// Relative repository path (`/`-separated).
    pub fn path(&self) -> String {
        let classifier = self.classifier.as_ref().map(|c| format!("-{c}")).unwrap_or_default();
        format!(
            "{}/{}/{}/{}-{}{}.{}",
            self.group.replace('.', "/"),
            self.artifact,
            self.version,
            self.artifact,
            self.version,
            classifier,
            self.ext
        )
    }

    /// Identity used to de-duplicate libraries (version-less).
    pub fn key(&self) -> String {
        match &self.classifier {
            Some(c) => format!("{}:{}:{}", self.group, self.artifact, c),
            None => format!("{}:{}", self.group, self.artifact),
        }
    }

    pub fn with_classifier(&self, classifier: &str) -> Self {
        Self { classifier: Some(classifier.to_string()), ..self.clone() }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_coords() {
        let c = Coord::parse("net.fabricmc:fabric-loader:0.16.9").unwrap();
        assert_eq!(c.path(), "net/fabricmc/fabric-loader/0.16.9/fabric-loader-0.16.9.jar");
        let c = Coord::parse("de.oceanlabs.mcp:mcp_config:1.20.1-20230612.114412@zip").unwrap();
        assert_eq!(c.path(), "de/oceanlabs/mcp/mcp_config/1.20.1-20230612.114412/mcp_config-1.20.1-20230612.114412.zip");
        let c = Coord::parse("[net.minecraft:client:1.20.1-20230612.114412:slim]").unwrap();
        assert_eq!(c.path(), "net/minecraft/client/1.20.1-20230612.114412/client-1.20.1-20230612.114412-slim.jar");
        assert_eq!(c.key(), "net.minecraft:client:slim");
        assert!(Coord::parse("nope").is_none());
    }
}

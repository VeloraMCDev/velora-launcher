//! Generic persisted template shape and validation; catalogs and storage belong to callers.
use crate::Error;
use serde::{Deserialize, Serialize};

#[derive(Clone, Serialize, Deserialize, Default, PartialEq, Debug)]
#[serde(default)]
pub struct EmailTemplate {
    pub id: String,
    pub name: String,
    pub subject: String,
    pub body: String,
}

pub fn validate_size(name: &str, subject: &str, body: &str) -> Result<(), Error> {
    if name.chars().count() > 80 || subject.chars().count() > 200 || body.chars().count() > 20_000 {
        return Err(Error::request("a template is too long"));
    }
    Ok(())
}

/// Preserve legacy bulk replacement rules, including its distinct ID policy.
pub fn validate_batch(templates: Vec<EmailTemplate>) -> Result<Vec<EmailTemplate>, Error> {
    if templates.len() > 50 {
        return Err(Error::request("at most 50 email templates"));
    }
    let mut seen = std::collections::HashSet::new();
    let mut out = Vec::new();
    for mut template in templates {
        template.id = template.id.trim().to_string();
        if template.id.is_empty()
            || !template.id.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
            || !seen.insert(template.id.clone())
        {
            return Err(Error::request("each template needs its own short id (letters, numbers, - and _)"));
        }
        validate_size(&template.name, &template.subject, &template.body)?;
        out.push(template);
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn template(id: &str) -> EmailTemplate {
        EmailTemplate { id: id.into(), name: "Synthetic".into(), subject: "Hello {unknown}".into(), body: "Synthetic body".into() }
    }
    #[test]
    fn defaults_and_stored_unknown_placeholders_round_trip_without_catalogs() {
        assert_eq!(serde_json::from_str::<EmailTemplate>("{}").unwrap(), EmailTemplate::default());
        let original = template("synthetic");
        assert_eq!(
            serde_json::to_value(&original).unwrap(),
            serde_json::json!({"id":"synthetic","name":"Synthetic","subject":"Hello {unknown}","body":"Synthetic body"})
        );
        assert_eq!(serde_json::from_value::<EmailTemplate>(serde_json::to_value(&original).unwrap()).unwrap(), original);
    }
    #[test]
    fn batch_keeps_trim_order_case_and_distinct_id_policy() {
        let out = validate_batch(vec![template(" first "), template("FIRST"), template(&"a".repeat(51))]).unwrap();
        assert_eq!(out.iter().map(|x| x.id.as_str()).collect::<Vec<_>>(), vec!["first", "FIRST", &"a".repeat(51)]);
        assert_eq!(out[0].subject, "Hello {unknown}");
        for id in ["", " ", "bad id", "世界", "a/b"] {
            assert!(validate_batch(vec![template(id)]).is_err());
        }
        assert_eq!(
            validate_batch(vec![template("x"), template(" x ")]).unwrap_err().message,
            "each template needs its own short id (letters, numbers, - and _)"
        );
        assert!(validate_batch(Vec::new()).unwrap().is_empty());
        assert!(validate_batch((0..50).map(|i| template(&i.to_string())).collect()).is_ok());
        assert_eq!(validate_batch(vec![template(""); 51]).unwrap_err().message, "at most 50 email templates");
    }
    #[test]
    fn unicode_size_limits_and_error_precedence_are_preserved() {
        assert!(validate_size(&"🎮".repeat(80), &"界".repeat(200), &"é".repeat(20_000)).is_ok());
        for (name, subject, body) in [
            ("🎮".repeat(81), String::new(), String::new()),
            (String::new(), "界".repeat(201), String::new()),
            (String::new(), String::new(), "é".repeat(20_001)),
        ] {
            assert_eq!(validate_size(&name, &subject, &body).unwrap_err().message, "a template is too long");
        }
        let mut invalid = template("bad id");
        invalid.body = "x".repeat(20_001);
        assert_eq!(validate_batch(vec![invalid]).unwrap_err().message, "each template needs its own short id (letters, numbers, - and _)");
        let mut invalid = template("x");
        invalid.body = "x".repeat(20_001);
        assert_eq!(validate_batch(vec![invalid, template("bad id")]).unwrap_err().message, "a template is too long");
    }
}

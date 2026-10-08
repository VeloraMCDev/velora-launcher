use serde::Deserialize;
use velora_panel_communications::render::{escape, html};

#[derive(Deserialize)]
struct Input {
    body: String,
    brand: String,
    accent: String,
    footer: String,
}

#[test]
fn mail_layout_matches_frozen_source_bytes_for_synthetic_cases() {
    let inputs: Vec<Input> = serde_json::from_str(include_str!("fixtures/render-inputs.json")).unwrap();
    let expected = [
        include_str!("fixtures/render-0.html"),
        include_str!("fixtures/render-1.html"),
        include_str!("fixtures/render-2.html"),
        include_str!("fixtures/render-3.html"),
    ];
    assert_eq!(inputs.len(), expected.len());
    for (input, expected) in inputs.iter().zip(expected) {
        assert_eq!(html(&input.body, &input.brand, &input.accent, &input.footer), expected);
    }
}

#[test]
fn renderer_preserves_escaping_link_policy_and_trusted_footer_boundary() {
    assert_eq!(escape("&<>\"'"), "&amp;&lt;&gt;&quot;'");
    let result = html(
        "<script>evil</script> [bad](javascript:alert(1)) [button: Go](https://example.invalid)",
        "<Brand>",
        "invalid",
        "<strong>Trusted footer</strong>",
    );
    assert!(!result.contains("<script>"));
    assert!(!result.contains("javascript:"));
    assert!(result.contains("&lt;Brand&gt;"));
    assert!(result.contains("background:#8b6cff;color:#ffffff"));
    assert!(result.contains("<strong>Trusted footer</strong>"));
    // Legacy formatting supports bold; single-star emphasis remains literal.
    assert!(html("*literal* **bold**", "", "", "").contains("*literal* <strong>bold</strong>"));
}

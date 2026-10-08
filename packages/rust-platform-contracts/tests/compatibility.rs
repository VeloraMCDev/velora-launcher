use serde_json::{json, Value};
use velora_platform_contracts::*;

#[test]
fn legacy_serializer_fixture_preserves_wire_shape_and_operator_configuration() {
    let fixture: Value = serde_json::from_str(include_str!("fixtures/legacy-manifest.json")).unwrap();
    let manifest: LauncherManifest = serde_json::from_value(fixture.clone()).unwrap();
    assert_eq!(manifest.api_version, API_VERSION);
    assert_eq!(manifest.branding.name, "Example Community");
    assert_eq!(manifest.instances[0].clean_epoch, 2);
    assert!(manifest.instances[0].experience.enabled("example-feature"));
    assert_eq!(serde_json::to_value(manifest).unwrap(), fixture);
}

#[test]
fn absent_configuration_does_not_enable_private_gameplay() {
    let instance: InstanceSummary = serde_json::from_value(json!({"id": "new-host"})).unwrap();
    assert_eq!(instance.experience.kind, "generic");
    assert!(instance.experience.features.is_empty());
    assert!(instance.experience.modules.is_empty());
    assert_eq!(Branding::default().name, "Velora");
    assert_eq!(instance.memory.max_mb, 4096);
}

#[test]
fn existing_explicit_smp_kind_and_custom_configuration_are_retained() {
    let experience: Experience = serde_json::from_value(json!({
        "kind": "smp", "features": ["operator-feature"],
        "modules": {"operator-module": {"nested": [true, 42, "custom"]}}
    }))
    .unwrap();
    assert_eq!(experience.kind, "smp");
    assert!(experience.enabled("operator-feature"));
    assert_eq!(experience.modules["operator-module"]["nested"][1], 42);
    assert_eq!(experience.features.len(), 1);
}

#[test]
fn minecraft_identity_matches_java_reference_values() {
    assert_eq!(offline_uuid("Notch"), "b50ad385-829d-3141-a216-7e7d7539ba7f");
    assert_eq!(offline_uuid("jeb_"), "a762f560-4fce-3236-812a-b80efff0b62b");
    assert!(valid_username("Steve_01"));
    for invalid in ["ab", "has space", "waytoolongusername1", "Stéve"] {
        assert!(!valid_username(invalid));
    }
}

#[test]
fn legacy_loader_aliases_and_server_defaults_remain_supported() {
    for (alias, expected) in [("fabric-loader", Loader::Fabric), ("quilt-loader", Loader::Quilt), ("neo-forge", Loader::NeoForge)] {
        assert_eq!(Loader::parse(alias), Some(expected));
    }
    let server: ServerEntry = serde_json::from_value(json!({"address": "example.test"})).unwrap();
    assert_eq!(server.port, 25565);
    assert!(server.inject);
    assert!(!server.auto_join);
}

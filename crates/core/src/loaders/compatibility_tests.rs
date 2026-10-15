use crate::version::{merge, Arg, VersionJson};

fn parent() -> VersionJson {
    serde_json::from_value(serde_json::json!({
        "id": "synthetic-vanilla",
        "mainClass": "net.minecraft.client.main.Main",
        "arguments": {"game": ["--username", "${auth_player_name}"], "jvm": ["-Dsynthetic.parent=true"]},
        "javaVersion": {"component": "synthetic-runtime", "majorVersion": 21},
        "assetIndex": {"id": "synthetic-assets", "url": "https://synthetic.invalid/assets.json"},
        "downloads": {"client": {"url": "https://synthetic.invalid/client.jar", "size": 8}},
        "libraries": [{"name": "synthetic:shared:1"}, {"name": "synthetic:parent-only:1"}]
    }))
    .unwrap()
}

#[test]
fn loader_family_profiles_preserve_protocol_identity_and_parent_runtime_metadata() {
    for (identifier, main_class) in [
        ("synthetic-fabric", "net.fabricmc.loader.impl.launch.knot.KnotClient"),
        ("synthetic-quilt", "org.quiltmc.loader.impl.launch.knot.KnotClient"),
        ("synthetic-forge", "cpw.mods.bootstraplauncher.BootstrapLauncher"),
        ("synthetic-neoforge", "cpw.mods.bootstraplauncher.BootstrapLauncher"),
    ] {
        let child: VersionJson = serde_json::from_value(serde_json::json!({
            "id": identifier,
            "inheritsFrom": "synthetic-vanilla",
            "mainClass": main_class,
            "arguments": {"game": ["--synthetic-loader"], "jvm": ["-Dsynthetic.loader=true"]},
            "libraries": [{"name": "synthetic:shared:2"}],
            "syntheticFutureExtension": {"ignoredByCurrentParser": true}
        }))
        .unwrap();
        let merged = merge(parent(), child);
        assert_eq!(merged.id, identifier);
        assert_eq!(merged.main_class.as_deref(), Some(main_class));
        assert_eq!(merged.java_version.unwrap().major_version, 21);
        assert_eq!(merged.asset_index.unwrap().id, "synthetic-assets");
        assert_eq!(merged.downloads.unwrap().client.unwrap().url, "https://synthetic.invalid/client.jar");
        assert_eq!(
            merged.libraries.iter().map(|library| library.name.as_str()).collect::<Vec<_>>(),
            vec!["synthetic:shared:2", "synthetic:parent-only:1"]
        );
        let arguments = merged.arguments.unwrap();
        assert_eq!(
            arguments.game,
            vec![Arg::Plain("--username".into()), Arg::Plain("${auth_player_name}".into()), Arg::Plain("--synthetic-loader".into())]
        );
        assert_eq!(arguments.jvm.len(), 2);
    }
}

#[test]
fn native_classifiers_remain_distinct_from_regular_libraries() {
    let child: VersionJson = serde_json::from_value(serde_json::json!({
        "id": "synthetic-native-loader",
        "libraries": [
            {"name": "synthetic:lwjgl:2", "natives": {"windows": "natives-windows", "linux": "natives-linux", "osx": "natives-osx"}},
            {"name": "synthetic:lwjgl:2"}
        ]
    }))
    .unwrap();
    let merged = merge(parent(), child);
    assert_eq!(merged.libraries.iter().filter(|library| library.name == "synthetic:lwjgl:2").count(), 2);
    let classifiers = merged.libraries.iter().find_map(|library| library.natives.as_ref()).unwrap();
    assert_eq!(classifiers["osx"], "natives-osx");
    assert_eq!(classifiers["windows"], "natives-windows");
    assert_eq!(classifiers["linux"], "natives-linux");
}

#[test]
fn legacy_game_arguments_and_modern_quick_play_keep_their_current_merge_semantics() {
    let child: VersionJson = serde_json::from_value(serde_json::json!({
        "id": "synthetic-legacy-forge",
        "minecraftArguments": "--username ${auth_player_name} --synthetic-legacy"
    }))
    .unwrap();
    let merged = merge(parent(), child);
    assert_eq!(merged.minecraft_arguments.as_deref(), Some("--username ${auth_player_name} --synthetic-legacy"));
    let modern: VersionJson = serde_json::from_value(serde_json::json!({
        "id": "synthetic-modern",
        "arguments": {"game": [{"rules": [{"action": "allow", "features": {"is_quick_play_multiplayer": true}}], "value": ["--quickPlayMultiplayer", "${quickPlayMultiplayer}"]}]}
    })).unwrap();
    assert!(modern.supports_quick_play());
    assert!(!merged.supports_quick_play());
}

#[test]
fn malformed_required_loader_profile_fields_are_rejected() {
    for fixture in
        [serde_json::json!({}), serde_json::json!({"id": 7}), serde_json::json!({"id": "synthetic", "libraries": [{"name": null}]})]
    {
        assert!(serde_json::from_value::<VersionJson>(fixture).is_err());
    }
}

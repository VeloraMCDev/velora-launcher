use scopenet_panel::config::Config;

#[test]
fn velora_config_takes_precedence_and_legacy_configuration_remains_usable() {
    // This integration-test binary contains one test, so environment changes
    // cannot race another Config::from_env call in the suite.
    let variables = ["VELORA_BIND", "SCOPENET_BIND", "VELORA_DATA_DIR", "SCOPENET_DATA_DIR"];
    let previous: Vec<_> = variables.iter().map(|key| (*key, std::env::var_os(key))).collect();
    for key in variables { std::env::remove_var(key); }
    std::env::set_var("SCOPENET_BIND", "127.0.0.1:9001");
    std::env::set_var("SCOPENET_DATA_DIR", "legacy-operator-data");
    assert_eq!(Config::from_env().bind, "127.0.0.1:9001");
    assert_eq!(Config::from_env().data_dir, std::path::PathBuf::from("legacy-operator-data"));
    std::env::set_var("VELORA_BIND", " 127.0.0.1:9002 ");
    assert_eq!(Config::from_env().bind, "127.0.0.1:9002");
    std::env::set_var("VELORA_BIND", "   ");
    assert_eq!(Config::from_env().bind, "127.0.0.1:9001");
    for (key, value) in previous {
        match value { Some(value) => std::env::set_var(key, value), None => std::env::remove_var(key) }
    }
}

use std::process::Command;

#[test]
fn command_initializes_paths_with_spaces_and_refuses_overwrite_and_invalid_arguments() {
    let folder = tempfile::tempdir().unwrap();
    let database = folder.path().join("synthetic authority.sqlite");
    let executable = env!("CARGO_BIN_EXE_velora-auth-tools");
    let output = Command::new(executable).arg("init").arg(&database).output().unwrap();
    assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
    let report: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(report["operation"], "init");
    assert_eq!(report["schema_version"], velora_auth_core::schema::VERSION);
    let before = std::fs::read(&database).unwrap();
    assert!(!Command::new(executable).arg("init").arg(&database).output().unwrap().status.success());
    assert_eq!(std::fs::read(&database).unwrap(), before);
    assert!(!Command::new(executable).arg("import").arg(&database).output().unwrap().status.success());
}

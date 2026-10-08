fn main() {
    println!("cargo:rerun-if-env-changed=GITHUB_SHA");
    let sha = std::env::var("GITHUB_SHA").unwrap_or_else(|_| "UNBUILT".into());
    assert!(
        sha == "UNBUILT" || (sha.len() == 40 && sha.bytes().all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))),
        "Invalid source revision"
    );
    println!("cargo:rustc-env=VELORA_AGENT_BUILD_SHA={sha}");
}

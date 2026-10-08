fn main() {
    // Re-run when build-time branding changes (set by CI).
    for var in ["SCOPENET_PANEL_URL", "SCOPENET_REPO", "SCOPENET_LOCK_PANEL"] {
        println!("cargo:rerun-if-env-changed={var}");
    }
    tauri_build::build()
}

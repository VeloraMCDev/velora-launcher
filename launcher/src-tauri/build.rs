fn main() {
    // Re-run when build-time branding changes (set by CI).
    for var in ["VELORA_PANEL_URL", "VELORA_REPO", "VELORA_LOCK_PANEL", "SCOPENET_PANEL_URL", "SCOPENET_REPO", "SCOPENET_LOCK_PANEL"] {
        println!("cargo:rerun-if-env-changed={var}");
    }
    tauri_build::build()
}

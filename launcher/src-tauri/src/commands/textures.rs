use crate::state::AppState;
use tauri::State;

#[tauri::command]
pub fn textures_status(state: State<'_, AppState>) -> velora_launcher_core::textures::Status {
    velora_launcher_core::textures::status(&state.layout)
}

/// Item textures as data URIs, keyed by the name asked for. Names with no texture are left out.
#[tauri::command]
pub fn item_textures(state: State<'_, AppState>, names: Vec<String>) -> std::collections::HashMap<String, String> {
    use base64::Engine;
    names
        .into_iter()
        .take(400)
        .filter_map(|n| {
            let png = velora_launcher_core::textures::find(&state.layout, &n)?;
            Some((n, format!("data:image/png;base64,{}", base64::engine::general_purpose::STANDARD.encode(png))))
        })
        .collect()
}

/// Other Minecraft sprites (HUD hearts, effect icons, panorama…) as data URIs. Names are `kind/path.png`, e.g. `gui/hud/heart/full.png`.
#[tauri::command]
pub fn mc_sprites(state: State<'_, AppState>, names: Vec<String>) -> std::collections::HashMap<String, String> {
    use base64::Engine;
    names
        .into_iter()
        .take(200)
        .filter_map(|n| {
            let (kind, rel) = n.split_once('/')?;
            let png = velora_launcher_core::textures::find_sprite(&state.layout, kind, rel)?;
            Some((n, format!("data:image/png;base64,{}", base64::engine::general_purpose::STANDARD.encode(png))))
        })
        .collect()
}

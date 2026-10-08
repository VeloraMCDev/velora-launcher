mod common;
use base64::{engine::general_purpose::STANDARD, Engine};
use common::*;
use std::io::{Cursor, Read, Write};

const PNG: &[u8] = b"\x89PNG\r\n\x1a\nabcd";

fn zip_of(files: &[(&str, &[u8])]) -> String {
    let mut z = zip::ZipWriter::new(Cursor::new(Vec::new()));
    let o = zip::write::SimpleFileOptions::default();
    for (n, d) in files {
        z.start_file(*n, o).unwrap();
        z.write_all(d).unwrap();
    }
    STANDARD.encode(z.finish().unwrap().into_inner())
}

fn fixture() -> String {
    let yml = r#"ruby_sword:
  itemname: '&cRuby Sword'
  material: DIAMOND_SWORD
  Pack:
    textures: [default/ruby_sword]
    custom_model_data: 1001
bench:
  itemname: 'Garden Bench'
  material: PAPER
  Pack:
    model: default/bench
  Mechanics:
    furniture:
      seat: {height: 0.5}
vault:
  itemname: 'Vault'
  material: PAPER
  Pack:
    model: default/bench
  Mechanics:
    furniture:
      storage: {rows: 4}
"#;
    zip_of(&[
        ("plugins/Oraxen/items/all.yml", yml.as_bytes()),
        ("plugins/Oraxen/pack/textures/default/ruby_sword.png", PNG),
        ("plugins/Oraxen/pack/textures/default/wood.png", PNG),
        ("plugins/Oraxen/pack/textures/default/unused.png", PNG),
        ("plugins/Oraxen/pack/models/default/bench.json", br##"{"textures":{"0":"default/wood"},"elements":[{"from":[0,0,0],"to":[16,8,16],"faces":{"up":{"uv":[0,0,16,16],"texture":"#0"}}}]}"##),
    ])
}

#[tokio::test]
async fn the_wizard_analyses_imports_and_serves_everything_to_the_pack_and_the_game() {
    let t = setup().await;
    let admin = t.login("admin", "supersecret").await;
    let (_, server) = t.call("POST", "/api/admin/servers", Some(&admin), Some(json!({"name":"SMP","instance_id":"smp"}))).await;
    let key = server["token"].as_str().unwrap();
    let data = fixture();

    assert_eq!(t.call("POST", "/api/admin/content/analyze", None, Some(json!({"data": data}))).await.0, StatusCode::UNAUTHORIZED);
    assert_eq!(t.call("POST", "/api/admin/content/analyze", Some(&admin), Some(json!({"data": STANDARD.encode("junk")}))).await.0, StatusCode::BAD_REQUEST);

    let (s, a) = t.call("POST", "/api/admin/content/analyze", Some(&admin), Some(json!({"data": data}))).await;
    assert_eq!(s, StatusCode::OK, "{a}");
    assert_eq!(a["detected"][0]["framework"], "oraxen");
    let entries = a["entries"].as_array().unwrap();
    let find = |id: &str| entries.iter().find(|e| e["id"] == id).unwrap_or_else(|| panic!("{id} in {a}"));
    assert_eq!((find("ruby_sword")["kind"].as_str(), find("bench")["kind"].as_str(), find("vault")["kind"].as_str()), (Some("weapon"), Some("decoration"), Some("chest")));
    assert!(find("bench")["preview"].as_str().unwrap().starts_with("data:image/png"));
    assert_eq!(find("bench")["view"]["mode"], "elements", "3D bundles come with the analysis");
    assert_eq!(find("ruby_sword")["view"]["mode"], "flat");

    // Import two of three, retitling the bench and sending the sword down as a tool.
    let picks = json!([
        {"key": "oraxen:ruby_sword", "kind": "tool", "title": "Ruby Blade"},
        {"key": "oraxen:bench", "title": "Park Bench", "name": "&6Park Bench", "extras": {"seat": true}},
    ]);
    let (s, done) = t.call("POST", "/api/admin/content/import", Some(&admin), Some(json!({"data": data, "entries": picks}))).await;
    assert_eq!(s, StatusCode::OK, "{done}");
    assert_eq!(done["created"], 2);
    assert_eq!(done["files_added"], 3, "sword texture + bench model + its texture; the unused texture stays behind");

    let (_, items) = t.call("GET", "/api/admin/custom-items", Some(&admin), None).await;
    let sword = &items["items"][0];
    assert_eq!((sword["id"].as_str(), sword["title"].as_str(), sword["spec"]["category"].as_str(), sword["spec"]["source"].as_str()), (Some("ruby_sword"), Some("Ruby Blade"), Some("tool"), Some("oraxen")));
    assert_eq!(sword["spec"]["custom_model_data"], 1001);

    let (_, lib) = t.call("GET", "/api/admin/content", Some(&admin), None).await;
    let lib = lib["items"].as_array().unwrap();
    assert_eq!(lib.len(), 2);
    let bench = lib.iter().find(|i| i["id"] == "bench").unwrap();
    assert_eq!((bench["kind"].as_str(), bench["store"].as_str(), bench["title"].as_str()), (Some("decoration"), Some("content"), Some("Park Bench")));
    assert!(bench["preview"].is_string());
    let (_, full) = t.call("GET", "/api/admin/content/bench", Some(&admin), None).await;
    assert_eq!((full["spec"]["seat"].as_bool(), full["spec"]["name"].as_str()), (Some(true), Some("&6Park Bench")));
    assert_eq!(full["spec"]["display"]["model"], "minecraft:default/bench", "{full}");
    let cmd = full["spec"]["display"]["custom_model_data"].as_i64().unwrap();
    assert!(cmd >= 100_000);

    // Importing again changes nothing.
    let (_, again) = t.call("POST", "/api/admin/content/import", Some(&admin), Some(json!({"data": data, "entries": picks}))).await;
    assert_eq!(again["created"], 0);
    assert_eq!(again["entries"][0]["status"], "exists");

    // The resource pack carries the content's model under a base-item override, and the game server receives it.
    t.call("PUT", "/api/admin/resource-pack", Some(&admin), Some(json!({"enabled": true, "required": true, "pack_format": 84}))).await;
    let (_, bytes) = t.fetch("/api/v1/resource-pack.zip").await;
    let mut zip = zip::ZipArchive::new(Cursor::new(bytes)).unwrap();
    let mut paper = String::new();
    zip.by_name("assets/minecraft/models/item/paper.json").unwrap().read_to_string(&mut paper).unwrap();
    assert!(paper.contains(&cmd.to_string()) && paper.contains("minecraft:default/bench"), "{paper}");
    assert!(zip.by_name("assets/minecraft/models/default/bench.json").is_ok());
    let (_, sync) = t.call("POST", "/api/server/v1/sync", Some(key), Some(json!({"online": []}))).await;
    assert_eq!(sync["content"][0]["id"], "bench");
    assert_eq!(sync["content"][0]["kind"], "decoration");

    // Manual editing validates by kind.
    let spec = json!({"display": {"item": "minecraft:paper", "custom_model_data": cmd + 5, "model": "minecraft:default/bench"}, "rows": 9});
    let (s, saved) = t.call("PUT", "/api/admin/content/store", Some(&admin), Some(json!({"kind": "chest", "title": "Store", "spec": spec}))).await;
    assert_eq!(s, StatusCode::OK, "{saved}");
    assert_eq!(saved["spec"]["rows"], 6, "rows are clamped to what a chest GUI holds");
    let dupe = json!({"display": {"item": "minecraft:paper", "custom_model_data": cmd, "model": "minecraft:default/bench"}});
    assert_eq!(t.call("PUT", "/api/admin/content/other", Some(&admin), Some(json!({"kind": "decoration", "title": "Other", "spec": dupe}))).await.0, StatusCode::BAD_REQUEST, "model data is unique per base item");
    assert_eq!(t.call("PUT", "/api/admin/content/crop1", Some(&admin), Some(json!({"kind": "crop", "title": "Crop", "spec": {"stages": []}}))).await.0, StatusCode::BAD_REQUEST);
    assert_eq!(t.call("PUT", "/api/admin/content/ruby_sword", Some(&admin), Some(json!({"kind": "block", "title": "x", "spec": spec}))).await.0, StatusCode::BAD_REQUEST, "ids are unique across items and content");
    assert_eq!(t.call("DELETE", "/api/admin/content/store", Some(&admin), None).await.0, StatusCode::OK);
}

use p3_durability_probe::{pack_project, unpack_project};

const REVIUNG41: &[u8] = include_bytes!("../fixtures/reviung41-original.boardstudio");

#[test]
fn copied_reviung41_project_and_all_assets_round_trip_through_rust_archive_api() {
    let original = unpack_project(REVIUNG41).expect("fixture passes Rust archive validation");
    let project: serde_json::Value =
        serde_json::from_str(&original.project_json).expect("project JSON remains valid");
    assert_eq!(project["name"], "REVIUNG41");
    assert_eq!(project["parts"].as_array().map(Vec::len), Some(85));
    assert_eq!(project["assets"].as_array().map(Vec::len), Some(4));
    assert_eq!(original.assets.len(), 4);

    let repacked = pack_project(&original).expect("project archives through Rust API");
    let restored = unpack_project(&repacked).expect("repacked archive validates");

    assert_eq!(restored.project_json, original.project_json);
    assert_eq!(restored.assets, original.assets);
}

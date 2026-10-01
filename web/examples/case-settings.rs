use std::io::{self, Read};
fn main() {
    let board = std::env::args().nth(1).expect("board ID argument");
    let mut input = String::new();
    io::stdin()
        .read_to_string(&mut input)
        .expect("document input");
    if board == "--update-instance" {
        let value: serde_json::Value = serde_json::from_str(&input).expect("valid instance input");
        let document = serde_json::from_value(value["document"].clone()).expect("valid document");
        let configuration =
            serde_json::from_value(value["configuration"].clone()).expect("valid configuration");
        let updated = boardstudio_web::case_settings::update_instance_settings(
            &document,
            value["instanceId"].as_str().expect("instance ID"),
            configuration,
        )
        .expect("instance update");
        println!(
            "{}",
            serde_json::to_string(&updated).expect("updated document")
        );
        return;
    }
    let document = serde_json::from_str(&input).expect("valid project document");
    let settings = boardstudio_web::case_settings::initial_settings(&document, &board)
        .expect("reference settings");
    println!(
        "{}",
        serde_json::to_string(&settings).expect("settings JSON")
    );
}

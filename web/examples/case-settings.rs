use std::io::{self, Read};
fn main() {
    let board = std::env::args().nth(1).expect("board ID argument");
    let mut input = String::new();
    io::stdin()
        .read_to_string(&mut input)
        .expect("document input");
    let document = serde_json::from_str(&input).expect("valid project document");
    let settings = boardstudio_web::case_settings::initial_settings(&document, &board)
        .expect("reference settings");
    println!(
        "{}",
        serde_json::to_string(&settings).expect("settings JSON")
    );
}

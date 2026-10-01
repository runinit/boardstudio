#[cfg(target_arch = "wasm32")]
fn main() {
    dioxus::launch(p2_lifecycle_probe::web::app);
}

#[cfg(not(target_arch = "wasm32"))]
fn main() {
    println!("P2 lifecycle probe is a browser-only feasibility build.");
}

#[cfg(target_arch = "wasm32")]
fn main() {
    dioxus::launch(app);
}

#[cfg(target_arch = "wasm32")]
fn app() -> dioxus::prelude::Element {
    use dioxus::prelude::*;

    use_effect(|| {
        if let Some(root) = web_sys::window()
            .and_then(|window| window.document())
            .and_then(|document| document.document_element())
        {
            let _ = root.set_attribute("lang", "en");
        }
    });

    p2_lifecycle_probe::web::app()
}

#[cfg(not(target_arch = "wasm32"))]
fn main() {
    println!("P2 lifecycle probe is a browser-only feasibility build.");
}

#[cfg(all(target_arch = "wasm32", feature = "page"))]
mod presentation;
#[cfg(all(target_arch = "wasm32", feature = "page"))]
mod runtime;

fn main() {
    #[cfg(all(target_arch = "wasm32", feature = "page"))]
    dioxus::launch(app);
}

#[cfg(all(target_arch = "wasm32", feature = "page"))]
fn app() -> dioxus::prelude::Element {
    dioxus::prelude::use_effect(|| {
        if let Some(root) = web_sys::window()
            .and_then(|window| window.document())
            .and_then(|document| document.document_element())
        {
            let _ = root.set_attribute("lang", "en");
        }
    });
    presentation::App()
}

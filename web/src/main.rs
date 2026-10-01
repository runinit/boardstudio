#[cfg(all(target_arch = "wasm32", feature = "page"))]
mod presentation;
#[cfg(all(target_arch = "wasm32", feature = "page"))]
mod runtime;

fn main() {
    #[cfg(all(target_arch = "wasm32", feature = "page"))]
    dioxus::launch(presentation::App);
}

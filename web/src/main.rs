#[cfg(target_arch = "wasm32")]
mod presentation;
#[cfg(target_arch = "wasm32")]
mod runtime;
#[cfg(target_arch = "wasm32")]
mod renderer_host;

fn main() {
    #[cfg(target_arch = "wasm32")]
    dioxus::launch(presentation::App);
}

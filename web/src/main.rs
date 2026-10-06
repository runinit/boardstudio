// The page's non-UI layer lives in `boardstudio-web-runtime`; re-export its modules at
// the crate root so presentation code keeps addressing them as `crate::<module>`.
#[cfg(feature = "page")]
#[allow(unused_imports)]
use boardstudio_web_runtime::{
    archive_export, bundled_models, case_generation_lifecycle, case_model_lifecycle, case_preview,
    firmware_position_projection, layout_viewer_source, model_delivery, operation_outcomes,
    parts_preview, pcb_wiring_mode_operation, portable_archive, runtime,
};

#[cfg(all(target_arch = "wasm32", feature = "page"))]
pub(crate) use boardstudio_web_case::cad_presentation;
#[cfg(all(feature = "page", target_arch = "wasm32"))]
pub(crate) use boardstudio_web_layout::matrix_transform_lifecycle;
#[cfg(all(feature = "page", target_arch = "wasm32"))]
pub(crate) use boardstudio_web_layout::mirrored_pair_geometry;
#[cfg(all(feature = "page", target_arch = "wasm32"))]
pub(crate) use boardstudio_web_layout::setup_guide_state;
#[cfg(all(target_arch = "wasm32", feature = "page"))]
mod presentation;

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

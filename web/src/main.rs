// The page's non-UI layer lives in `boardstudio-web-runtime`; re-export its modules at
// the crate root so presentation code keeps addressing them as `crate::<module>`.
#[cfg(feature = "page")]
#[allow(unused_imports)]
use boardstudio_web_runtime::{
    archive_export, bundled_models, case_generation_lifecycle, case_model_lifecycle, case_preview,
    firmware_position_projection, layout_viewer_source, model_delivery, operation_outcomes,
    parts_preview, pcb_wiring_mode_operation, portable_archive, runtime,
};

#[cfg(all(feature = "page", any(test, target_arch = "wasm32")))]
mod setup_guide_state;

#[cfg(all(target_arch = "wasm32", feature = "page"))]
pub(crate) use boardstudio_web_case::cad_presentation;
#[cfg(feature = "page")]
mod outline_settings;
#[cfg(all(feature = "page", target_arch = "wasm32"))]
pub(crate) use boardstudio_web_catalogue::matrix_setup_operation;
#[cfg(all(target_arch = "wasm32", feature = "page"))]
mod presentation;

#[cfg(all(feature = "page", test, not(target_arch = "wasm32")))]
mod presentation {
    pub(crate) use crate::model_delivery;

    #[path = "outline_grid_rounding.rs"]
    mod outline_grid_rounding_tests;

    pub(crate) mod objects {
        #[path = "keycap_resize.rs"]
        mod keycap_resize;
    }
}

#[cfg(all(target_arch = "wasm32", feature = "page"))]
#[path = "matrix_transform_operation.rs"]
mod matrix_transform_operation;

#[cfg(all(feature = "page", any(test, target_arch = "wasm32")))]
mod mirrored_pair_lifecycle;

#[cfg(all(feature = "page", any(test, target_arch = "wasm32")))]
mod matrix_transform_lifecycle;

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

#[cfg(feature = "page")]
mod mirrored_pair_geometry;

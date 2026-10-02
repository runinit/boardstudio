#[cfg(feature = "page")]
mod archive_export;
#[cfg(feature = "page")]
mod bundled_models;
#[cfg(all(feature = "page", any(test, target_arch = "wasm32")))]
mod case_model_lifecycle;
#[cfg(all(feature = "page", any(test, target_arch = "wasm32")))]
mod setup_guide_state;

#[cfg(all(target_arch = "wasm32", feature = "page"))]
mod cad_presentation;
#[cfg(all(feature = "page", any(test, target_arch = "wasm32")))]
mod physical_setup;
#[cfg(feature = "page")]
mod portable_archive;
#[cfg(all(target_arch = "wasm32", feature = "page"))]
mod presentation;
#[cfg(all(target_arch = "wasm32", feature = "page"))]
mod renderer_host_page;
#[cfg(all(target_arch = "wasm32", feature = "page"))]
mod runtime;

#[cfg(all(target_arch = "wasm32", feature = "page"))]
mod firmware_request_adapter;

#[cfg(all(target_arch = "wasm32", feature = "page"))]
mod preview_generator;

#[cfg(all(target_arch = "wasm32", feature = "page"))]
mod case_preview_lifecycle;

#[cfg(all(feature = "page", any(test, target_arch = "wasm32")))]
mod case_preview;

#[cfg(all(feature = "page", any(test, target_arch = "wasm32")))]
mod operation_outcomes;

#[cfg(all(feature = "page", any(test, target_arch = "wasm32")))]
mod case_generation_admission;

#[cfg(all(feature = "page", any(test, target_arch = "wasm32")))]
mod mechanical_feedback;

#[cfg(all(feature = "page", any(test, target_arch = "wasm32")))]
mod firmware_position_projection;

#[cfg(all(feature = "page", any(test, target_arch = "wasm32")))]
mod firmware_position_choices;

#[cfg(all(feature = "page", test, not(target_arch = "wasm32")))]
#[path = "presentation/instance_selection.rs"]
mod instance_selection;

#[cfg(all(feature = "page", any(test, target_arch = "wasm32")))]
#[path = "presentation/footprint_forms.rs"]
mod footprint_forms;

#[cfg(all(feature = "page", test, not(target_arch = "wasm32")))]
mod renderer_host_source_sync;

#[cfg(all(feature = "page", test, not(target_arch = "wasm32")))]
#[path = "presentation/case_display.rs"]
mod case_display;

#[cfg(all(feature = "page", test, not(target_arch = "wasm32")))]
#[path = "presentation/model_delivery.rs"]
mod model_delivery;

#[cfg(all(feature = "page", any(test, target_arch = "wasm32")))]
#[path = "presentation/closure_clearance.rs"]
mod closure_clearance;

#[cfg(all(target_arch = "wasm32", feature = "page"))]
#[path = "matrix_transform_operation.rs"]
mod matrix_transform_operation;

#[cfg(all(feature = "page", any(test, target_arch = "wasm32")))]
mod matrix_setup_operation;

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
